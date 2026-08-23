//! PMAT-077: pins that keep the curl-install path working end-to-end.
//!
//! The v1.4.2 installer was broken three independent ways: mixed v-prefix
//! asset naming between the two release workflows, a missing SHA256SUMS
//! asset, and an extraction path that ignored the directory inside the
//! tarball. Each pin below guards one link of the chain:
//! committed install.sh == generator output == workflow asset naming.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

// ── the committed installer must RUN, not merely parse ──
//
// Every assertion above this line is text-vs-text. `sh -n` is no help
// either: calling `usage`/`die` before they are defined is valid POSIX
// syntax that fails only at runtime, so the published install.sh shipped
// `usage: not found` / exit 127 on `--help` while every gate said PASS.
// These two tests execute the real file.

/// Tools the installer could use to reach the network or write to the host.
/// Shimmed to refuse and placed first on PATH, so these tests can execute
/// install.sh without any chance of installing something.
const DENIED_TOOLS: [&str; 8] = [
    "curl", "wget", "sudo", "tar", "install", "cp", "chmod", "mktemp",
];

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("forjar-install-sh-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// A PATH whose first entry refuses every install-capable tool.
fn deny_path(dir: &Path) -> OsString {
    let deny = dir.join("deny-bin");
    fs::create_dir_all(&deny).expect("create deny-bin");
    for name in DENIED_TOOLS {
        let shim = deny.join(name);
        fs::write(
            &shim,
            format!("#!/bin/sh\necho \"denied: {name}\" >&2\nexit 97\n"),
        )
        .expect("write shim");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).expect("chmod shim");
        }
    }
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![deny];
    paths.extend(std::env::split_paths(&inherited));
    std::env::join_paths(paths).expect("join sandbox PATH")
}

/// Run the committed install.sh with the given args, sandboxed.
fn run_install_sh(tag: &str, args: &[&str]) -> (Option<i32>, String, String) {
    let script = fs::canonicalize("install.sh").expect("install.sh must exist");
    let dir = scratch_dir(tag);
    let path = deny_path(&dir);
    let out = Command::new("sh")
        .arg(&script)
        .args(args)
        .current_dir(&dir)
        .env("PATH", &path)
        .env("HOME", &dir)
        .stdin(Stdio::null())
        .output()
        .expect("spawn sh install.sh");
    let _ = fs::remove_dir_all(&dir);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// `sh install.sh --help` must exit 0 and print the usage block.
#[test]
fn install_sh_help_exits_zero_and_prints_usage() {
    let (code, stdout, stderr) = run_install_sh("help", &["--help"]);
    assert_eq!(
        code,
        Some(0),
        "`sh install.sh --help` exited {code:?}; stderr: {stderr}"
    );
    for needle in ["USAGE:", "OPTIONS:", "--help, -h"] {
        assert!(
            stdout.contains(needle),
            "--help printed no usage (missing {needle:?}); stdout: {stdout}"
        );
    }
}

/// An unknown flag must reach `die()` — the real message and exit 1, not a
/// 127 `die: not found`.
#[test]
fn install_sh_unknown_option_reaches_die() {
    let (code, _stdout, stderr) = run_install_sh("bogus", &["--not-a-real-flag"]);
    assert!(
        !stderr.contains("not found"),
        "error path hit an undefined function: {stderr}"
    );
    assert_eq!(
        code,
        Some(1),
        "unknown option should exit 1; stderr: {stderr}"
    );
    assert!(
        stderr.contains("unknown option: --not-a-real-flag"),
        "die() message missing; stderr: {stderr}"
    );
}
