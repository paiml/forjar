//! IMG-001 gap 1: a base declared by digest must resolve to THAT digest.

use super::base_resolve::*;

const D: &str = "sha256:0a9887931b0e38af2e3487ed39e29436b0535c4f3f52e57745baaf79d1898956";

#[test]
fn pinned_base_accepts_a_matching_repo_digest() {
    let seen = vec![format!("ubuntu@{D}")];
    assert!(verify_declared_digest(&format!("ubuntu@{D}"), &seen).is_ok());
}

#[test]
fn pinned_base_refuses_a_different_digest_and_names_both() {
    let other = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    let err =
        verify_declared_digest(&format!("ubuntu@{D}"), &[format!("ubuntu@{other}")]).unwrap_err();
    assert!(err.contains(D) && err.contains(other), "{err}");
}

#[test]
fn pinned_base_refuses_when_nothing_could_be_measured() {
    let err = verify_declared_digest(&format!("ubuntu@{D}"), &[]).unwrap_err();
    assert!(err.contains("could not be verified"), "{err}");
}

#[test]
fn tag_only_base_is_not_a_digest_claim() {
    assert!(verify_declared_digest("ubuntu:22.04", &[]).is_ok());
}

#[test]
fn local_layout_dir_name_matches_the_historic_state_images_convention() {
    assert_eq!(local_base_dir_name("ubuntu:22.04"), "ubuntu_22.04");
    assert_eq!(local_base_dir_name("a/b:c"), "a_b_c");
}

#[test]
fn a_real_registry_config_with_null_collections_parses() {
    // Shape docker/containerd emit for ubuntu: Entrypoint, ExposedPorts and
    // Labels are JSON null, not absent.
    let raw = r#"{"architecture":"amd64","os":"linux","config":{"Env":["PATH=/usr/bin"],
        "Entrypoint":null,"Cmd":["/bin/bash"],"ExposedPorts":null,"Labels":null,
        "WorkingDir":"","User":""},
        "rootfs":{"type":"layers","diff_ids":["sha256:aa"]},"history":null}"#;
    let cfg: crate::core::types::OciImageConfig = serde_json::from_str(raw).unwrap();
    assert!(cfg.config.entrypoint.is_empty());
    assert_eq!(cfg.config.cmd, vec!["/bin/bash"]);
    assert!(cfg.history.is_empty());
}

#[test]
fn img001_pushed_reference_is_name_at_digest() {
    let r =
        crate::core::store::image_ref::parse_image_ref("almacen:5000/paiml/aprender-build:spike")
            .unwrap();
    assert_eq!(
        r.to_digest_reference(D),
        format!("almacen:5000/paiml/aprender-build@{D}")
    );
}

// ---- forjar#757 quorum findings: a pinned LOCAL base is verified; a declared
// base that cannot be produced fails instead of becoming a scratch image ----

use super::tests_image_assembler_img001::{write_base_layout, BASE_MANIFEST_HEX};

fn local_base_root(base: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join(local_base_dir_name(base));
    std::fs::create_dir_all(&dir).unwrap();
    write_base_layout(&dir);
    root
}

#[test]
fn pinned_local_base_with_a_different_manifest_digest_is_refused() {
    let pinned = format!("ubuntu@sha256:{}", "1".repeat(64));
    let root = local_base_root(&pinned);
    let err = load_local_base(root.path(), &pinned).unwrap_err();
    assert!(err.contains("mismatch"), "{err}");
    assert!(
        err.contains(BASE_MANIFEST_HEX),
        "names what it found: {err}"
    );
}

#[test]
fn pinned_local_base_with_the_matching_digest_loads() {
    let pinned = format!("ubuntu@sha256:{BASE_MANIFEST_HEX}");
    let root = local_base_root(&pinned);
    assert!(load_local_base(root.path(), &pinned).unwrap().is_some());
}

#[test]
fn pinned_local_base_whose_digest_cannot_be_measured_is_refused() {
    let pinned = format!("ubuntu@sha256:{BASE_MANIFEST_HEX}");
    let root = local_base_root(&pinned);
    let dir = root.path().join(local_base_dir_name(&pinned));
    std::fs::write(
        dir.join("index.json"),
        b"{\"schemaVersion\":2,\"manifests\":[]}",
    )
    .unwrap();
    assert!(load_local_base(root.path(), &pinned).is_err());
}

#[test]
fn tag_only_local_base_loads_without_a_digest_claim() {
    let root = local_base_root("ubuntu:22.04");
    assert!(load_local_base(root.path(), "ubuntu:22.04")
        .unwrap()
        .is_some());
}

#[test]
fn absent_local_base_is_none_not_an_error() {
    let root = tempfile::tempdir().unwrap();
    assert!(load_local_base(root.path(), "ubuntu:22.04")
        .unwrap()
        .is_none());
}

#[test]
fn unreadable_local_base_layout_is_an_error_not_a_skip() {
    let root = local_base_root("ubuntu:22.04");
    let dir = root.path().join(local_base_dir_name("ubuntu:22.04"));
    std::fs::write(dir.join("index.json"), b"not json").unwrap();
    assert!(load_local_base(root.path(), "ubuntu:22.04").is_err());
}

#[test]
fn files_only_build_without_a_local_base_or_runtime_fails_instead_of_going_scratch() {
    let root = tempfile::tempdir().unwrap();
    let work = root.path().join("w");
    let err = resolve_build_base("ubuntu:22.04", root.path(), &work, None).unwrap_err();
    assert!(
        err.contains("ubuntu:22.04") && err.contains("cannot"),
        "{err}"
    );
}

#[test]
fn files_only_build_prefers_a_verified_local_base() {
    let root = local_base_root("ubuntu:22.04");
    let work = root.path().join("w");
    assert!(resolve_build_base("ubuntu:22.04", root.path(), &work, None).is_ok());
}
