//! Architecture selection for multi-platform base images (PMAT-756, IMG-001).
//! Every test NAMES the architecture; none depends on the chip it runs on
//! except the one that asserts the host mapping itself.

use super::tests::{create_test_layout, CONFIG_HEX, MANIFEST_HEX};
use super::*;

/// Write a multi-platform index whose amd64 and arm64 manifests carry
/// DIFFERENT layer digests, so which one was picked is observable.
fn create_multi_arch_layout(dir: &Path) -> (String, String) {
    create_test_layout(dir);
    let blobs = dir.join("blobs/sha256");
    let mut picks = Vec::new();
    for (arch, layer_hex) in [("amd64", "a1".repeat(32)), ("arm64", "b2".repeat(32))] {
        let manifest = OciManifest::new(
            format!("sha256:{CONFIG_HEX}"),
            vec![OciDescriptor::gzip_layer(format!("sha256:{layer_hex}"), 15)],
        );
        let json = serde_json::to_vec(&manifest).unwrap();
        let hex = format!("{:0>64x}", arch.len() * 7 + usize::from(arch == "arm64"));
        std::fs::write(blobs.join(&hex), json).unwrap();
        picks.push((arch, hex, layer_hex));
    }
    let entry = |arch: &str, hex: &str| {
        format!(
            r#"{{"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":"sha256:{hex}","size":1,"platform":{{"os":"linux","architecture":"{arch}"}}}}"#
        )
    };
    // arm64 listed FIRST so "take the first entry" is wrong for amd64.
    let inner = format!(
        r#"{{"schemaVersion":2,"manifests":[{},{}]}}"#,
        entry("arm64", &picks[1].1),
        entry("amd64", &picks[0].1)
    );
    let idx_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    std::fs::write(blobs.join(idx_hex), inner).unwrap();
    let top = format!(
        r#"{{"schemaVersion":2,"manifests":[{{"mediaType":"application/vnd.oci.image.index.v1+json","digest":"sha256:{idx_hex}","size":1}}]}}"#
    );
    std::fs::write(dir.join("index.json"), top).unwrap();
    (picks[0].2.clone(), picks[1].2.clone())
}

#[test]
fn index_base_resolves_to_the_named_architecture_amd64() {
    let dir = tempfile::tempdir().unwrap();
    let (amd, _) = create_multi_arch_layout(dir.path());
    let r = extract_base_layers_for_arch(dir.path(), "amd64").unwrap();
    assert_eq!(r.layers[0].digest, format!("sha256:{amd}"));
}

#[test]
fn index_base_resolves_to_the_named_architecture_arm64() {
    let dir = tempfile::tempdir().unwrap();
    let (_, arm) = create_multi_arch_layout(dir.path());
    let r = extract_base_layers_for_arch(dir.path(), "arm64").unwrap();
    assert_eq!(r.layers[0].digest, format!("sha256:{arm}"));
}

#[test]
fn index_nested_too_deeply_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    create_test_layout(dir.path());
    let blobs = dir.path().join("blobs/sha256");
    // An index whose only entry is itself.
    let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let me = format!(
        r#"{{"manifests":[{{"digest":"sha256:{hex}","size":1,"platform":{{"os":"linux","architecture":"amd64"}}}}]}}"#
    );
    std::fs::write(blobs.join(hex), me).unwrap();
    let top = format!(
        r#"{{"schemaVersion":2,"manifests":[{{"mediaType":"application/vnd.oci.image.index.v1+json","digest":"sha256:{hex}","size":1}}]}}"#
    );
    std::fs::write(dir.path().join("index.json"), top).unwrap();
    let err = extract_base_layers_for_arch(dir.path(), "amd64").unwrap_err();
    assert!(err.contains("nested too deeply"), "{err}");
}

#[test]
fn extract_base_layers_builds_for_the_host_architecture() {
    let dir = tempfile::tempdir().unwrap();
    let (amd, arm) = create_multi_arch_layout(dir.path());
    let want = match host_oci_arch() {
        "amd64" => amd,
        _ => arm,
    };
    let r = extract_base_layers(dir.path()).unwrap();
    assert_eq!(r.layers[0].digest, format!("sha256:{want}"));
}

#[test]
fn index_chain_of_three_resolves_and_four_is_refused() {
    // idx0 -> idx1 -> ... -> manifest. Three nested indices are allowed,
    // a fourth is not (depth limit 3).
    for (levels, ok) in [(3usize, true), (4, false)] {
        let dir = tempfile::tempdir().unwrap();
        create_test_layout(dir.path());
        let blobs = dir.path().join("blobs/sha256");
        let hexes: Vec<String> = (0..levels)
            .map(|i| format!("{:0>64x}", 0xabc0 + i))
            .collect();
        for (i, h) in hexes.iter().enumerate() {
            let next = hexes.get(i + 1).map_or(MANIFEST_HEX, String::as_str);
            let body = format!(
                r#"{{"manifests":[{{"digest":"sha256:{next}","size":1,"platform":{{"os":"linux","architecture":"amd64"}}}}]}}"#
            );
            std::fs::write(blobs.join(h), body).unwrap();
        }
        let top = format!(
            r#"{{"schemaVersion":2,"manifests":[{{"mediaType":"application/vnd.oci.image.index.v1+json","digest":"sha256:{}","size":1}}]}}"#,
            hexes[0]
        );
        std::fs::write(dir.path().join("index.json"), top).unwrap();
        let r = extract_base_layers_for_arch(dir.path(), "amd64");
        assert_eq!(r.is_ok(), ok, "levels={levels}: {:?}", r.err());
    }
}

#[test]
fn oci_arch_maps_rust_arch_names() {
    assert_eq!(oci_arch_for("x86_64"), "amd64");
    assert_eq!(oci_arch_for("aarch64"), "arm64");
    assert_eq!(oci_arch_for("riscv64"), "riscv64");
}

#[test]
fn the_host_architecture_maps_to_an_oci_name() {
    // The build target is the host: x86_64 builds amd64, aarch64 builds arm64.
    let want = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    };
    assert_eq!(host_oci_arch(), want);
}

/// Write a leaf manifest blob and return (hex, layer_hex) for it.
fn put_manifest(dir: &Path, seed: usize) -> (String, String) {
    let layer_hex = format!("{:0>64x}", 0xc0de00 + seed);
    let manifest = OciManifest::new(
        format!("sha256:{CONFIG_HEX}"),
        vec![OciDescriptor::gzip_layer(format!("sha256:{layer_hex}"), 15)],
    );
    let hex = format!("{:0>64x}", 0xfeed00 + seed);
    std::fs::write(
        dir.join("blobs/sha256").join(&hex),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    (hex, layer_hex)
}

fn entry(os: &str, arch: &str, hex: &str) -> String {
    format!(
        r#"{{"digest":"sha256:{hex}","size":1,"platform":{{"os":"{os}","architecture":"{arch}"}}}}"#
    )
}

/// Write an index blob holding `entries` and return its hex.
fn put_index(dir: &Path, seed: usize, entries: &[String]) -> String {
    let hex = format!("{:0>64x}", 0xabcd00 + seed);
    let body = format!(
        r#"{{"schemaVersion":2,"manifests":[{}]}}"#,
        entries.join(",")
    );
    std::fs::write(dir.join("blobs/sha256").join(&hex), body).unwrap();
    hex
}

fn point_top_index_at(dir: &Path, hex: &str) {
    let top = format!(
        r#"{{"schemaVersion":2,"manifests":[{{"mediaType":"application/vnd.oci.image.index.v1+json","digest":"sha256:{hex}","size":1}}]}}"#
    );
    std::fs::write(dir.join("index.json"), top).unwrap();
}

#[test]
fn non_linux_entry_is_never_picked_even_when_listed_first() {
    let dir = tempfile::tempdir().unwrap();
    create_test_layout(dir.path());
    let (win, _) = put_manifest(dir.path(), 1);
    let (lin, lin_layer) = put_manifest(dir.path(), 2);
    let idx = put_index(
        dir.path(),
        1,
        &[
            entry("windows", "amd64", &win),
            entry("linux", "amd64", &lin),
        ],
    );
    point_top_index_at(dir.path(), &idx);
    let r = extract_base_layers_for_arch(dir.path(), "amd64").unwrap();
    assert_eq!(r.layers[0].digest, format!("sha256:{lin_layer}"));
}

#[test]
fn index_with_only_non_linux_entries_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    create_test_layout(dir.path());
    let (win, _) = put_manifest(dir.path(), 1);
    let (unk, _) = put_manifest(dir.path(), 2);
    let idx = put_index(
        dir.path(),
        2,
        &[
            entry("windows", "amd64", &win),
            entry("unknown", "unknown", &unk),
        ],
    );
    point_top_index_at(dir.path(), &idx);
    let err = extract_base_layers_for_arch(dir.path(), "amd64").unwrap_err();
    assert!(err.contains("no linux/amd64"), "{err}");
}

#[test]
fn index_without_the_architecture_is_refused_not_silently_other_arch() {
    let dir = tempfile::tempdir().unwrap();
    create_test_layout(dir.path());
    let (arm, _) = put_manifest(dir.path(), 1);
    let idx = put_index(dir.path(), 3, &[entry("linux", "arm64", &arm)]);
    point_top_index_at(dir.path(), &idx);
    let err = extract_base_layers_for_arch(dir.path(), "amd64").unwrap_err();
    assert!(err.contains("no linux/amd64"), "{err}");
}

#[test]
fn nested_index_resolves_the_named_architecture_below_depth_zero() {
    for (arch, want_arm) in [("arm64", true), ("amd64", false)] {
        let dir = tempfile::tempdir().unwrap();
        create_test_layout(dir.path());
        let (amd, amd_layer) = put_manifest(dir.path(), 1);
        let (arm, arm_layer) = put_manifest(dir.path(), 2);
        // inner index lists amd64 FIRST; outer entry is linux/<arch> too.
        let inner = put_index(
            dir.path(),
            4,
            &[entry("linux", "amd64", &amd), entry("linux", "arm64", &arm)],
        );
        let outer = put_index(dir.path(), 5, &[entry("linux", arch, &inner)]);
        point_top_index_at(dir.path(), &outer);
        let r = extract_base_layers_for_arch(dir.path(), arch).unwrap();
        let want = if want_arm { arm_layer } else { amd_layer };
        assert_eq!(r.layers[0].digest, format!("sha256:{want}"), "{arch}");
    }
}

#[test]
fn host_extract_picks_the_cfg_architecture_from_a_decoy_first_index() {
    let dir = tempfile::tempdir().unwrap();
    create_test_layout(dir.path());
    let (amd, amd_layer) = put_manifest(dir.path(), 1);
    let (arm, arm_layer) = put_manifest(dir.path(), 2);
    let idx = put_index(
        dir.path(),
        6,
        &[entry("linux", "arm64", &arm), entry("linux", "amd64", &amd)],
    );
    point_top_index_at(dir.path(), &idx);
    let want = if cfg!(target_arch = "aarch64") {
        arm_layer
    } else {
        amd_layer
    };
    if cfg!(any(target_arch = "aarch64", target_arch = "x86_64")) {
        let r = extract_base_layers(dir.path()).unwrap();
        assert_eq!(r.layers[0].digest, format!("sha256:{want}"));
    }
}
