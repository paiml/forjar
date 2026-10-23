//! forjar#613: a real `forjar apply` never installs a `github_release` pin
//! OLDER than the live binary.
//!
//! paiml/infra pinned ollama to `v0.33.2` on a box running `0.34.2`. The apply
//! downloaded whatever the pin named and installed it over the newer binary.
//!
//! These tests run the forjar binary against `localhost` with a fresh state
//! dir. The installed "ollama" is a script that prints a version. A fake `curl`
//! first on PATH records that it was reached and fails, so nothing touches the
//! network. Reaching curl means the download started: the guard let it through.
//!
//! Usage: cargo test --test falsification_github_release_never_downgrades

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn exe(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// Apply a `v0.33.2` pin over a live binary that prints `live`.
/// Returns (output, curl reached, the live binary's bytes after the apply).
fn apply_over(live: &str) -> (String, bool, String) {
    let dir = tempfile::tempdir().unwrap();
    let bin_dir = dir.path().join("bin");
    let fake = dir.path().join("fake");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::create_dir_all(&fake).unwrap();
    let binary = bin_dir.join("ollama");
    exe(&binary, &format!("echo 'ollama version is {live}'"));
    let before = fs::read_to_string(&binary).unwrap();
    let reached = dir.path().join("curl-reached");
    exe(
        &fake.join("curl"),
        &format!("touch '{}'; exit 22", reached.display()),
    );
    let cfg = dir.path().join("forjar.yaml");
    fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: never-downgrade
machines:
  localhost:
    hostname: localhost
    addr: localhost
resources:
  ollama-binary:
    type: github_release
    machine: localhost
    repo: ollama/ollama
    tag: v0.33.2
    asset_pattern: "*linux-amd64*"
    binary: ollama
    install_dir: {bin}
"#,
            bin = bin_dir.display()
        ),
    )
    .unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(["apply", "--yes", "-f"])
        .arg(&cfg)
        .arg("--state-dir")
        .arg(dir.path().join("state"))
        .env(
            "PATH",
            format!("{}:{}", fake.display(), std::env::var("PATH").unwrap()),
        )
        .output()
        .expect("forjar must run");
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    let after = fs::read_to_string(&binary).unwrap_or_default();
    assert_eq!(before, after, "the live binary was replaced:\n{s}");
    (s, reached.exists(), after)
}

#[test]
fn an_apply_refuses_a_pin_older_than_the_live_binary() {
    // THE DEFECT: without the guard the apply reaches curl for v0.33.2.
    let (out, reached, _) = apply_over("0.34.2");
    assert!(
        !reached,
        "the download of v0.33.2 started over a live 0.34.2:\n{out}"
    );
    assert!(out.contains("refusing to downgrade"), "{out}");
}

#[test]
fn an_apply_still_upgrades_an_older_live_binary() {
    // The control: the guard stands aside for an upgrade, so the apply
    // reaches the download (which the fake curl then fails).
    let (out, reached, _) = apply_over("0.33.1");
    assert!(
        reached,
        "an upgrade from 0.33.1 to v0.33.2 was refused:\n{out}"
    );
    assert!(!out.contains("refusing to downgrade"), "{out}");
}
