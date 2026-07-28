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
    assert!(script.contains("'x'\\'';reboot;'\\'''"), "{script}");
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
    assert!(script.contains("'\\''"), "{script}");
    assert!(!script.contains("--force 'ruff';id"), "{script}");
}

/// brew install/list route the package through the escaper.
#[test]
fn fj154_brew_package_quote_neutralized() {
    let mut r = make_apt_resource(&["jq';id;'"]);
    r.provider = Some("brew".to_string());
    let script = apply_script(&r);
    assert!(script.contains("'\\''"), "{script}");
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
        script.contains("--path '/build/x'\\'';reboot;'\\'''"),
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
    assert!(script.contains("'\\''"), "{script}");
    assert!(!script.contains(" 'x';id"), "{script}");
}
