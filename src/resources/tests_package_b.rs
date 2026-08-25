#![allow(unused_imports)]
use super::package::*;
use super::package_check::check_script;
use super::tests_package::make_apt_resource;
use crate::core::types::{MachineTarget, Resource, ResourceType};

#[test]
fn test_fj006_state_query_cargo_output_format() {
    let mut r = make_apt_resource(&["pmat"]);
    r.provider = Some("cargo".to_string());
    let script = state_query_script(&r);
    // `pmat=installed` now carries the per-binary status suffix, so the token
    // is a prefix rather than a whole line. The MISSING arm is unchanged.
    assert!(script.contains("pmat=installed"));
    assert!(script.contains("echo 'pmat=MISSING'"));
}

/// paiml/infra#208 — the observable must EXECUTE differently when a binary is
/// gone. Every other assertion in this file reads the script's text; this one
/// runs it, because the defect being fixed is that the script's text looked
/// perfectly correct while reporting `installed` over an empty bin directory.
#[cfg(unix)]
#[test]
fn cargo_observable_reports_a_deleted_binary_as_gone() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().expect("tempdir");
    let bin = tmp.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();

    // A fake `cargo` whose `install --list` reports a crate with two binaries,
    // exactly as the real one formats it.
    let fake_cargo = bin.join("cargo");
    std::fs::write(
        &fake_cargo,
        "#!/bin/sh\nprintf 'demo-crate v1.0.0:\\n    demo-one\\n    demo-two\\n'\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake_cargo, std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut r = make_apt_resource(&["demo-crate"]);
    r.provider = Some("cargo".to_string());
    let script = state_query_script(&r);

    let run = |extra_bins: &[&str]| -> String {
        for b in extra_bins {
            let p = bin.join(b);
            std::fs::write(&p, "#!/bin/sh\nexit 0\n").unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // The fake bin dir FIRST so our `cargo` shadows the real one and the
        // demo binaries resolve — but the system dirs must stay, or `awk`,
        // `grep` and `sh` itself disappear and the test measures nothing.
        // (First cut set PATH to the fake dir alone and died on ENOENT for
        // `sh` — a test that cannot run is not a passing test.)
        let path = format!("{}:/usr/bin:/bin", bin.to_str().unwrap());
        let out = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(&script)
            .env("PATH", &path)
            .output()
            .expect("run observable");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    // Both binaries absent: registered, but nothing on disk.
    let gone = run(&[]);
    assert!(
        gone.contains("demo-one:GONE") && gone.contains("demo-two:GONE"),
        "a registered crate with NO binaries must not read as healthy, got: {gone}"
    );

    // Both present.
    let ok = run(&["demo-one", "demo-two"]);
    assert!(
        ok.contains("demo-one:ok") && ok.contains("demo-two:ok"),
        "a fully installed crate must read as ok, got: {ok}"
    );

    // THE POINT: the two states must be DISTINGUISHABLE. The old observable
    // emitted the identical string for both, which is why a fleet-wide binary
    // deletion produced zero drift findings.
    assert_ne!(
        gone, ok,
        "the observable produces the SAME output whether the binaries exist or \
         not — it cannot generate a drift signal"
    );
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

    // Every package is checked, and in the declared order — which is what
    // "preserves all" means. This used to assert a newline COUNT as a proxy
    // for "one line per package"; that broke when verdict.rs gave the
    // condition its own line so a folded YAML scalar could not collide with
    // `if`/`then`, even though every package was still checked in order.
    let positions: Vec<usize> = ["a", "b", "c"]
        .iter()
        .map(|p| {
            script
                .find(&format!("dpkg -l '{p}'"))
                .unwrap_or_else(|| panic!("package {p} is not checked at all:\n{script}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|w| w[0] < w[1]),
        "packages are checked out of declared order:\n{script}"
    );
}

/// BH-MUT: cargo install uses conditional check before installing.
#[test]
fn test_fj006_cargo_install_unconditional_force() {
    let mut r = make_apt_resource(&["tool"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    // --force makes install idempotent; FJ-51 adds --root for cache staging
    assert!(script.contains("cargo install --force --locked --root"));
    assert!(script.contains("'tool'"));
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
    assert!(script.contains("cargo install --force --locked --root"));
    assert!(script.contains("'batuta@0.3.0'"));
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
fn test_fj006_cargo_absent_uninstalls() {
    // WAS `test_fj006_cargo_absent_unsupported`, asserting
    // `script.contains("unsupported")` — "cargo absent should be unsupported".
    //
    // It encoded the defect as a requirement, so no correct fix could pass it.
    // apt, uv and brew all had an absent arm; cargo did not, so a declared
    // removal fell to the catch-all, echoed, exited 0, and reported CONVERGED.
    // (forjar#278.)
    let mut r = make_apt_resource(&["tool"]);
    r.provider = Some("cargo".to_string());
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("cargo uninstall 'tool'"),
        "a declared cargo removal must actually uninstall:\n{script}"
    );
    assert!(
        !script.contains("unsupported"),
        "cargo absent is supported now:\n{script}"
    );
}

/// An unsupported (provider, state) pair must REFUSE, not converge.
///
/// The catch-all was `echo 'unsupported: ...'`, which exits 0 — so forjar
/// reported the resource converged and silently ignored the declaration.
#[test]
fn an_unsupported_declaration_refuses_instead_of_converging() {
    let mut r = make_apt_resource(&["tool"]);
    r.provider = Some("nonesuch".to_string());
    r.state = Some("present".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("exit 1"),
        "an unsupported declaration must exit non-zero:\n{script}"
    );
    assert!(
        script.contains("provider=nonesuch"),
        "the refusal must name the pair it could not handle:\n{script}"
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
        script.contains("cargo install --force --locked --root"),
        "must still install (with --root for cache): {script}"
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
        script.contains("cargo install --force --locked --path '/build/apr-cli'"),
        "cargo source must use --path: {script}"
    );
    assert!(
        !script.contains("cargo install --force --locked 'apr-cli'"),
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
        script.contains("cargo install --force --locked --path '/build/apr-cli'"),
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
    // GH-257: the intent — `source:` must not change the crate's identity —
    // is kept; the mechanism is not. This asserted
    // `script.contains("command -v 'apr-cli'")`, which enshrined the defect:
    // `apr-cli` installs a binary called `apr`, so that lookup FAILS on a host
    // where the crate is installed and working. The check now consults cargo's
    // own record, which is keyed by crate name.
    let mut r = make_apt_resource(&["apr-cli"]);
    r.provider = Some("cargo".to_string());
    r.source = Some("/build/apr-cli".to_string());
    let script = check_script(&r);
    assert!(
        script.contains("'^apr-cli v'"),
        "check must identify the crate by name even with source: {script}"
    );
    assert!(
        !script.contains("command -v"),
        "PATH lookup cannot answer this"
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
        script.contains("cargo install --force --locked --root"),
        "cargo install with version must use --root for cache: {script}"
    );
    assert!(
        script.contains("'ripgrep@14.1.0'"),
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

/// Match arm: ("apt", "latest") — PMAT-161
///
/// Latest semantics: refresh package lists then run apt-get install,
/// which installs missing packages or upgrades to the newest available
/// (no-op if already current). Postcondition: dpkg -l shows ii.
#[test]
fn test_apply_script_arm_apt_latest() {
    let mut r = make_apt_resource(&["docker-ce", "docker-ce-cli"]);
    r.state = Some("latest".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("apt-get update"),
        "apt latest must refresh lists: {script}"
    );
    assert!(
        script.contains("apt-get install"),
        "apt latest must install/upgrade: {script}"
    );
    assert!(
        script.contains("docker-ce") && script.contains("docker-ce-cli"),
        "apt latest must reference all packages: {script}"
    );
    assert!(
        !script.contains("unsupported"),
        "apt latest must not fall through to unsupported arm: {script}"
    );
    assert!(
        script.contains("set -euo pipefail"),
        "apt latest must use safety flags: {script}"
    );
    assert!(
        script.contains("dpkg -l"),
        "apt latest must verify postcondition via dpkg: {script}"
    );
    // FJ-PMAT-161-1: tolerate apt-get update partial failures
    assert!(
        script.contains("apt-get update -qq || true"),
        "apt latest must tolerate apt-get update failures: {script}"
    );
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
