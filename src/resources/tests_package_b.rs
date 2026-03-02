#![allow(unused_imports)]
use super::package::*;
use super::tests_package::make_apt_resource;
use crate::core::types::{MachineTarget, Resource, ResourceType};

#[test]
fn test_fj006_state_query_cargo_output_format() {
    let mut r = make_apt_resource(&["pmat"]);
    r.provider = Some("cargo".to_string());
    let script = state_query_script(&r);
    assert!(script.contains("echo 'pmat=installed'"));
    assert!(script.contains("echo 'pmat=MISSING'"));
}

/// BH-MUT-0003: Kill mutation of apt state_query_script format.
#[test]
fn test_fj006_state_query_apt_output_format() {
    let r = make_apt_resource(&["vim"]);
    let script = state_query_script(&r);
    assert!(script.contains("vim"));
    assert!(script.contains("vim=MISSING"));
    assert!(script.contains("dpkg-query -W"));
}

/// BH-MUT: Multi-package list preserves order.
#[test]
fn test_fj006_multi_package_check_preserves_all() {
    let r = make_apt_resource(&["a", "b", "c"]);
    let script = check_script(&r);
    // All packages present in output
    assert!(script.contains("dpkg -l 'a'"));
    assert!(script.contains("dpkg -l 'b'"));
    assert!(script.contains("dpkg -l 'c'"));
    // Verify newline separation (multi-line script)
    assert_eq!(script.matches('\n').count(), 2);
}

/// BH-MUT: cargo install uses conditional check before installing.
#[test]
fn test_fj006_cargo_install_unconditional_force() {
    let mut r = make_apt_resource(&["tool"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    // --force makes install idempotent — no conditional check needed
    assert!(script.contains("cargo install --force 'tool'"));
    assert!(
        !script.contains("if !"),
        "should not have conditional check with --force"
    );
}

#[test]
fn test_fj006_apt_version_constraint() {
    let mut r = make_apt_resource(&["nginx"]);
    r.version = Some("1.18.0-0ubuntu1".to_string());
    let script = apply_script(&r);
    assert!(script.contains("'nginx=1.18.0-0ubuntu1'"));
    // Check commands still use unversioned name
    assert!(script.contains("dpkg -l \"$pkg\""));
}

#[test]
fn test_fj006_cargo_version_constraint() {
    let mut r = make_apt_resource(&["batuta"]);
    r.provider = Some("cargo".to_string());
    r.version = Some("0.3.0".to_string());
    let script = apply_script(&r);
    assert!(script.contains("cargo install --force 'batuta@0.3.0'"));
}

#[test]
fn test_fj006_uv_version_constraint() {
    let mut r = make_apt_resource(&["ruff"]);
    r.provider = Some("uv".to_string());
    r.version = Some("0.4.0".to_string());
    let script = apply_script(&r);
    assert!(script.contains("uv tool install --force 'ruff==0.4.0'"));
}

#[test]
fn test_fj006_no_version_unchanged() {
    // Without version, scripts should be the same as before
    let r = make_apt_resource(&["curl"]);
    let script = apply_script(&r);
    assert!(script.contains("'curl'"));
    assert!(!script.contains("curl="));
}

#[test]
fn test_fj006_default_provider_is_apt() {
    let mut r = make_apt_resource(&["curl"]);
    r.provider = None; // Default
    let script = apply_script(&r);
    assert!(
        script.contains("apt-get install"),
        "default provider should be apt"
    );
}

#[test]
fn test_fj006_default_state_is_present() {
    let mut r = make_apt_resource(&["curl"]);
    r.state = None; // Default
    let script = apply_script(&r);
    assert!(
        script.contains("apt-get install"),
        "default state should be present (install)"
    );
    assert!(!script.contains("apt-get remove"));
}

#[test]
fn test_fj006_apt_idempotent_check() {
    // apt apply has pre-check: only runs install if needed
    let r = make_apt_resource(&["curl"]);
    let script = apply_script(&r);
    assert!(
        script.contains("NEED_INSTALL=0"),
        "must have idempotent check"
    );
    assert!(
        script.contains("NEED_INSTALL=1"),
        "must set flag when package missing"
    );
}

#[test]
fn test_fj006_apt_postcondition_verify() {
    // apt apply verifies all packages installed after install
    let r = make_apt_resource(&["curl", "wget"]);
    let script = apply_script(&r);
    // Postcondition check at end
    let last_dpkg = script.rfind("dpkg -l").unwrap();
    let install = script.find("apt-get install").unwrap();
    assert!(
        last_dpkg > install,
        "postcondition check must come after install"
    );
}

#[test]
fn test_fj006_uv_absent_tolerant() {
    // uv uninstall uses `|| true` to tolerate already-absent packages
    let mut r = make_apt_resource(&["ruff"]);
    r.provider = Some("uv".to_string());
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("|| true"),
        "uv uninstall should tolerate already-absent"
    );
}

#[test]
fn test_fj006_cargo_absent_unsupported() {
    // cargo provider doesn't support absent state
    let mut r = make_apt_resource(&["tool"]);
    r.provider = Some("cargo".to_string());
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("unsupported"),
        "cargo absent should be unsupported"
    );
}

/// FJ-1005: cargo provider bootstraps rustup if cargo is missing and sets PATH.
#[test]
fn test_fj1005_cargo_bootstrap_rustup() {
    let mut r = make_apt_resource(&["realizar"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("command -v cargo"),
        "must check if cargo exists: {script}"
    );
    assert!(
        script.contains("rustup.rs"),
        "must bootstrap via rustup: {script}"
    );
    assert!(
        script.contains("cargo install --force 'realizar'"),
        "must still install: {script}"
    );
    assert!(
        script.contains(".cargo/bin:$PATH"),
        "must add cargo to PATH: {script}"
    );
}

/// PMAT-043: rustup bootstrap must NOT pipe curl to sh (SEC008/SEC015 violation).
/// bashrs I8 validation rejects `curl | sh` patterns — download to tmpfile then execute.
#[test]
fn test_pmat043_rustup_no_curl_pipe_to_sh() {
    let mut r = make_apt_resource(&["realizar"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        !script.contains("| sh"),
        "must not pipe curl to sh (SEC008): {script}"
    );
    assert!(
        !script.contains("| bash"),
        "must not pipe curl to bash: {script}"
    );
    // Should download to a file first, then execute
    assert!(
        script.contains("rustup-init"),
        "should download rustup-init to a file: {script}"
    );
}

/// FJ-1008: cargo install limits build parallelism to avoid OOM on high-core machines.
/// Root cause: unbounded `cargo install` defaults to nproc jobs, causing OOM on 32-core+.
#[test]
fn test_fj1008_cargo_install_limits_parallelism() {
    let mut r = make_apt_resource(&["realizar"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("CARGO_BUILD_JOBS"),
        "cargo install must set CARGO_BUILD_JOBS to limit parallelism: {script}"
    );
    assert!(
        script.contains("nproc"),
        "must derive job limit from nproc: {script}"
    );
}

// --- cargo source (--path) tests ---

/// cargo install from local source uses --path instead of crate name.
#[test]
fn test_fj_cargo_install_from_source() {
    let mut r = make_apt_resource(&["apr-cli"]);
    r.provider = Some("cargo".to_string());
    r.source = Some("/build/apr-cli".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("cargo install --force --path '/build/apr-cli'"),
        "cargo source must use --path: {script}"
    );
    assert!(
        !script.contains("cargo install --force 'apr-cli'"),
        "cargo source must NOT use crate name for install: {script}"
    );
}

/// When source is set, version is ignored (version comes from Cargo.toml at path).
#[test]
fn test_fj_cargo_source_ignores_version() {
    let mut r = make_apt_resource(&["apr-cli"]);
    r.provider = Some("cargo".to_string());
    r.source = Some("/build/apr-cli".to_string());
    r.version = Some("0.1.0".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("cargo install --force --path '/build/apr-cli'"),
        "cargo source+version must still use --path: {script}"
    );
    assert!(
        !script.contains("@0.1.0"),
        "cargo source must ignore version: {script}"
    );
}

/// check_script still uses package name (binary name) even with source set.
#[test]
fn test_fj_cargo_source_check_uses_binary_name() {
    let mut r = make_apt_resource(&["apr-cli"]);
    r.provider = Some("cargo".to_string());
    r.source = Some("/build/apr-cli".to_string());
    let script = check_script(&r);
    assert!(
        script.contains("command -v 'apr-cli'"),
        "check_script must use package name even with source: {script}"
    );
}

// --- FJ-036: Additional package tests ---

#[test]
fn test_fj036_package_cargo_install_with_version() {
    let mut r = make_apt_resource(&["ripgrep"]);
    r.provider = Some("cargo".to_string());
    r.version = Some("14.1.0".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("cargo install --force 'ripgrep@14.1.0'"),
        "cargo install with version must use @version syntax: {script}"
    );
}

#[test]
fn test_fj036_package_uv_install() {
    let mut r = make_apt_resource(&["ruff", "black"]);
    r.provider = Some("uv".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("uv tool install --force 'ruff'"),
        "uv provider must generate uv tool install: {script}"
    );
    assert!(
        script.contains("uv tool install --force 'black'"),
        "uv provider must install all packages: {script}"
    );
    assert!(
        script.contains("set -euo pipefail"),
        "uv install must start with safety flags: {script}"
    );
}

// ── Explicit per-arm match variant tests (apply_script) ───────

/// Match arm: ("apt", "present")
#[test]
fn test_apply_script_arm_apt_present() {
    let r = make_apt_resource(&["curl"]);
    let script = apply_script(&r);
    assert!(script.contains("apt-get install"), "apt present: {script}");
}

/// Match arm: ("apt", "absent")
#[test]
fn test_apply_script_arm_apt_absent() {
    let mut r = make_apt_resource(&["curl"]);
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("apt-get remove"), "apt absent: {script}");
}

/// Match arm: ("cargo", "present")
#[test]
fn test_apply_script_arm_cargo_present() {
    let mut r = make_apt_resource(&["tool"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(script.contains("cargo install"), "cargo present: {script}");
}

/// Match arm: ("uv", "present")
#[test]
fn test_apply_script_arm_uv_present() {
    let mut r = make_apt_resource(&["ruff"]);
    r.provider = Some("uv".to_string());
    let script = apply_script(&r);
    assert!(script.contains("uv tool install"), "uv present: {script}");
}

/// Match arm: ("uv", "absent")
#[test]
fn test_apply_script_arm_uv_absent() {
    let mut r = make_apt_resource(&["ruff"]);
    r.provider = Some("uv".to_string());
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("uv tool uninstall"), "uv absent: {script}");
}

/// Match arm: (other_provider, other_state)
#[test]
fn test_apply_script_arm_other_provider_other_state() {
    let mut r = make_apt_resource(&["foo"]);
    r.provider = Some("pip".to_string());
    r.state = Some("present".to_string());
    let script = apply_script(&r);
    assert!(script.contains("unsupported"), "other arm: {script}");
}
