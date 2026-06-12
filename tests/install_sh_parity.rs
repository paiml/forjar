//! PMAT-077: pins that keep the curl-install path working end-to-end.
//!
//! The v1.4.2 installer was broken three independent ways: mixed v-prefix
//! asset naming between the two release workflows, a missing SHA256SUMS
//! asset, and an extraction path that ignored the directory inside the
//! tarball. Each pin below guards one link of the chain:
//! committed install.sh == generator output == workflow asset naming.

use std::fs;

fn repo_file(name: &str) -> String {
    fs::read_to_string(name).unwrap_or_else(|e| panic!("cannot read {name}: {e}"))
}

/// The committed dogfood install.sh must be exactly what `forjar dist
/// --installer` generates from dist-forjar.yaml — no hand edits.
#[test]
fn committed_install_sh_matches_generator_output() {
    let yaml = repo_file("dist-forjar.yaml");
    let config: forjar::core::types::ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let dist = config
        .dist
        .expect("dist-forjar.yaml must have a dist: block");
    let generated = forjar::cli::dist_generators::generate_installer(&dist);
    let committed = repo_file("install.sh");
    assert_eq!(
        committed, generated,
        "install.sh is stale — regenerate with: cargo run -- dist -f dist-forjar.yaml --installer -o install.sh"
    );
}

#[test]
fn install_sh_has_no_placeholder_urls() {
    let script = repo_file("install.sh");
    assert!(
        !script.contains("example.com"),
        "usage header must point at the real raw URL"
    );
    assert!(script.contains("https://raw.githubusercontent.com/paiml/forjar/main/install.sh"));
}

#[test]
fn install_sh_fallback_dir_expands() {
    let script = repo_file("install.sh");
    assert!(
        script.contains(r#"FALLBACK_DIR="$HOME/"#),
        "a quoted ~ never tilde-expands; fallback dir must use $HOME"
    );
}

#[test]
fn install_sh_extracts_from_archive_directory() {
    let script = repo_file("install.sh");
    assert!(
        script.contains(r#"SRC="$TMPDIR/${ASSET%.tar.gz}/$BINARY""#),
        "tarballs contain a directory named after the asset; cp from it"
    );
}

#[test]
fn install_sh_falls_back_to_per_asset_sha256() {
    let script = repo_file("install.sh");
    assert!(
        script.contains("${ASSET}.sha256"),
        "per-asset .sha256 fallback missing"
    );
}

/// install.sh resolves `forjar-<x.y.z>-<target>.tar.gz` (no v prefix);
/// binary-release.yml must produce exactly that scheme and upload the
/// combined SHA256SUMS install.sh prefers.
#[test]
fn workflow_asset_naming_matches_installer_expectations() {
    let workflow = repo_file(".github/workflows/binary-release.yml");
    assert!(
        workflow.contains(r#"VERSION="${TAG#v}""#),
        "binary-release.yml must strip the v prefix from asset names"
    );
    assert!(
        workflow.contains("SHA256SUMS"),
        "combined SHA256SUMS job missing"
    );

    let script = repo_file("install.sh");
    for target in [
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-gnu",
        "aarch64-unknown-linux-musl",
    ] {
        let asset = format!("forjar-{{version}}-{target}.tar.gz");
        assert!(
            script.contains(&asset),
            "install.sh missing workflow-built target {target}"
        );
        assert!(
            workflow.contains(target),
            "binary-release.yml missing target {target}"
        );
    }
}
