//! PMAT-080: Falsification tests for checksum-bearing dist artifacts.
//!
//! Homebrew formula (FJ-3602) and Nix flake (FJ-3604) generation with
//! real checksums resolved from a pinned release (spec F-3608/F-3610).
//!
//! Usage: cargo test --test falsification_dist_checksums

use forjar::cli::dist_checksums::{parse_sha256sums, resolve_release, ResolvedRelease};
use forjar::core::types::{DistBinaryTarget, DistConfig, DistHomebrewConfig};

// ============================================================================
// Helpers
// ============================================================================

/// SHA256SUMS fixture for all mytool assets at v1.1.1 (offline — no network).
fn sums_fixture() -> &'static str {
    "1f2e3d4c5b6a79881f2e3d4c5b6a79881f2e3d4c5b6a79881f2e3d4c5b6a7988  mytool-1.1.1-x86_64-unknown-linux-gnu.tar.gz\n\
     2e3d4c5b6a7988f12e3d4c5b6a7988f12e3d4c5b6a7988f12e3d4c5b6a7988f1  mytool-1.1.1-x86_64-unknown-linux-musl.tar.gz\n\
     3d4c5b6a7988f1e23d4c5b6a7988f1e23d4c5b6a7988f1e23d4c5b6a7988f1e2  mytool-1.1.1-aarch64-apple-darwin.tar.gz\n\
     4c5b6a7988f1e2d34c5b6a7988f1e2d34c5b6a7988f1e2d34c5b6a7988f1e2d3  mytool-1.1.1-aarch64-unknown-linux-gnu.tar.gz\n"
}

/// Resolved release for the fixture (PMAT-080 checksum resolution).
fn release() -> ResolvedRelease {
    ResolvedRelease {
        version: "1.1.1".into(),
        checksums: parse_sha256sums(sums_fixture()),
    }
}

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

fn linux_aarch64() -> DistBinaryTarget {
    DistBinaryTarget {
        os: "linux".into(),
        arch: "aarch64".into(),
        asset: "mytool-{version}-aarch64-unknown-linux-gnu.tar.gz".into(),
        libc: Some("gnu".into()),
    }
}

// ============================================================================
// FJ-3602: Homebrew formula
// ============================================================================

#[test]
fn homebrew_has_class_declaration() {
    let formula =
        forjar::cli::dist_generators::generate_homebrew(&minimal_dist(), &release()).unwrap();
    assert!(
        formula.contains("class Mytool < Formula"),
        "homebrew must declare a Formula class with capitalized binary name"
    );
}

#[test]
fn homebrew_contains_description() {
    let formula =
        forjar::cli::dist_generators::generate_homebrew(&minimal_dist(), &release()).unwrap();
    assert!(
        formula.contains("A test tool"),
        "homebrew must embed dist.description"
    );
}

#[test]
fn homebrew_skips_musl_targets() {
    let mut dist = minimal_dist();
    dist.targets = vec![linux_gnu_x86(), linux_musl_x86(), darwin_aarch64()];
    let formula = forjar::cli::dist_generators::generate_homebrew(&dist, &release()).unwrap();
    assert!(
        !formula.contains("musl"),
        "homebrew formula must skip musl targets (Homebrew uses glibc)"
    );
}

#[test]
fn homebrew_nests_arch_inside_os() {
    let mut dist = minimal_dist();
    dist.targets = vec![linux_gnu_x86(), linux_aarch64(), darwin_aarch64()];
    let formula = forjar::cli::dist_generators::generate_homebrew(&dist, &release()).unwrap();
    // on_linux should appear once, containing both arch blocks
    let linux_count = formula.matches("on_linux do").count();
    assert_eq!(
        linux_count, 1,
        "homebrew must nest arch blocks inside a single OS block, found {linux_count} on_linux"
    );
}

#[test]
fn homebrew_includes_caveats() {
    let mut dist = minimal_dist();
    dist.homebrew = Some(DistHomebrewConfig {
        tap: "acme/tap".into(),
        dependencies: vec!["openssl".into()],
        caveats: Some("Run: mytool init".into()),
    });
    let formula = forjar::cli::dist_generators::generate_homebrew(&dist, &release()).unwrap();
    assert!(
        formula.contains("def caveats"),
        "homebrew must include caveats block"
    );
    assert!(
        formula.contains("mytool init"),
        "homebrew must embed caveats text"
    );
}

#[test]
fn homebrew_includes_dependencies() {
    let mut dist = minimal_dist();
    dist.homebrew = Some(DistHomebrewConfig {
        tap: "acme/tap".into(),
        dependencies: vec!["openssl".into(), "libgit2".into()],
        caveats: None,
    });
    let formula = forjar::cli::dist_generators::generate_homebrew(&dist, &release()).unwrap();
    assert!(
        formula.contains(r#"depends_on "openssl""#),
        "homebrew must list openssl dependency"
    );
    assert!(
        formula.contains(r#"depends_on "libgit2""#),
        "homebrew must list libgit2 dependency"
    );
}

#[test]
fn homebrew_has_test_block() {
    let formula =
        forjar::cli::dist_generators::generate_homebrew(&minimal_dist(), &release()).unwrap();
    assert!(
        formula.contains("test do"),
        "homebrew must include a test block"
    );
    assert!(
        formula.contains("shell_output"),
        "homebrew test must invoke the binary"
    );
}

// ============================================================================
// FJ-3604: Nix flake
// ============================================================================

#[test]
fn nix_has_description() {
    let flake = forjar::cli::dist_generators_b::generate_nix(&minimal_dist(), &release()).unwrap();
    assert!(
        flake.contains("A test tool"),
        "nix flake must embed dist.description"
    );
}

#[test]
fn nix_skips_musl_targets() {
    let mut dist = minimal_dist();
    dist.targets = vec![linux_gnu_x86(), linux_musl_x86(), darwin_aarch64()];
    let flake = forjar::cli::dist_generators_b::generate_nix(&dist, &release()).unwrap();
    assert!(
        !flake.contains("musl"),
        "nix flake must skip musl targets (uses system libc)"
    );
}

#[test]
fn nix_maps_targets_to_nix_systems() {
    let mut dist = minimal_dist();
    dist.targets = vec![linux_gnu_x86(), darwin_aarch64()];
    let flake = forjar::cli::dist_generators_b::generate_nix(&dist, &release()).unwrap();
    assert!(
        flake.contains("x86_64-linux"),
        "nix must map linux/x86_64 to x86_64-linux"
    );
    assert!(
        flake.contains("aarch64-darwin"),
        "nix must map darwin/aarch64 to aarch64-darwin"
    );
}

#[test]
fn nix_uses_flake_utils() {
    let flake = forjar::cli::dist_generators_b::generate_nix(&minimal_dist(), &release()).unwrap();
    assert!(
        flake.contains("flake-utils"),
        "nix flake must use flake-utils for eachDefaultSystem"
    );
}

#[test]
fn nix_contains_binary_in_install_phase() {
    let flake = forjar::cli::dist_generators_b::generate_nix(&minimal_dist(), &release()).unwrap();
    assert!(
        flake.contains("cp mytool $out/bin/"),
        "nix flake installPhase must copy binary to $out/bin"
    );
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn homebrew_with_no_caveats_omits_block() {
    let formula =
        forjar::cli::dist_generators::generate_homebrew(&minimal_dist(), &release()).unwrap();
    assert!(
        !formula.contains("def caveats"),
        "homebrew must omit caveats block when not configured"
    );
}

#[test]
fn nix_empty_targets_still_valid() {
    let mut dist = minimal_dist();
    dist.targets = vec![];
    let flake = forjar::cli::dist_generators_b::generate_nix(&dist, &release()).unwrap();
    assert!(
        flake.contains("description"),
        "nix flake with empty targets must still generate valid structure"
    );
}

// ============================================================================
// PMAT-080: Checksum resolution — F-3608 and F-3610
// ============================================================================

/// Build a release through the real `--checksums-file` path (offline fixture).
fn release_from_fixture_file() -> ResolvedRelease {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("SHA256SUMS");
    std::fs::write(&path, sums_fixture()).expect("write fixture");
    resolve_release(&minimal_dist(), Some("v1.1.1"), Some(&path))
        .expect("offline checksum resolution must succeed")
}

/// F-3608: Generated artifacts use real checksums, not placeholders.
/// Spec test: grep for "TODO\|PLACEHOLDER\|000000" in output — must find zero.
#[test]
fn f3608_homebrew_has_no_placeholders() {
    let release = release_from_fixture_file();
    let formula =
        forjar::cli::dist_generators::generate_homebrew(&minimal_dist(), &release).unwrap();
    for needle in ["TODO", "PLACEHOLDER", "000000", "\"VERSION\""] {
        assert!(
            !formula.contains(needle),
            "F-3608 falsified: homebrew formula contains '{needle}'"
        );
    }
    assert!(
        formula.contains(r#"version "1.1.1""#),
        "formula must pin the real version"
    );
    assert!(
        formula.contains("1f2e3d4c5b6a79881f2e3d4c5b6a79881f2e3d4c5b6a79881f2e3d4c5b6a7988"),
        "formula must embed the real gnu/x86_64 sha256"
    );
}

/// F-3608: Nix flake uses real checksums, not placeholders.
#[test]
fn f3608_nix_has_no_placeholders() {
    let release = release_from_fixture_file();
    let flake = forjar::cli::dist_generators_b::generate_nix(&minimal_dist(), &release).unwrap();
    for needle in ["TODO", "PLACEHOLDER", "000000", "\"VERSION\""] {
        assert!(
            !flake.contains(needle),
            "F-3608 falsified: nix flake contains '{needle}'"
        );
    }
    assert!(
        flake.contains(r#"version = "1.1.1";"#),
        "flake must pin the real version"
    );
    assert!(
        flake.contains("3d4c5b6a7988f1e23d4c5b6a7988f1e23d4c5b6a7988f1e23d4c5b6a7988f1e2"),
        "flake must embed the real darwin/aarch64 sha256"
    );
}

/// F-3608: Missing checksum is a hard error naming the asset — never a placeholder.
#[test]
fn f3608_missing_checksum_hard_errors() {
    let mut release = release_from_fixture_file();
    release
        .checksums
        .shift_remove("mytool-1.1.1-aarch64-apple-darwin.tar.gz");
    let err = forjar::cli::dist_generators_b::generate_nix(&minimal_dist(), &release)
        .expect_err("missing checksum must be a hard error");
    assert!(
        err.contains("mytool-1.1.1-aarch64-apple-darwin.tar.gz"),
        "error must name the missing asset: {err}"
    );
    assert!(
        err.contains("--checksums-file"),
        "error must suggest --checksums-file: {err}"
    );
}

/// F-3608: Without a pinned version, checksum-bearing generation hard-errors.
#[test]
fn f3608_no_version_is_hard_error() {
    let err = resolve_release(&minimal_dist(), None, None)
        .expect_err("missing --version must be a hard error");
    assert!(
        err.contains("--version"),
        "error must tell the user to pass --version: {err}"
    );
}

/// F-3610: Version pinning produces reproducible (byte-identical) output.
#[test]
fn f3610_pinned_version_output_is_reproducible() {
    let release_a = release_from_fixture_file();
    let release_b = release_from_fixture_file();
    let dist = minimal_dist();

    let formula_a = forjar::cli::dist_generators::generate_homebrew(&dist, &release_a).unwrap();
    let formula_b = forjar::cli::dist_generators::generate_homebrew(&dist, &release_b).unwrap();
    assert_eq!(
        formula_a, formula_b,
        "F-3610 falsified: same pinned version produced different formulas"
    );

    let flake_a = forjar::cli::dist_generators_b::generate_nix(&dist, &release_a).unwrap();
    let flake_b = forjar::cli::dist_generators_b::generate_nix(&dist, &release_b).unwrap();
    assert_eq!(
        flake_a, flake_b,
        "F-3610 falsified: same pinned version produced different flakes"
    );

    let installer_a = forjar::cli::dist_generators::generate_installer(&dist);
    let installer_b = forjar::cli::dist_generators::generate_installer(&dist);
    assert_eq!(
        installer_a, installer_b,
        "F-3610 falsified: installer generation is not deterministic"
    );
}
