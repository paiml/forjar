//! FJ-3600: Distribution artifact generation falsification.
//!
//! Popperian rejection criteria for:
//! - Shell installer generation (FJ-3601)
//! - cargo-binstall metadata (FJ-3603)
//! - GitHub Action generation (FJ-3605)
//! - Debian/RPM packaging (FJ-3606)
//! - Helper functions (to_class_name, to_rust_triple)
//! - Config parsing with dist: section
//!
//! Homebrew (FJ-3602) and Nix (FJ-3604) generation live in
//! falsification_dist_checksums.rs (PMAT-080: real checksum resolution).
//!
//! Usage: cargo test --test falsification_dist

use forjar::core::types::{DistBinaryTarget, DistConfig};

// ============================================================================
// Helpers
// ============================================================================

fn minimal_dist() -> DistConfig {
    DistConfig {
        source: "github_release".into(),
        repo: "acme/tool".into(),
        binary: "mytool".into(),
        targets: vec![linux_gnu_x86(), darwin_aarch64()],
        install_dir: "/usr/local/bin".into(),
        install_dir_fallback: "~/.local/bin".into(),
        checksums: Some("SHA256SUMS".into()),
        checksum_algo: "sha256".into(),
        description: "A test tool".into(),
        homepage: "https://example.com".into(),
        license: "MIT".into(),
        maintainer: "Test Author".into(),
        version_cmd: Some("mytool --version".into()),
        latest_tag: true,
        post_install: None,
        homebrew: None,
        nix: None,
    }
}

fn linux_gnu_x86() -> DistBinaryTarget {
    DistBinaryTarget {
        os: "linux".into(),
        arch: "x86_64".into(),
        asset: "mytool-{version}-x86_64-unknown-linux-gnu.tar.gz".into(),
        libc: Some("gnu".into()),
    }
}

fn linux_musl_x86() -> DistBinaryTarget {
    DistBinaryTarget {
        os: "linux".into(),
        arch: "x86_64".into(),
        asset: "mytool-{version}-x86_64-unknown-linux-musl.tar.gz".into(),
        libc: Some("musl".into()),
    }
}

fn darwin_aarch64() -> DistBinaryTarget {
    DistBinaryTarget {
        os: "darwin".into(),
        arch: "aarch64".into(),
        asset: "mytool-{version}-aarch64-apple-darwin.tar.gz".into(),
        libc: None,
    }
}

// ============================================================================
// FJ-3601: Shell installer — structural invariants
// ============================================================================

#[test]
fn installer_is_posix_sh() {
    let script = forjar::cli::dist_generators::generate_installer(&minimal_dist());
    assert!(
        script.starts_with("#!/bin/sh\n"),
        "installer must use POSIX sh shebang, not bash"
    );
}

#[test]
fn installer_uses_set_eu() {
    let script = forjar::cli::dist_generators::generate_installer(&minimal_dist());
    assert!(
        script.contains("set -eu"),
        "installer must enable errexit and nounset"
    );
}

#[test]
fn installer_contains_binary_variable() {
    let script = forjar::cli::dist_generators::generate_installer(&minimal_dist());
    assert!(
        script.contains(r#"BINARY="mytool""#),
        "installer must define BINARY variable matching dist.binary"
    );
}

#[test]
fn installer_contains_repo_variable() {
    let script = forjar::cli::dist_generators::generate_installer(&minimal_dist());
    assert!(
        script.contains(r#"REPO="acme/tool""#),
        "installer must define REPO variable matching dist.repo"
    );
}

#[test]
fn installer_has_platform_detection() {
    let script = forjar::cli::dist_generators::generate_installer(&minimal_dist());
    assert!(
        script.contains("detect_os()"),
        "installer must have detect_os function"
    );
    assert!(
        script.contains("detect_arch()"),
        "installer must have detect_arch function"
    );
    assert!(
        script.contains("detect_libc()"),
        "installer must have detect_libc function"
    );
}

#[test]
fn installer_has_download_helpers() {
    let script = forjar::cli::dist_generators::generate_installer(&minimal_dist());
    assert!(
        script.contains("curl") && script.contains("wget"),
        "installer must support both curl and wget"
    );
}

#[test]
fn installer_includes_checksum_verification() {
    let script = forjar::cli::dist_generators::generate_installer(&minimal_dist());
    assert!(
        script.contains("verify_checksum"),
        "installer must call verify_checksum when checksums configured"
    );
    assert!(
        script.contains("SHA256SUMS"),
        "installer must reference configured checksum file"
    );
}

#[test]
fn installer_skips_checksum_when_none() {
    let mut dist = minimal_dist();
    dist.checksums = None;
    let script = forjar::cli::dist_generators::generate_installer(&dist);
    assert!(
        script.contains("no checksums configured"),
        "installer must skip checksum when dist.checksums is None"
    );
}

#[test]
fn installer_includes_version_verify() {
    let script = forjar::cli::dist_generators::generate_installer(&minimal_dist());
    assert!(
        script.contains(r#""$DEST/$BINARY" --version"#),
        "installer must run version_cmd against the just-installed binary"
    );
}

#[test]
fn installer_omits_version_verify_when_none() {
    let mut dist = minimal_dist();
    dist.version_cmd = None;
    let script = forjar::cli::dist_generators::generate_installer(&dist);
    assert!(
        !script.contains("verifying install"),
        "installer must omit version check when version_cmd is None"
    );
}

#[test]
fn installer_has_fallback_directory() {
    let script = forjar::cli::dist_generators::generate_installer(&minimal_dist());
    assert!(
        script.contains("$HOME/.local/bin"),
        "quoted ~ never expands; generator must emit $HOME (PMAT-077)"
    );
}

#[test]
fn installer_handles_post_install_script() {
    let mut dist = minimal_dist();
    dist.post_install = Some("echo installed".into());
    let script = forjar::cli::dist_generators::generate_installer(&dist);
    assert!(
        script.contains("echo installed"),
        "installer must embed post_install script"
    );
}

#[test]
fn installer_noop_post_install_when_none() {
    let script = forjar::cli::dist_generators::generate_installer(&minimal_dist());
    // When no post_install, the function should be a no-op (:)
    assert!(
        script.contains("post_install()"),
        "installer must always define post_install function"
    );
}

#[test]
fn installer_groups_libc_variants_into_single_case() {
    let mut dist = minimal_dist();
    dist.targets = vec![linux_gnu_x86(), linux_musl_x86(), darwin_aarch64()];
    let script = forjar::cli::dist_generators::generate_installer(&dist);
    // linux/x86_64 should appear exactly once as a case pattern
    let count = script.matches("linux/x86_64)").count();
    assert_eq!(
        count, 1,
        "installer must group libc variants under a single case arm, found {count} occurrences"
    );
}

#[test]
fn installer_contains_all_target_assets() {
    let mut dist = minimal_dist();
    dist.targets = vec![linux_gnu_x86(), linux_musl_x86(), darwin_aarch64()];
    let script = forjar::cli::dist_generators::generate_installer(&dist);
    assert!(
        script.contains("x86_64-unknown-linux-gnu"),
        "installer must reference gnu asset"
    );
    assert!(
        script.contains("x86_64-unknown-linux-musl"),
        "installer must reference musl asset"
    );
    assert!(
        script.contains("aarch64-apple-darwin"),
        "installer must reference darwin asset"
    );
}

#[test]
fn installer_has_argument_parsing() {
    let script = forjar::cli::dist_generators::generate_installer(&minimal_dist());
    assert!(
        script.contains("--version"),
        "installer must accept --version flag"
    );
    assert!(
        script.contains("--force"),
        "installer must accept --force flag"
    );
    assert!(
        script.contains("--prefix"),
        "installer must accept --prefix flag"
    );
}

#[test]
fn installer_empty_targets_no_case_arms() {
    let mut dist = minimal_dist();
    dist.targets = vec![];
    let script = forjar::cli::dist_generators::generate_installer(&dist);
    // Should still generate a valid script with the catch-all case
    assert!(
        script.contains("no pre-built binary"),
        "installer must have catch-all case even with empty targets"
    );
}

// ============================================================================
// FJ-3603: cargo-binstall
// ============================================================================

#[test]
fn binstall_has_metadata_section() {
    let toml = forjar::cli::dist_generators::generate_binstall(&minimal_dist());
    assert!(
        toml.contains("[package.metadata.binstall]"),
        "binstall must define [package.metadata.binstall] section"
    );
}

#[test]
fn binstall_has_pkg_url() {
    let toml = forjar::cli::dist_generators::generate_binstall(&minimal_dist());
    assert!(
        toml.contains("pkg-url"),
        "binstall must define pkg-url template"
    );
}

#[test]
fn binstall_has_target_overrides() {
    let toml = forjar::cli::dist_generators::generate_binstall(&minimal_dist());
    assert!(
        toml.contains("[package.metadata.binstall.overrides."),
        "binstall must have per-target override sections"
    );
}

#[test]
fn binstall_references_repo_url() {
    let toml = forjar::cli::dist_generators::generate_binstall(&minimal_dist());
    assert!(
        toml.contains("github.com/acme/tool"),
        "binstall must reference the dist.repo in URLs"
    );
}

// ============================================================================
// FJ-3605: GitHub Action
// ============================================================================

#[test]
fn github_action_has_name() {
    let action = forjar::cli::dist_generators_b::generate_github_action(&minimal_dist());
    assert!(
        action.contains("name: Setup mytool"),
        "github action must have name with binary"
    );
}

#[test]
fn github_action_only_linux_targets() {
    let action = forjar::cli::dist_generators_b::generate_github_action(&minimal_dist());
    assert!(
        !action.contains("darwin"),
        "github action must only include linux targets (GHA runs Ubuntu)"
    );
}

#[test]
fn github_action_skips_musl() {
    let mut dist = minimal_dist();
    dist.targets = vec![linux_gnu_x86(), linux_musl_x86()];
    let action = forjar::cli::dist_generators_b::generate_github_action(&dist);
    assert!(
        !action.contains("musl"),
        "github action must prefer gnu over musl for Ubuntu runners"
    );
}

#[test]
fn github_action_has_version_input() {
    let action = forjar::cli::dist_generators_b::generate_github_action(&minimal_dist());
    assert!(
        action.contains("version:"),
        "github action must expose version input"
    );
    assert!(
        action.contains("default: \"latest\""),
        "github action version must default to latest"
    );
}

#[test]
fn github_action_is_composite() {
    let action = forjar::cli::dist_generators_b::generate_github_action(&minimal_dist());
    assert!(
        action.contains("using: composite"),
        "github action must be a composite action"
    );
}

// ============================================================================
// FJ-3606: RPM spec
// ============================================================================

#[test]
fn rpm_contains_name() {
    let spec = forjar::cli::dist_generators_b::generate_rpm(&minimal_dist());
    assert!(
        spec.contains("Name:    mytool"),
        "rpm spec must have Name matching dist.binary"
    );
}

#[test]
fn rpm_contains_license() {
    let spec = forjar::cli::dist_generators_b::generate_rpm(&minimal_dist());
    assert!(
        spec.contains("License: MIT"),
        "rpm spec must include dist.license"
    );
}

#[test]
fn rpm_contains_homepage() {
    let spec = forjar::cli::dist_generators_b::generate_rpm(&minimal_dist());
    assert!(
        spec.contains("URL:     https://example.com"),
        "rpm spec must include dist.homepage as URL"
    );
}

#[test]
fn rpm_contains_install_section() {
    let spec = forjar::cli::dist_generators_b::generate_rpm(&minimal_dist());
    assert!(
        spec.contains("%install"),
        "rpm spec must have %install section"
    );
    assert!(
        spec.contains("usr/local/bin"),
        "rpm spec must install to /usr/local/bin"
    );
}

#[test]
fn rpm_prefers_gnu_source() {
    let mut dist = minimal_dist();
    dist.targets = vec![linux_gnu_x86(), linux_musl_x86()];
    let spec = forjar::cli::dist_generators_b::generate_rpm(&dist);
    assert!(
        spec.contains("linux-gnu"),
        "rpm spec must use gnu target for Source0, not musl"
    );
}

// ============================================================================
// Helpers: to_class_name
// ============================================================================

#[test]
fn class_name_capitalizes_simple() {
    assert_eq!(
        forjar::cli::dist_generators_b::to_class_name("forjar"),
        "Forjar"
    );
}

#[test]
fn class_name_handles_hyphens() {
    assert_eq!(
        forjar::cli::dist_generators_b::to_class_name("my-cool-tool"),
        "MyCoolTool"
    );
}

#[test]
fn class_name_single_char() {
    assert_eq!(forjar::cli::dist_generators_b::to_class_name("x"), "X");
}

#[test]
fn class_name_empty_string() {
    assert_eq!(forjar::cli::dist_generators_b::to_class_name(""), "");
}

// ============================================================================
// Helpers: to_rust_triple
// ============================================================================

#[test]
fn rust_triple_linux_gnu() {
    let t = linux_gnu_x86();
    assert_eq!(
        forjar::cli::dist_generators_b::to_rust_triple(&t),
        "x86_64-unknown-linux-gnu"
    );
}

#[test]
fn rust_triple_linux_musl() {
    let t = linux_musl_x86();
    assert_eq!(
        forjar::cli::dist_generators_b::to_rust_triple(&t),
        "x86_64-unknown-linux-musl"
    );
}

#[test]
fn rust_triple_darwin() {
    let t = darwin_aarch64();
    assert_eq!(
        forjar::cli::dist_generators_b::to_rust_triple(&t),
        "aarch64-apple-darwin"
    );
}

#[test]
fn rust_triple_linux_default_libc_is_gnu() {
    let t = DistBinaryTarget {
        os: "linux".into(),
        arch: "x86_64".into(),
        asset: "test".into(),
        libc: None,
    };
    assert_eq!(
        forjar::cli::dist_generators_b::to_rust_triple(&t),
        "x86_64-unknown-linux-gnu",
        "linux with no libc specified must default to gnu"
    );
}

// ============================================================================
// Config parsing: dist: section round-trip
// ============================================================================

#[test]
fn config_with_dist_section_parses() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: localhost
    user: root
resources: {}
dist:
  source: github_release
  repo: acme/tool
  binary: mytool
  targets:
    - os: linux
      arch: x86_64
      asset: "mytool-{version}-x86_64-unknown-linux-gnu.tar.gz"
      libc: gnu
  install_dir: /usr/local/bin
  description: "A test tool"
  homepage: "https://example.com"
  license: MIT
"#;
    let config: forjar::core::types::ForjarConfig =
        serde_yaml_ng::from_str(yaml).expect("config with dist: section must parse");
    let dist = config.dist.expect("dist must be Some");
    assert_eq!(dist.binary, "mytool");
    assert_eq!(dist.targets.len(), 1);
    assert_eq!(dist.targets[0].os, "linux");
}

#[test]
fn config_without_dist_section_parses() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: localhost
    user: root
resources: {}
"#;
    let config: forjar::core::types::ForjarConfig =
        serde_yaml_ng::from_str(yaml).expect("config without dist: must parse");
    assert!(
        config.dist.is_none(),
        "dist must be None when not specified"
    );
}

// ============================================================================
// Edge cases: empty/minimal configs
// ============================================================================

#[test]
fn installer_with_empty_description_uses_binary() {
    let mut dist = minimal_dist();
    dist.description = String::new();
    let script = forjar::cli::dist_generators::generate_installer(&dist);
    // The binary name "mytool" should still appear in the description comment
    assert!(
        script.contains("mytool"),
        "installer must fall back to binary name when description empty"
    );
}
