//! Refs #210 / #213: `oci-pack` must write the layout it reports.
//!
//! The published 1.12.3 exited 0 with "OCI layout generation requires
//! sha2+flate2 crates" and created nothing at all — no `--output` directory,
//! no `oci-layout`, no `index.json`, no blobs — and with `--json` it printed a
//! complete OCI manifest for that non-existent layout. These tests fail on
//! that binary and pass on this one.

use super::*;

/// A directory with two files at different depths, to prove the walk is real.
fn sample_tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"alpha").unwrap();
    std::fs::create_dir_all(dir.path().join("etc")).unwrap();
    std::fs::write(dir.path().join("etc/app.conf"), b"key=value").unwrap();
    dir
}

#[test]
fn pack_writes_a_real_oci_layout() {
    let src = sample_tree();
    let out = tempfile::tempdir().unwrap();
    let layout = out.path().join("oci-output");

    cmd_oci_pack(src.path(), "myapp:v1", &layout, false).expect("pack");

    // RED on 1.12.3: none of these exist there.
    assert!(layout.join("oci-layout").is_file(), "oci-layout missing");
    assert!(layout.join("index.json").is_file(), "index.json missing");
    let blobs: Vec<_> = std::fs::read_dir(layout.join("blobs/sha256"))
        .expect("blobs dir")
        .filter_map(Result::ok)
        .collect();
    assert!(
        blobs.len() >= 3,
        "expected layer + config + manifest blobs, got {}",
        blobs.len()
    );
}

#[test]
fn index_json_points_at_a_blob_that_exists() {
    let src = sample_tree();
    let out = tempfile::tempdir().unwrap();
    let layout = out.path().join("out");
    cmd_oci_pack(src.path(), "myapp:v1", &layout, false).expect("pack");

    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(layout.join("index.json")).unwrap()).unwrap();
    let digest = index["manifests"][0]["digest"]
        .as_str()
        .expect("index has a manifest digest");
    let hex = digest.strip_prefix("sha256:").expect("sha256 digest");
    assert!(
        layout.join("blobs/sha256").join(hex).is_file(),
        "index.json references a blob that was never written: {digest}"
    );
}

#[test]
fn tag_reaches_the_docker_manifest() {
    let src = sample_tree();
    let out = tempfile::tempdir().unwrap();
    let layout = out.path().join("out");
    cmd_oci_pack(src.path(), "ghcr.io/example/app:3.2.1", &layout, false).expect("pack");

    let docker = std::fs::read_to_string(layout.join("manifest.json")).unwrap();
    assert!(
        docker.contains("ghcr.io/example/app:3.2.1"),
        "tag not present in manifest.json: {docker}"
    );
}

#[test]
fn empty_tag_is_refused() {
    let src = sample_tree();
    let out = tempfile::tempdir().unwrap();
    let err = cmd_oci_pack(src.path(), "  ", &out.path().join("o"), false).unwrap_err();
    assert!(err.contains("--tag"), "unexpected error: {err}");
}

// ── Refs #213: the two failure modes must be distinguishable ──────────

#[test]
fn existing_file_is_not_reported_as_missing() {
    let src = sample_tree();
    let file = src.path().join("a.txt");
    let out = tempfile::tempdir().unwrap();

    let err = cmd_oci_pack(&file, "t:1", &out.path().join("o"), false).unwrap_err();
    // RED on 1.12.3: it said "directory '<file>' does not exist".
    assert!(
        !err.contains("does not exist"),
        "an existing file was reported as missing: {err}"
    );
    assert!(err.contains("not a directory"), "unexpected error: {err}");
}

#[test]
fn genuinely_absent_path_still_says_does_not_exist() {
    let out = tempfile::tempdir().unwrap();
    let absent = out.path().join("really-absent");
    let err = cmd_oci_pack(&absent, "t:1", &out.path().join("o"), false).unwrap_err();
    assert!(err.contains("does not exist"), "unexpected error: {err}");
}

// ── Non-regression: a real pack still packs ───────────────────────────

#[test]
fn json_and_text_pack_the_same_bytes() {
    let src = sample_tree();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let la = a.path().join("l");
    let lb = b.path().join("l");
    cmd_oci_pack(src.path(), "same:1", &la, false).unwrap();
    cmd_oci_pack(src.path(), "same:1", &lb, true).unwrap();

    let names = |root: &std::path::Path| {
        let mut v: Vec<String> = std::fs::read_dir(root.join("blobs/sha256"))
            .unwrap()
            .filter_map(|e| Some(e.ok()?.file_name().to_string_lossy().into_owned()))
            .collect();
        v.sort();
        v
    };
    assert_eq!(
        names(&la),
        names(&lb),
        "--json must not change what is written"
    );
}
