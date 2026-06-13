//! Tests: FJ-3607 Tier 2 container-installer-run orchestration.
//!
//! HERMETIC by design — these exercise target selection, harness
//! construction, result parsing, report aggregation, and docker-absent
//! degradation WITHOUT spawning containers. The one container-spawning
//! test is `#[ignore]`d (the proxied clean-room HANGS on docker/network,
//! the same constraint as PR #153's OCI tests); run it locally with
//! `cargo test -- --ignored`.

use super::dist_verify_tier2::*;
use crate::core::store::convergence_container::detect_container_runtime;
use crate::core::types::{DistBinaryTarget, DistConfig};

fn sample_dist() -> DistConfig {
    DistConfig {
        source: "github_release".into(),
        repo: "acme/tool".into(),
        binary: "mytool".into(),
        targets: vec![
            DistBinaryTarget {
                os: "linux".into(),
                arch: "x86_64".into(),
                asset: "mytool-{version}-x86_64-unknown-linux-gnu.tar.gz".into(),
                libc: Some("gnu".into()),
            },
            DistBinaryTarget {
                os: "linux".into(),
                arch: "x86_64".into(),
                asset: "mytool-{version}-x86_64-unknown-linux-musl.tar.gz".into(),
                libc: Some("musl".into()),
            },
        ],
        install_dir: "/usr/local/bin".into(),
        install_dir_fallback: "~/.local/bin".into(),
        checksums: Some("SHA256SUMS".into()),
        checksum_algo: "sha256".into(),
        description: "A test tool".into(),
        homepage: "https://example.com".into(),
        license: "MIT".into(),
        maintainer: "Test".into(),
        version_cmd: Some("mytool --version".into()),
        latest_tag: true,
        post_install: None,
        homebrew: None,
        nix: None,
    }
}

// ── target selection ──

#[test]
fn tier2_targets_cover_gnu_and_musl() {
    let libcs: Vec<&str> = TIER2_TARGETS.iter().map(|t| t.libc).collect();
    assert!(libcs.contains(&"gnu"));
    assert!(libcs.contains(&"musl"));
    assert_eq!(TIER2_TARGETS.len(), 2);
}

#[test]
fn ubuntu_selects_gnu_asset() {
    let dist = sample_dist();
    let ubuntu = &TIER2_TARGETS[0];
    assert_eq!(ubuntu.label, "ubuntu");
    let asset = select_asset_for_target(&dist, ubuntu).unwrap();
    assert!(asset.contains("gnu"), "got: {asset}");
}

#[test]
fn alpine_selects_musl_asset() {
    let dist = sample_dist();
    let alpine = &TIER2_TARGETS[1];
    assert_eq!(alpine.label, "alpine");
    let asset = select_asset_for_target(&dist, alpine).unwrap();
    assert!(asset.contains("musl"), "got: {asset}");
}

#[test]
fn missing_libc_asset_is_clear_error() {
    let mut dist = sample_dist();
    dist.targets.retain(|t| t.libc.as_deref() != Some("musl"));
    let err = select_asset_for_target(&dist, &TIER2_TARGETS[1]).unwrap_err();
    assert!(err.contains("musl"), "got: {err}");
    assert!(err.contains("alpine"), "got: {err}");
}

// ── harness construction ──

#[test]
fn strip_trailing_main_removes_lone_main() {
    assert_eq!(strip_trailing_main("set -eu\nmain\n"), "set -eu");
    assert_eq!(strip_trailing_main("set -eu\nmain"), "set -eu");
}

#[test]
fn strip_trailing_main_leaves_body_without_main() {
    assert_eq!(strip_trailing_main("echo hi\n"), "echo hi");
}

#[test]
fn harness_pins_tag_and_overrides_downloads() {
    let installer = crate::cli::dist_generators::generate_installer(&sample_dist());
    let h = build_install_harness(
        &installer,
        "/tmp/x.tar.gz",
        "deadbeef  x.tar.gz",
        "SHA256SUMS",
        "v1.2.3",
        "mytool --version",
    );
    // Pins the tag (no /releases/latest call).
    assert!(h.contains(r#"TAG="v1.2.3""#));
    assert!(h.contains("resolve_version()"));
    // Replaces the network fetch with a local copy.
    assert!(h.contains("download_file() {"));
    assert!(h.contains(r#"cp "/tmp/x.tar.gz""#));
    // Serves the staged checksums line so verify_checksum verifies.
    assert!(h.contains("deadbeef  x.tar.gz"));
    assert!(h.contains("*SHA256SUMS*|*.sha256)"));
    // Runs main and the post-install version check.
    assert!(h.contains("\nmain\n"));
    assert!(h.contains("mytool --version"));
    assert!(h.contains("TIER2_INSTALL_OK"));
    assert!(h.contains("TIER2_VERSION_OK"));
}

#[test]
fn harness_strips_original_main_call() {
    let installer = crate::cli::dist_generators::generate_installer(&sample_dist());
    let h = build_install_harness(
        &installer,
        "/tmp/x.tar.gz",
        "deadbeef  x.tar.gz",
        "SHA256SUMS",
        "v1",
        "mytool --version",
    );
    // Exactly one `main` invocation line (ours, after the overrides),
    // because the generator's trailing `main` was stripped.
    let main_calls = h.matches("\nmain\n").count();
    assert_eq!(main_calls, 1, "harness must call main exactly once");
}

// ── #165: download override matches the CONFIGURED checksums filename ──

/// Extract the line of the harness `download()` override whose `case` arm
/// serves the staged sums line, to assert on its glob pattern. The override
/// arm is uniquely `<glob>) printf '%s\n' '<sums_line>' ;;`.
fn sums_case_arm(harness: &str) -> &str {
    harness
        .lines()
        .find(|l| l.contains(r") printf '%s\n' '") && l.trim_end().ends_with(";;"))
        .expect("harness must have a sums-serving case arm")
}

/// Crude POSIX `case`-glob match for the patterns we emit (`*x*` / `*.sha256`),
/// enough to prove the configured-sums URL would hit the sums arm.
fn glob_matches(pattern: &str, url: &str) -> bool {
    pattern.split('|').any(|alt| {
        let alt = alt.trim();
        match (alt.starts_with('*'), alt.ends_with('*')) {
            (true, true) => url.contains(alt.trim_matches('*')),
            (true, false) => url.ends_with(alt.trim_start_matches('*')),
            (false, true) => url.starts_with(alt.trim_end_matches('*')),
            (false, false) => url == alt,
        }
    })
}

#[test]
fn harness_matches_custom_checksums_filename() {
    // #165: a custom dist.checksums (e.g. CHECKSUMS.txt) must be matched by the
    // download override so verify_checksum FETCHES + verifies rather than
    // falling through to the tarball arm (which makes the installer warn-skip).
    let mut dist = sample_dist();
    dist.checksums = Some("CHECKSUMS.txt".into());
    let installer = crate::cli::dist_generators::generate_installer(&dist);
    let h = build_install_harness(
        &installer,
        "/tmp/x.tar.gz",
        "deadbeef  x.tar.gz",
        "CHECKSUMS.txt",
        "v1.2.3",
        "mytool --version",
    );
    // The case arm must reference the custom basename, not a hardcoded glob.
    let arm = sums_case_arm(&h);
    assert!(
        arm.contains("CHECKSUMS.txt"),
        "sums case arm must match the configured filename, got: {arm}"
    );
    assert!(!arm.contains("SHA256SUMS"), "must not hardcode SHA256SUMS: {arm}");

    // The arm's glob must actually match the SUMS_URL the installer builds.
    let sums_url = "https://github.com/acme/tool/releases/download/v1.2.3/CHECKSUMS.txt";
    let pattern = arm.split(')').next().unwrap().trim();
    assert!(
        glob_matches(pattern, sums_url),
        "glob {pattern:?} must match the installer's SUMS_URL {sums_url:?} (else verify_checksum skips)"
    );
    // The tarball URL must NOT hit the sums arm (else we'd serve sums for the binary).
    let asset_url = "https://github.com/acme/tool/releases/download/v1.2.3/mytool-1.2.3-x86_64-unknown-linux-gnu.tar.gz";
    assert!(
        !glob_matches(pattern, asset_url),
        "the tarball URL must fall through to the *) tarball arm"
    );
}

#[test]
fn harness_default_sha256sums_still_matches() {
    // The default SHA256SUMS name must keep working (regression guard for #165).
    let dist = sample_dist(); // checksums = Some("SHA256SUMS")
    let installer = crate::cli::dist_generators::generate_installer(&dist);
    let h = build_install_harness(
        &installer,
        "/tmp/x.tar.gz",
        "deadbeef  x.tar.gz",
        "SHA256SUMS",
        "v1.2.3",
        "mytool --version",
    );
    let arm = sums_case_arm(&h);
    let pattern = arm.split(')').next().unwrap().trim();
    let sums_url = "https://github.com/acme/tool/releases/download/v1.2.3/SHA256SUMS";
    assert!(glob_matches(pattern, sums_url), "default SHA256SUMS must match");
    // The per-asset .sha256 fallback must also match.
    let fallback = "https://github.com/acme/tool/releases/download/v1.2.3/asset.tar.gz.sha256";
    assert!(glob_matches(pattern, fallback), "the .sha256 fallback must match");
}

// ── result parsing ──

#[test]
fn parse_both_markers_is_pass() {
    let out = "info: ...\nTIER2_INSTALL_OK\nmytool 1.0\nTIER2_VERSION_OK\n";
    assert_eq!(parse_harness_output(out), Tier2RunStatus::Passed);
    assert!(parse_harness_output(out).passed());
}

#[test]
fn parse_install_only_is_version_fail() {
    let out = "TIER2_INSTALL_OK\n";
    assert_eq!(
        parse_harness_output(out),
        Tier2RunStatus::InstalledButVersionFailed
    );
    assert!(!parse_harness_output(out).passed());
}

#[test]
fn parse_no_markers_is_install_fail() {
    assert_eq!(
        parse_harness_output("error: download failed\n"),
        Tier2RunStatus::InstallFailed
    );
}

#[test]
fn status_details_are_distinct() {
    let a = Tier2RunStatus::Passed.detail();
    let b = Tier2RunStatus::InstalledButVersionFailed.detail();
    let c = Tier2RunStatus::InstallFailed.detail();
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_ne!(a, c);
}

// ── docker run argv ──

#[test]
fn run_argv_is_ephemeral_named_sleeper() {
    let argv = build_run_argv("ubuntu:latest", "forjar-dist-t2-ubuntu-1");
    assert_eq!(argv[0], "run");
    assert!(argv.contains(&"-d"));
    assert!(argv.contains(&"--rm"));
    assert!(argv.contains(&"--name"));
    assert!(argv.contains(&"forjar-dist-t2-ubuntu-1"));
    assert!(argv.contains(&"ubuntu:latest"));
    assert!(argv.contains(&"sleep"));
}

// ── report aggregation ──

#[test]
fn report_all_passing_is_ok() {
    let outcomes = vec![
        Tier2Outcome {
            label: "ubuntu".into(),
            passed: true,
            detail: "ok".into(),
        },
        Tier2Outcome {
            label: "alpine".into(),
            passed: true,
            detail: "ok".into(),
        },
    ];
    assert!(report_outcomes(&outcomes).is_ok());
}

#[test]
fn report_any_failing_is_err_naming_the_target() {
    let outcomes = vec![
        Tier2Outcome {
            label: "ubuntu".into(),
            passed: true,
            detail: "ok".into(),
        },
        Tier2Outcome {
            label: "alpine".into(),
            passed: false,
            detail: "extraction failed".into(),
        },
    ];
    let err = report_outcomes(&outcomes).unwrap_err();
    assert!(err.contains("verify-containers: FAIL"));
    assert!(err.contains("alpine"));
    assert!(err.contains("extraction failed"));
}

// ── docker-absent degradation (hermetic: only asserts the runtime
//    detection + skip path when no runtime is present) ──

#[test]
fn run_verify_containers_skips_cleanly_without_runtime() {
    // When no container runtime is installed, Tier 2 must NOT fail — it
    // returns Ok with a skip message. When a runtime IS present (dev
    // box), this would try to spawn, so guard on detection.
    if detect_container_runtime().is_none() {
        let dist = sample_dist();
        let args = crate::cli::commands::DistArgs {
            file: "forjar.yaml".into(),
            installer: false,
            homebrew: false,
            binstall: false,
            nix: false,
            github_action: false,
            deb: false,
            rpm: false,
            all: false,
            verify: false,
            verify_containers: true,
            version: None,
            checksums_file: None,
            output: None,
            output_dir: None,
            json: false,
        };
        assert!(
            run_verify_containers(&dist, &args).is_ok(),
            "Tier 2 must degrade to a clean skip without Docker"
        );
    }
}

// ── container-spawning: ignored in CI (proxied clean-room HANGS on
//    docker/network); run locally with `cargo test -- --ignored`. ──

#[test]
#[ignore] // requires Docker or Podman; HANGS proxied clean-room CI
fn tier2_runs_installer_in_ubuntu_and_alpine() {
    let runtime = match detect_container_runtime() {
        Some(rt) => rt,
        None => return, // no runtime locally — nothing to assert
    };
    let dist = sample_dist();
    let outcomes = run_all_targets(&runtime, &dist);
    assert_eq!(outcomes.len(), 2);
    for o in &outcomes {
        assert!(o.passed, "{} failed: {}", o.label, o.detail);
    }
}
