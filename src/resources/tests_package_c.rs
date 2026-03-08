//! FJ-51: Cargo binary cache tests.

use super::package::*;
use super::tests_package::make_apt_resource;

/// Cache hit path: script checks FORJAR_CACHE_DIR before compiling.
#[test]
fn test_fj51_cargo_cache_hit_path() {
    let mut r = make_apt_resource(&["ripgrep"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("FORJAR_CACHE_DIR"),
        "must reference cache dir env var: {script}"
    );
    assert!(
        script.contains("cache-hit ripgrep"),
        "must echo cache-hit on hit: {script}"
    );
    assert!(
        script.contains("_CACHE_DIR"),
        "must use _CACHE_DIR variable: {script}"
    );
}

/// Cache miss path: installs to staging then populates cache.
#[test]
fn test_fj51_cargo_cache_miss_populates_cache() {
    let mut r = make_apt_resource(&["bat"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("mktemp -d /tmp/forjar-cargo"),
        "cache miss must create staging dir: {script}"
    );
    assert!(
        script.contains("cp -a \"$_STAGING/bin\" \"$_CACHE_DIR/\""),
        "cache miss must copy to cache: {script}"
    );
    assert!(
        script.contains("rm -rf \"$_STAGING\""),
        "must clean up staging dir: {script}"
    );
    assert!(
        script.contains("cached bat"),
        "must echo cached on miss: {script}"
    );
}

/// Cache key includes architecture for cross-machine safety.
#[test]
fn test_fj51_cargo_cache_key_includes_arch() {
    let mut r = make_apt_resource(&["fd-find"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("$(uname -m)"),
        "cache key must include arch: {script}"
    );
}

/// Cache key includes version when specified.
#[test]
fn test_fj51_cargo_cache_key_includes_version() {
    let mut r = make_apt_resource(&["ripgrep"]);
    r.provider = Some("cargo".to_string());
    r.version = Some("14.1.0".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("ripgrep-14.1.0-"),
        "cache key must include version: {script}"
    );
}

/// Cache key uses "latest" when no version specified.
#[test]
fn test_fj51_cargo_cache_key_latest_when_no_version() {
    let mut r = make_apt_resource(&["bat"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("bat-latest-"),
        "cache key must use 'latest' without version: {script}"
    );
}

/// FORJAR_NO_CARGO_CACHE disables caching.
#[test]
fn test_fj51_cargo_cache_disable_env() {
    let mut r = make_apt_resource(&["tool"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("FORJAR_NO_CARGO_CACHE"),
        "must check disable env var: {script}"
    );
}

/// Source (local path) installs skip caching entirely.
#[test]
fn test_fj51_cargo_source_skips_cache() {
    let mut r = make_apt_resource(&["apr-cli"]);
    r.provider = Some("cargo".to_string());
    r.source = Some("/build/apr-cli".to_string());
    let script = apply_script(&r);
    assert!(
        !script.contains("_CACHE_KEY"),
        "source installs must not use cache: {script}"
    );
    assert!(
        !script.contains("FORJAR_CACHE_DIR"),
        "source installs must not reference cache dir: {script}"
    );
    assert!(
        script.contains("cargo install --force --locked --path '/build/apr-cli'"),
        "source installs use --path: {script}"
    );
}

/// Cargo install uses --root for staging (cache integration).
#[test]
fn test_fj51_cargo_install_uses_root_staging() {
    let mut r = make_apt_resource(&["pmat"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("--root \"$_STAGING\""),
        "crate install must use --root for staging: {script}"
    );
}

/// Staging binaries are copied to $CARGO_HOME/bin.
#[test]
fn test_fj51_cargo_cache_copies_to_cargo_bin() {
    let mut r = make_apt_resource(&["forjar"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("_CARGO_BIN"),
        "must define _CARGO_BIN: {script}"
    );
    assert!(
        script.contains("CARGO_HOME"),
        "must respect CARGO_HOME: {script}"
    );
    assert!(
        script.contains("cp \"$_STAGING/bin/\"* \"$_CARGO_BIN/\""),
        "must copy staging binaries to cargo bin: {script}"
    );
}

/// Multi-package resource gets per-package cache keys.
#[test]
fn test_fj51_cargo_multi_package_separate_cache_keys() {
    let mut r = make_apt_resource(&["ripgrep", "fd-find", "bat"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("ripgrep-latest-"),
        "each package gets own cache key: {script}"
    );
    assert!(
        script.contains("fd-find-latest-"),
        "each package gets own cache key: {script}"
    );
    assert!(
        script.contains("bat-latest-"),
        "each package gets own cache key: {script}"
    );
}
