//! Coverage tests for build_distribution.rs — FAR archive creation.

use super::build_distribution::*;

// ── cmd_build_far ────────────────────────────────────────────────────

#[test]
fn build_far_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    std::fs::create_dir_all(&oci).unwrap();
    let result = cmd_build_far("test-image", &oci);
    assert!(result.is_ok());
    let far_path = oci.with_extension("far");
    assert!(far_path.exists());
}

#[test]
fn build_far_with_files() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    std::fs::create_dir_all(&oci).unwrap();
    std::fs::write(oci.join("manifest.json"), r#"{"schemaVersion":2}"#).unwrap();
    std::fs::write(oci.join("index.json"), r#"{"mediaType":"application/vnd.oci.image.index.v1+json"}"#).unwrap();
    let result = cmd_build_far("my-app", &oci);
    assert!(result.is_ok());
    let far_path = oci.with_extension("far");
    assert!(far_path.exists());
    assert!(far_path.metadata().unwrap().len() > 0);
}

#[test]
fn build_far_nested_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    std::fs::create_dir_all(oci.join("blobs/sha256")).unwrap();
    std::fs::write(oci.join("blobs/sha256/abc123"), "layer data").unwrap();
    std::fs::write(oci.join("oci-layout"), r#"{"imageLayoutVersion":"1.0.0"}"#).unwrap();
    let result = cmd_build_far("nested-test", &oci);
    assert!(result.is_ok());
}

// ── cmd_build_push (Refs #210) ───────────────────────────────────────
//
// `--push` used to print "Push complete: N uploaded" and exit 0 while nothing
// was uploaded: it swallowed every transport failure into
// "push skipped: registry unreachable" and returned Ok. These tests pin the
// rule that a push is a failure unless the registry confirms it.
//
// All of them are hermetic: the target is a closed loopback port, never a real
// registry.

/// A registry that is guaranteed not to answer: port 1 on loopback.
const DEAD_REGISTRY: &str = "127.0.0.1:1";

/// Write a minimal but structurally valid OCI layout (layer + config +
/// manifest + index) so `discover_blobs` classifies all three kinds.
fn write_oci_layout(oci: &std::path::Path) {
    let blobs = oci.join("blobs").join("sha256");
    std::fs::create_dir_all(&blobs).unwrap();

    let layer_hex = "1".repeat(64);
    let config_hex = "2".repeat(64);
    let manifest_hex = "3".repeat(64);

    std::fs::write(blobs.join(&layer_hex), b"layer-bytes").unwrap();
    let config = br#"{"architecture":"amd64","os":"linux"}"#;
    std::fs::write(blobs.join(&config_hex), config).unwrap();

    let manifest = format!(
        r#"{{"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json",
"config":{{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"sha256:{config_hex}","size":{}}},
"layers":[{{"mediaType":"application/vnd.oci.image.layer.v1.tar+gzip","digest":"sha256:{layer_hex}","size":11}}]}}"#,
        config.len()
    );
    std::fs::write(blobs.join(&manifest_hex), manifest.as_bytes()).unwrap();

    let index = format!(
        r#"{{"schemaVersion":2,"manifests":[{{"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":"sha256:{manifest_hex}","size":{}}}]}}"#,
        manifest.len()
    );
    std::fs::write(oci.join("index.json"), index.as_bytes()).unwrap();
}

/// THE REGRESSION TEST. A push that cannot reach its registry must fail.
///
/// RED before the fix: `cmd_build_push` matched the transport error, printed
/// "push skipped: registry unreachable (...)" and returned `Ok(())`, so the
/// CLI exited 0 having uploaded nothing.
#[test]
fn push_to_an_unreachable_registry_is_an_error_not_a_skip() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    write_oci_layout(&oci);

    let err = cmd_build_push(&format!("{DEAD_REGISTRY}/team/app:v1"), &oci)
        .expect_err("a push that reached no registry must not report success");

    assert!(
        !err.to_lowercase().contains("skip"),
        "an unreachable registry is a failed push, not a skipped one: {err}"
    );
    assert!(
        err.contains(DEAD_REGISTRY),
        "the error must name the registry it could not push to: {err}"
    );
}

/// An empty layout is nothing to push, and nothing pushed is not a success.
#[test]
fn push_refuses_an_empty_oci_layout() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    std::fs::create_dir_all(&oci).unwrap();

    let err = cmd_build_push(&format!("{DEAD_REGISTRY}/team/app:v1"), &oci)
        .expect_err("pushing an empty layout must not report success");
    assert!(err.contains("nothing to push"), "{err}");
}

/// A layout with blobs but no manifest cannot produce a tag, so it cannot
/// produce a completed push.
#[test]
fn push_refuses_a_layout_with_no_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    let blobs = oci.join("blobs").join("sha256");
    std::fs::create_dir_all(&blobs).unwrap();
    std::fs::write(blobs.join("4".repeat(64)), b"orphan").unwrap();
    std::fs::write(oci.join("index.json"), br#"{"schemaVersion":2,"manifests":[]}"#).unwrap();

    let err = cmd_build_push(&format!("{DEAD_REGISTRY}/team/app:v1"), &oci)
        .expect_err("a layout with no manifest must not report a completed push");
    assert!(err.contains("no manifest"), "{err}");
}

/// The push target comes from the reference, never from a hardcoded default.
#[test]
fn push_rejects_a_reference_it_cannot_target() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    write_oci_layout(&oci);

    for reference in ["", "ghcr.io/foo/bar@sha256:abc", "ghcr.io/Foo/Bar:1"] {
        assert!(
            cmd_build_push(reference, &oci).is_err(),
            "reference {reference:?} must be refused, not silently replaced by a default"
        );
    }
}
