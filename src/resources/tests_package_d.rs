//! FJ-154 / GH #154: shell-injection hardening tests for the package handler.
//!
//! These tests assert on the *generated script text* only — they never spawn
//! a shell or run a package manager.

use super::package::*;
use super::package_check::check_script;
use super::tests_package::make_apt_resource;

/// A package name with an embedded single quote must not break out of the
/// `'...'` wrapping; the quote is escaped to `'\''`.
#[test]
fn fj154_apt_package_quote_neutralized() {
    let r = make_apt_resource(&["x';reboot;'"]);
    let script = apply_script(&r);
    assert!(script.contains("'x'\"'\"';reboot;'\"'\"''"), "{script}");
    // No bare `reboot` left outside quotes in the install list.
    assert!(!script.contains(" 'x';reboot"), "{script}");
}

/// Benign apt package output is byte-identical to the pre-fix form (single
/// quoting an alphanumeric value is a no-op for the escaper).
#[test]
fn fj154_apt_benign_unchanged() {
    let r = make_apt_resource(&["curl"]);
    let script = apply_script(&r);
    assert!(script.contains("'curl'"));
    assert!(check_script(&r).contains("dpkg -l 'curl'"));
}

/// uv install routes the package through the escaper.
#[test]
fn fj154_uv_package_quote_neutralized() {
    let mut r = make_apt_resource(&["ruff';id;'"]);
    r.provider = Some("uv".to_string());
    let script = apply_script(&r);
    assert!(script.contains("'\"'\"'"), "{script}");
    assert!(!script.contains("--force 'ruff';id"), "{script}");
}

/// brew install/list route the package through the escaper.
#[test]
fn fj154_brew_package_quote_neutralized() {
    let mut r = make_apt_resource(&["jq';id;'"]);
    r.provider = Some("brew".to_string());
    let script = apply_script(&r);
    assert!(script.contains("'\"'\"'"), "{script}");
    assert!(!script.contains("brew install 'jq';id"), "{script}");
}

/// Cargo crate names that contain command-substitution are rejected with an
/// error script (they cannot be a real crate, and would otherwise be live in
/// the double-quoted cache key).
#[test]
fn fj154_cargo_command_substitution_rejected() {
    let mut r = make_apt_resource(&["x$(reboot)"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("ERROR: unsafe cargo package/version token"),
        "{script}"
    );
    assert!(script.contains("exit 1"), "{script}");
    // The dangerous token never reaches a live `_CACHE_KEY=` assignment.
    assert!(!script.contains("_CACHE_KEY=\"x$(reboot)"), "{script}");
}

/// Cargo version with shell metacharacters is also rejected.
#[test]
fn fj154_cargo_unsafe_version_rejected() {
    let mut r = make_apt_resource(&["ripgrep"]);
    r.provider = Some("cargo".to_string());
    r.version = Some("1.0;reboot".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("ERROR: unsafe cargo package/version token"),
        "{script}"
    );
}

/// A legitimate cargo crate still produces the normal cached-install script.
#[test]
fn fj154_cargo_legit_crate_unchanged() {
    let mut r = make_apt_resource(&["ripgrep"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(script.contains("cache-hit ripgrep"), "{script}");
    assert!(script.contains("$(uname -m)"), "{script}");
    assert!(!script.contains("ERROR: unsafe"), "{script}");
}

/// Cargo path installs (source set) skip the cache and quote the path arg.
#[test]
fn fj154_cargo_path_install_quoted() {
    let mut r = make_apt_resource(&["apr-cli"]);
    r.provider = Some("cargo".to_string());
    r.source = Some("/build/x';reboot;'".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("--path '/build/x'\"'\"';reboot;'\"'\"''"),
        "{script}"
    );
    assert!(!script.contains("--path '/build/x';reboot"), "{script}");
}

/// State query for apt routes both the query arg and the MISSING label
/// through the escaper.
#[test]
fn fj154_apt_state_query_quoted() {
    let r = make_apt_resource(&["x';id;'"]);
    let script = state_query_script(&r);
    assert!(script.contains("'\"'\"'"), "{script}");
    assert!(!script.contains(" 'x';id"), "{script}");
}

/// forjar#694: `completion_check` on a package resource is emitted into the
/// check script, for every provider that has presence assertions.
#[test]
fn gh694_completion_check_reaches_the_check_script() {
    for provider in ["apt", "cargo", "uv", "brew"] {
        let mut r = make_apt_resource(&["jre"]);
        r.provider = Some(provider.to_string());
        r.completion_check = Some("java -version 2>&1 | grep -q 17".to_string());
        let script = check_script(&r);
        assert!(
            script.contains("java -version 2>&1 | grep -q 17"),
            "{provider}: {script}"
        );
        assert!(script.contains("task=pending"), "{provider}: {script}");
        assert!(
            script.contains("missing:jre"),
            "{provider}: presence check dropped: {script}"
        );
    }
}

/// forjar#694: without a `completion_check` nothing is added.
#[test]
fn gh694_no_completion_check_adds_nothing() {
    for provider in ["apt", "cargo", "uv", "brew"] {
        let mut r = make_apt_resource(&["jre"]);
        r.provider = Some(provider.to_string());
        assert!(
            !check_script(&r).contains("task=pending"),
            "{provider} adds an assertion with no completion_check"
        );
    }
}

/// forjar#694: the verdict follows the declared check. dpkg is stubbed to say
/// the package is installed, so only `completion_check` can fail the script.
#[test]
fn gh694_failing_completion_check_diverges_a_present_package() {
    let dir = tempfile::tempdir().unwrap();
    let dpkg = dir.path().join("dpkg");
    std::fs::write(&dpkg, "#!/bin/sh\necho 'ii  jre'\n").unwrap();
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&dpkg)
        .status()
        .unwrap();
    let run = |check: &str| {
        let mut r = make_apt_resource(&["jre"]);
        r.completion_check = Some(check.to_string());
        std::process::Command::new("sh")
            .arg("-c")
            .arg(check_script(&r))
            .env("PATH", format!("{}:/usr/bin:/bin", dir.path().display()))
            .status()
            .unwrap()
            .code()
    };
    assert_eq!(run("true"), Some(0));
    assert_eq!(run("false"), Some(1));
}

/// #694: a blank `completion_check` asserts nothing. It must not reach the
/// script as an empty `if` condition, which is a shell syntax error (`if\n  \nthen`).
#[test]
fn gh694_blank_completion_check_adds_no_assertion() {
    for provider in ["apt", "cargo", "uv", "brew"] {
        let mut r = make_apt_resource(&["jre"]);
        r.provider = Some(provider.to_string());
        let without = check_script(&r);
        for blank in ["", "   ", "\n\t "] {
            r.completion_check = Some(blank.to_string());
            assert_eq!(check_script(&r), without, "{provider}: {blank:?}");
        }
    }
}
