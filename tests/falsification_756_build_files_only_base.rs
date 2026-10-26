//! IMG-001 (forjar#756 / PR #757 review): a files-only `forjar build` of an
//! image that declares a base.
//!
//! * `--json` owns stdout: it must be exactly one JSON document, with the
//!   human base line on stderr (`forjar build --json | jq` must work).
//! * A declared base that cannot be produced FAILS; it never quietly becomes a
//!   scratch image.

use std::path::Path;
use std::process::{Command, Output};

const MANIFEST: &str = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";
const CONFIG: &str = "aabbccdd00112233445566778899aabbccddeeff00112233445566778899aabb";
const LAYER: &str = "11223344556677889900aabbccddeeff00112233445566778899aabbccddeeff";
const DIFF: &str = "99887766554433221100ffeeddccbbaa99887766554433221100ffeeddccbbaa";

fn write_cfg(dir: &Path, image: &str) {
    std::fs::write(
        dir.join("forjar.yaml"),
        format!(
            "version: \"1.0\"\nname: t\nmachines:\n  m: {{hostname: m, addr: 127.0.0.1}}\nresources:\n  img:\n    type: image\n    machine: m\n    image: \"{image}\"\n    tag: \"t.invalid/x:1\"\n    path: /opt/a.txt\n    content: hi\n"
        ),
    )
    .unwrap();
}

/// A structurally valid one-layer OCI layout at `state/images/<name>`.
fn write_base(dir: &Path, name: &str) {
    let d = dir.join("state/images").join(name);
    let blobs = d.join("blobs/sha256");
    std::fs::create_dir_all(&blobs).unwrap();
    std::fs::write(
        blobs.join(CONFIG),
        format!(r#"{{"architecture":"amd64","os":"linux","config":{{}},"rootfs":{{"type":"layers","diff_ids":["sha256:{DIFF}"]}}}}"#),
    )
    .unwrap();
    std::fs::write(blobs.join(LAYER), b"base-layer-bytes").unwrap();
    std::fs::write(
        blobs.join(MANIFEST),
        format!(r#"{{"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"sha256:{CONFIG}","size":1}},"layers":[{{"mediaType":"application/vnd.oci.image.layer.v1.tar+gzip","digest":"sha256:{LAYER}","size":16}}]}}"#),
    )
    .unwrap();
    std::fs::write(
        d.join("index.json"),
        format!(r#"{{"schemaVersion":2,"manifests":[{{"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":"sha256:{MANIFEST}","size":1}}]}}"#),
    )
    .unwrap();
}

fn build(dir: &Path, json: bool) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_forjar"));
    c.current_dir(dir)
        .args(["build", "-f", "forjar.yaml", "--resource", "img"]);
    if json {
        c.arg("--json");
    }
    c.output().unwrap()
}

#[test]
fn json_stdout_is_one_document_and_the_base_line_is_on_stderr() {
    let t = tempfile::tempdir().unwrap();
    write_cfg(t.path(), "ubuntu:22.04");
    write_base(t.path(), "ubuntu_22.04");
    let out = build(t.path(), true);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "build failed: {stderr}");
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not one JSON document ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    });
    assert!(doc["manifest_digest"].is_string());
    assert!(
        stderr.contains("ubuntu"),
        "the base line still reaches the human: {stderr}"
    );
}

#[test]
fn declared_base_that_cannot_be_produced_fails_loudly() {
    let t = tempfile::tempdir().unwrap();
    write_cfg(t.path(), "forjar-756-no-such-base.invalid/nothing:0");
    let out = build(t.path(), false);
    assert!(
        !out.status.success(),
        "built a scratch image under a declared base: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("forjar-756-no-such-base"));
}

fn write_cfg_scratch(dir: &Path) {
    std::fs::write(
        dir.join("forjar.yaml"),
        "version: \"1.0\"\nname: t\nmachines:\n  m: {hostname: m, addr: 127.0.0.1}\nresources:\n  img:\n    type: image\n    machine: m\n    tag: \"t.invalid/x:1\"\n    path: /opt/a.txt\n    content: hi\n",
    )
    .unwrap();
}

fn json_doc(out: &Output) -> serde_json::Value {
    assert!(
        out.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("one JSON document")
}

/// A layout first built as scratch must NOT come back CACHED once a base is
/// declared and available: the base is part of the cache key.
#[test]
fn scratch_build_then_a_base_is_not_a_cache_hit() {
    let t = tempfile::tempdir().unwrap();
    write_cfg_scratch(t.path());
    let first = json_doc(&build(t.path(), true));
    assert_eq!(first["cached"], false);
    let hash_scratch =
        std::fs::read_to_string(t.path().join("state/images/img/build-cache.hash")).unwrap();

    write_cfg(t.path(), "ubuntu:22.04");
    write_base(t.path(), "ubuntu_22.04");
    let second = json_doc(&build(t.path(), true));
    assert_eq!(
        second["cached"], false,
        "scratch build served as the based image"
    );
    let hash_based =
        std::fs::read_to_string(t.path().join("state/images/img/build-cache.hash")).unwrap();
    assert_ne!(hash_scratch, hash_based, "input_hash ignores the base");
}

/// The same base twice is still a hit.
#[test]
fn same_base_twice_is_a_cache_hit() {
    let t = tempfile::tempdir().unwrap();
    write_cfg(t.path(), "ubuntu:22.04");
    write_base(t.path(), "ubuntu_22.04");
    assert_eq!(json_doc(&build(t.path(), true))["cached"], false);
    assert_eq!(json_doc(&build(t.path(), true))["cached"], true);
}
