//! PMAT-989 (IMG-001 3/N, phase 1): `forjar build` layers the base image its
//! config declares, and refuses to build without it.
//!
//! Every test drives the real `cmd_build_in` path against a private state
//! directory, so none of them touches the shared cwd-relative `state/images`.

use super::build_image::cmd_build_in;
use crate::core::types::{OciDescriptor, OciImageConfig, OciIndex, OciManifest};
use std::path::{Path, PathBuf};

pub(super) const BASE_REF: &str = "ubuntu:22.04";
/// `build_image` maps `:` and `/` in a base reference to `_` for the dir name.
pub(super) const BASE_DIR_NAME: &str = "ubuntu_22.04";

/// A valid 64-hex string made of one repeated byte.
pub(super) fn hex(seed: u8) -> String {
    format!("{seed:02x}").repeat(32)
}

/// Write a small, structurally valid single-layer base OCI layout. `seed`
/// picks every digest; `layer` is the layer blob content.
pub(super) fn write_base(dir: &Path, seed: u8, layer: &[u8]) {
    let blobs = dir.join("blobs/sha256");
    std::fs::create_dir_all(&blobs).unwrap();
    let (cfg_hex, layer_hex, man_hex) = (hex(seed), hex(seed + 1), hex(seed + 2));
    let cfg = OciImageConfig::linux_amd64(vec![format!("sha256:{}", hex(seed + 3))]);
    std::fs::write(blobs.join(&cfg_hex), serde_json::to_vec(&cfg).unwrap()).unwrap();
    std::fs::write(blobs.join(&layer_hex), layer).unwrap();
    let manifest = OciManifest::new(
        format!("sha256:{cfg_hex}"),
        vec![OciDescriptor::gzip_layer(
            format!("sha256:{layer_hex}"),
            layer.len() as u64,
        )],
    );
    std::fs::write(blobs.join(&man_hex), serde_json::to_vec(&manifest).unwrap()).unwrap();
    let index = OciIndex::single(OciDescriptor {
        media_type: "application/vnd.oci.image.manifest.v1+json".into(),
        digest: format!("sha256:{man_hex}"),
        size: 1,
        annotations: Default::default(),
    });
    std::fs::write(dir.join("index.json"), serde_json::to_vec(&index).unwrap()).unwrap();
}

/// The digest `write_base(.., seed, ..)` gives its layer.
pub(super) fn base_layer_digest(seed: u8) -> String {
    format!("sha256:{}", hex(seed + 1))
}

fn write_config(dir: &Path, with_base: bool) -> PathBuf {
    let base = if with_base {
        format!("    image: \"{BASE_REF}\"\n")
    } else {
        String::new()
    };
    let path = dir.join("forjar.yaml");
    std::fs::write(
        &path,
        format!(
            "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\n\
             resources:\n  app:\n    type: image\n    machine: m\n    name: myapp\n    \
             version: \"1.0\"\n{base}    path: /app/bin\n"
        ),
    )
    .unwrap();
    path
}

fn build(state: &Path, config: &Path) -> Result<(), String> {
    cmd_build_in(state, config, "app", false, false, false, false, false)
}

/// The manifest the built layout's `index.json` points at.
pub(super) fn read_manifest(out: &Path) -> OciManifest {
    let index: OciIndex =
        serde_json::from_slice(&std::fs::read(out.join("index.json")).unwrap()).unwrap();
    let hex = index.manifests[0].digest.strip_prefix("sha256:").unwrap();
    serde_json::from_slice(&std::fs::read(out.join("blobs/sha256").join(hex)).unwrap()).unwrap()
}

pub(super) fn manifest_digest(out: &Path) -> String {
    let index: OciIndex =
        serde_json::from_slice(&std::fs::read(out.join("index.json")).unwrap()).unwrap();
    index.manifests[0].digest.clone()
}

#[test]
fn t1_base_layers_come_first_and_their_blobs_are_in_the_output() {
    let work = tempfile::tempdir().unwrap();
    let state = work.path().join("state");
    write_base(&state.join(BASE_DIR_NAME), 0x10, b"base-layer-bytes");
    let cfg = write_config(work.path(), true);

    build(&state, &cfg).expect("a build whose declared base is on disk must succeed");

    let out = state.join("app");
    let manifest = read_manifest(&out);
    assert_eq!(manifest.layers.len(), 2, "base layer + one built layer");
    assert_eq!(
        manifest.layers[0].digest,
        base_layer_digest(0x10),
        "the base layer must be FIRST in the manifest"
    );
    assert_ne!(manifest.layers[1].digest, base_layer_digest(0x10));
    let blob = out
        .join("blobs/sha256")
        .join(base_layer_digest(0x10).strip_prefix("sha256:").unwrap());
    assert_eq!(std::fs::read(&blob).unwrap(), b"base-layer-bytes");
}

#[test]
fn t2_declared_base_without_a_layout_fails_closed_and_writes_nothing() {
    let work = tempfile::tempdir().unwrap();
    let state = work.path().join("state");
    let cfg = write_config(work.path(), true);

    let err = build(&state, &cfg)
        .expect_err("a declared base that is absent must be an error, not a scratch image");

    let dir = state.join(BASE_DIR_NAME);
    assert!(err.contains(BASE_REF), "names the base reference: {err}");
    assert!(err.contains(&dir.display().to_string()), "names the dir: {err}");
    assert!(
        err.contains(&format!("skopeo copy docker://{BASE_REF} oci:{}", dir.display())),
        "gives the exact command that supplies it: {err}"
    );
    assert!(
        !state.join("app/index.json").exists(),
        "no image may be written"
    );
}

#[test]
fn t3_base_with_a_missing_blob_fails_naming_the_digest() {
    let work = tempfile::tempdir().unwrap();
    let state = work.path().join("state");
    let base = state.join(BASE_DIR_NAME);
    write_base(&base, 0x20, b"base-layer-bytes");
    let layer = base_layer_digest(0x20);
    std::fs::remove_file(base.join("blobs/sha256").join(layer.strip_prefix("sha256:").unwrap()))
        .unwrap();
    let cfg = write_config(work.path(), true);

    let err = build(&state, &cfg).expect_err("a base with a missing blob must be refused");

    assert!(err.contains(&layer), "names the missing digest: {err}");
    assert!(err.contains(BASE_REF), "names the base reference: {err}");
    assert!(!state.join("app/index.json").exists(), "no image written");
}

#[test]
fn t4_same_config_and_base_give_the_same_manifest_digest_in_two_output_dirs() {
    let work = tempfile::tempdir().unwrap();
    let cfg = write_config(work.path(), true);
    let digest = |name: &str| {
        let state = work.path().join(name);
        write_base(&state.join(BASE_DIR_NAME), 0x30, b"base-layer-bytes");
        build(&state, &cfg).unwrap();
        manifest_digest(&state.join("app"))
    };
    assert_eq!(digest("first"), digest("second"));
}

#[test]
fn t5_cache_keys_on_the_base_identity() {
    let work = tempfile::tempdir().unwrap();
    let state = work.path().join("state");
    let base = state.join(BASE_DIR_NAME);
    let cfg = write_config(work.path(), true);
    let out = state.join("app");

    write_base(&base, 0x40, b"base-A");
    build(&state, &cfg).unwrap();
    let key_a = std::fs::read_to_string(out.join("build-cache.hash")).unwrap();

    // Same base, same files: a cache hit, so nothing is reassembled. The
    // deleted index.json is the witness — a rebuild would write it back.
    std::fs::remove_file(out.join("index.json")).unwrap();
    build(&state, &cfg).unwrap();
    assert!(
        !out.join("index.json").exists(),
        "unchanged base + unchanged files must be a cache hit"
    );

    // A different base (different layer bytes and digests) must miss.
    std::fs::remove_dir_all(&base).unwrap();
    write_base(&base, 0x50, b"base-B-different");
    build(&state, &cfg).unwrap();
    assert!(
        out.join("index.json").exists(),
        "changing the base must invalidate the build cache"
    );
    let key_b = std::fs::read_to_string(out.join("build-cache.hash")).unwrap();
    assert_ne!(key_a, key_b, "the input hash must include the base identity");
    assert_eq!(read_manifest(&out).layers[0].digest, base_layer_digest(0x50));
}

#[test]
fn t6_no_image_field_builds_a_scratch_image_as_before() {
    let work = tempfile::tempdir().unwrap();
    let state = work.path().join("state");
    let cfg = write_config(work.path(), false);

    build(&state, &cfg).expect("no `image:` means no base is required");

    assert_eq!(
        read_manifest(&state.join("app")).layers.len(),
        1,
        "only the built layer"
    );
}

#[test]
fn t7_report_json_carries_the_base_reference_and_layer_count() {
    use super::build_image_report::build_json_doc;
    let out = Path::new("state/images/app");
    let with = build_json_doc("t:1", out, None, false, Some((BASE_REF, 3)), None);
    assert_eq!(with["base"], BASE_REF);
    assert_eq!(with["base_layers"], 3);
    let without = build_json_doc("t:1", out, None, false, None, None);
    assert!(without["base"].is_null());
    assert_eq!(without["base_layers"], 0);
    for k in ["tag", "layout", "layout_exists", "cached", "layers", "layer_count", "total_size"] {
        assert!(with.get(k).is_some(), "existing field {k} must stay");
    }
}

/// `s` as a YAML double-quoted scalar; control characters (DEL included, which
/// JSON leaves raw and YAML rejects) are written as `\xNN` escapes.
fn yaml_quote(s: &str) -> String {
    let body: String = s
        .chars()
        .map(|c| match c {
            '"' | '\\' => format!("\\{c}"),
            c if c.is_control() => format!("\\x{:02x}", c as u32),
            c => c.to_string(),
        })
        .collect();
    format!("\"{body}\"")
}

/// Config whose `image:` is `base`, verbatim.
fn write_config_with_base(dir: &Path, base: &str) -> PathBuf {
    let path = dir.join("forjar.yaml");
    std::fs::write(
        &path,
        format!(
            "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\n\
             resources:\n  app:\n    type: image\n    machine: m\n    name: myapp\n    \
             version: \"1.0\"\n    image: {}\n    path: /app/bin\n",
            yaml_quote(base)
        ),
    )
    .unwrap();
    path
}

/// A base reference is handed to the container runtime as a bare argument
/// (`<rt> image inspect <ref>`, `<rt> run .. <ref>`), so one that is empty,
/// starts with '-', or holds whitespace or control characters is refused at
/// the single entry point, before any lookup: even when a layout for it exists
/// on disk, and without producing a "fetch it with" hint for it.
#[test]
fn t8_unsafe_base_references_are_refused_before_any_lookup() {
    for bad in ["", "-x", "--privileged", "-", "a b", " lead", "a\tb", "a\nb", "a\u{7f}b"] {
        let work = tempfile::tempdir().unwrap();
        let state = work.path().join("state");
        // A complete layout exists under the directory name the reference maps to.
        write_base(&state.join(bad.replace([':', '/'], "_")), 0x10, b"base-layer");
        let cfg = write_config_with_base(work.path(), bad);

        let err = build(&state, &cfg)
            .expect_err(&format!("base reference {bad:?} must be refused, not built on"));

        assert!(err.contains(&format!("{bad:?}")), "names the reference {bad:?}: {err}");
        for hint in ["skopeo", "docker pull", "docker save", "Fetch it with"] {
            assert!(!err.contains(hint), "no fetch hint for {bad:?} ({hint}): {err}");
        }
        assert!(!state.join("app/index.json").exists(), "nothing written for {bad:?}");
    }
}
