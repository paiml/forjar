//! PMAT-081 + PMAT-082: `forjar dist` source validation and `--verify`
//! Tier 1 static verification, falsified at the binary boundary.
//!
//! F-3609 (spec 25): "`--verify` catches a broken installer —
//! deliberately break asset URL, verify `--verify` reports failure"
//! with a non-zero exit code and a useful message.
//!
//! Usage: cargo test --test falsification_dist_verify

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn write_config(dir: &Path, dist_block: &str) -> PathBuf {
    let config = dir.join("forjar.yaml");
    let yaml = format!(
        "version: \"1.0\"\n\
         name: t\n\
         machines:\n\
         \x20 local:\n\
         \x20   hostname: l\n\
         \x20   addr: localhost\n\
         \x20   user: root\n\
         resources: {{}}\n\
         dist:\n{dist_block}"
    );
    std::fs::write(&config, yaml).unwrap();
    config
}

fn run_dist(config: &Path, extra_args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
        .arg("dist")
        .arg("-f")
        .arg(config)
        .args(extra_args)
        .output()
        .expect("forjar binary must run")
}

fn valid_dist_block() -> &'static str {
    "  source: github_release\n\
     \x20 repo: acme/tool\n\
     \x20 binary: mytool\n\
     \x20 checksums: SHA256SUMS\n\
     \x20 targets:\n\
     \x20   - os: linux\n\
     \x20     arch: x86_64\n\
     \x20     asset: \"mytool-{version}-x86_64-unknown-linux-gnu.tar.gz\"\n\
     \x20     libc: gnu\n"
}

// ── PMAT-082 / F-3609 ───────────────────────────────────────────────

/// `--verify` alone (no artifact flags) passes on a valid config.
#[test]
fn verify_passes_on_valid_config() {
    let dir = tempfile::tempdir().unwrap();
    let config = write_config(dir.path(), valid_dist_block());
    let out = run_dist(&config, &["--verify"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "verify must pass: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("verify: PASS"), "got: {stdout}");
}

/// F-3609: asset template without {version} → broken pinned-install URL
/// → non-zero exit and a message naming the problem.
#[test]
fn f3609_verify_fails_on_asset_without_version() {
    let dir = tempfile::tempdir().unwrap();
    let block = valid_dist_block().replace("mytool-{version}-", "mytool-");
    let config = write_config(dir.path(), &block);
    let out = run_dist(&config, &["--verify"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "verify must fail on broken asset URL"
    );
    assert!(stderr.contains("verify: FAIL"), "got: {stderr}");
    assert!(
        stderr.contains("{version} placeholder"),
        "message must name the broken asset URL: {stderr}"
    );
}

/// F-3609: repo with a malformed scheme → download URL cannot match the
/// github.com release shape → non-zero exit.
#[test]
fn f3609_verify_fails_on_malformed_repo_scheme() {
    let dir = tempfile::tempdir().unwrap();
    let block = valid_dist_block().replace("acme/tool", "https://github.com/acme/tool");
    let config = write_config(dir.path(), &block);
    let out = run_dist(&config, &["--verify"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "verify must fail on malformed repo");
    assert!(
        stderr.contains("not a valid <org>/<repo> slug"),
        "got: {stderr}"
    );
}

/// `--verify` combined with artifact flags still verifies (temp dir, no
/// artifacts written to ./dist).
#[test]
fn verify_with_artifact_flags_does_not_write_dist_dir() {
    let dir = tempfile::tempdir().unwrap();
    let config = write_config(dir.path(), valid_dist_block());
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir.path())
        .args([
            "dist",
            "-f",
            "forjar.yaml",
            "--installer",
            "--rpm",
            "--verify",
        ])
        .output()
        .expect("forjar binary must run");
    assert!(out.status.success(), "config: {}", config.display());
    assert!(
        !dir.path().join("dist").exists(),
        "--verify must not write the dist/ output dir"
    );
}

// ── PMAT-081: dist.source validation ────────────────────────────────

/// Unsupported sources fail fast with the exact documented message.
#[test]
fn dist_source_local_url_s3_rejected() {
    for source in ["local", "url", "s3"] {
        let dir = tempfile::tempdir().unwrap();
        let block = valid_dist_block().replace("github_release", source);
        let config = write_config(dir.path(), &block);
        let out = run_dist(&config, &["--installer"]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "source {source} must be rejected");
        assert!(
            stderr.contains(&format!(
                "dist.source \"{source}\" is not yet supported (only github_release)"
            )),
            "got: {stderr}"
        );
    }
}

/// Source validation applies to --verify too.
#[test]
fn dist_source_validated_before_verify() {
    let dir = tempfile::tempdir().unwrap();
    let block = valid_dist_block().replace("github_release", "s3");
    let config = write_config(dir.path(), &block);
    let out = run_dist(&config, &["--verify"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("not yet supported"));
}

// ── PMAT-081: dist block in forjar schema (binary boundary) ─────────

#[test]
fn forjar_schema_output_contains_dist_block() {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .arg("schema")
        .output()
        .expect("forjar binary must run");
    assert!(out.status.success());
    let schema: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("schema output must be valid JSON");
    let dist = &schema["properties"]["dist"];
    assert!(
        dist.is_object(),
        "schema must contain a dist property block"
    );
    assert_eq!(
        dist["properties"]["source"]["enum"],
        serde_json::json!(["github_release"])
    );
}
