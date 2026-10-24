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
fn index_base_without_the_architecture_falls_back_to_its_first_entry() {
    let dir = tempfile::tempdir().unwrap();
    let (_, arm) = create_multi_arch_layout(dir.path());
    let r = extract_base_layers_for_arch(dir.path(), "riscv64").unwrap();
    assert_eq!(r.layers[0].digest, format!("sha256:{arm}"));
}

#[test]
fn index_nested_too_deeply_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    create_test_layout(dir.path());
    let blobs = dir.path().join("blobs/sha256");
    // An index whose only entry is itself.
    let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let me = format!(r#"{{"manifests":[{{"digest":"sha256:{hex}","size":1}}]}}"#);
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
        _ => arm, // arm64, or no match: the index's first entry (arm64 here)
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
            let body = format!(r#"{{"manifests":[{{"digest":"sha256:{next}","size":1}}]}}"#);
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
