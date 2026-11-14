//! forjar#1048: the cargo cache key for an unpinned crate follows the
//! upstream version resolved at apply time, never the literal `latest`.
//!
//! Script-text assertions plus a real `sh` run of the resolve + key lines with
//! a fake `cargo` on PATH, so the key the script computes is what is asserted.

use super::package::*;
use super::tests_package::make_apt_resource;
use std::os::unix::fs::PermissionsExt;

fn cargo_script(version: Option<&str>) -> String {
    let mut r = make_apt_resource(&["ripgrep"]);
    r.provider = Some("cargo".to_string());
    r.version = version.map(str::to_string);
    apply_script(&r)
}

/// Run the resolve snippet and the `_CACHE_KEY=` line of `script` under `sh`
/// with `fake_cargo` as the body of a `cargo` executable; print key and flag.
fn run_key(script: &str, fake_cargo: &str) -> String {
    let start = script.find("_FJ_NC=").expect("resolve snippet present");
    let key_line = script
        .lines()
        .find(|l| l.starts_with("_CACHE_KEY="))
        .expect("key line");
    let end = script.find(key_line).unwrap() + key_line.len();
    let body = format!(
        "set -euo pipefail\n{}\necho \"$_CACHE_KEY|${{_FJ_NC:-}}\"\n",
        &script[start..end]
    );
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("cargo");
    std::fs::write(&bin, format!("#!/bin/sh\n{fake_cargo}\n")).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(&body)
        .env("PATH", format!("{}:/usr/bin:/bin", dir.path().display()))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn search_says(v: &str) -> String {
    format!("echo 'ripgrep-extra = \"9.9.9\"    # decoy'\necho 'ripgrep = \"{v}\"    # fast grep'")
}

#[test]
fn fj1048_pinned_key_unchanged() {
    let s = cargo_script(Some("14.1.0"));
    assert!(
        s.contains("_CACHE_KEY=\"ripgrep-14.1.0-$(uname -m)\"\n"),
        "{s}"
    );
    assert!(
        s.contains("[ -z \"${FORJAR_NO_CARGO_CACHE:-}\" ] && "),
        "{s}"
    );
    assert!(s.contains("--root \"$_STAGING\" 'ripgrep@14.1.0'"), "{s}");
    assert!(!s.contains("_FJ_") && !s.contains("cargo search"), "{s}");
}

#[test]
fn fj1048_unpinned_key_tracks_resolved_version() {
    let s = cargo_script(None);
    assert!(!s.contains("ripgrep-latest"), "{s}");
    let a = run_key(&s, &search_says("14.1.0"));
    let b = run_key(&s, &search_says("14.2.0"));
    assert!(a.starts_with("ripgrep-14.1.0-"), "{a}");
    assert!(b.starts_with("ripgrep-14.2.0-"), "{b}");
    assert_ne!(a, b);
    assert!(
        a.ends_with("|") && b.ends_with("|"),
        "cache stays on: {a} {b}"
    );
}

#[test]
fn fj1048_unresolved_builds_without_cache() {
    let s = cargo_script(None);
    for fake in ["exit 101", "true", "echo 'other = \"1.0.0\"'"] {
        let out = run_key(&s, fake);
        assert!(
            out.ends_with("|1"),
            "cache must be bypassed ({fake}): {out}"
        );
        assert!(!out.contains("latest"), "{out}");
    }
    assert!(s.contains("building without cache"), "{s}");
    assert!(s.contains("[ -z \"${_FJ_NC:-}\" ]"), "{s}");
}

#[test]
fn fj1048_unpinned_and_pinned_scripts_pass_purifier() {
    for v in [None, Some("14.1.0")] {
        let s = cargo_script(v);
        crate::core::purifier::validate_script(&crate::transport::strip_data_payloads(&s))
            .unwrap_or_else(|e| panic!("purifier rejected {v:?}: {e}\n{s}"));
    }
}
