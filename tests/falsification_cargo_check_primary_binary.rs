//! forjar#662: the cargo package check must ask the crate's PRIMARY binary for its version, not every binary the crate
//! installed, and must never let a helper's side effect land in the caller's directory.
//!
//! Before: the check ran every binary in `cargo install --list` with `--version`. A helper that ignores its arguments does
//! its normal job (bashrs's `quality-dashboard` writes `docs/quality-dashboard.md`), and a helper that prints nothing
//! (exit 2) failed the check of a correctly installed crate. This test EXECUTES the script against a fake toolchain.

use forjar::core::types::{MachineTarget, Resource, ResourceType};

#[cfg(unix)]
fn write_exe(path: &std::path::Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Run the check for `democrate v1.0.0` (binaries: `democrate`, `democrate-helper`) from `cwd`; return its stdout.
#[cfg(unix)]
fn run_check(helper_body: &str, cwd: &std::path::Path, tmp: &std::path::Path) -> String {
    let bin = tmp.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    write_exe(
        &bin.join("cargo"),
        "#!/bin/sh\nprintf 'democrate v1.0.0:\\n    democrate\\n    democrate-helper\\n'\n",
    );
    write_exe(
        &bin.join("democrate"),
        "#!/bin/sh\necho 'democrate 1.0.0'\n",
    );
    write_exe(&bin.join("democrate-helper"), helper_body);
    let r = Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("local".to_string()),
        provider: Some("cargo".to_string()),
        packages: vec!["democrate".to_string()],
        version: Some("1.0.0".to_string()),
        ..Default::default()
    };
    let script = forjar::resources::package_check::check_script(&r);
    let path = format!("{}:/usr/bin:/bin", bin.to_str().unwrap());
    let out = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(&script)
        .current_dir(cwd)
        .env("PATH", &path)
        .env("CARGO_HOME", tmp)
        .output()
        .expect("run check");
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[cfg(unix)]
#[test]
fn a_helper_that_ignores_version_is_not_run_in_the_callers_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    // Does its normal job whatever argv says, then prints the pinned version, so only the SIDE EFFECT can fail this test.
    let out = run_check(
        "#!/bin/sh\necho wrote > report.md\necho 'democrate-helper 1.0.0'\n",
        cwd.path(),
        tmp.path(),
    );
    assert!(out.contains("installed:democrate"), "check output: {out}");
    assert!(
        !cwd.path().join("report.md").exists(),
        "the check ran a helper binary and it wrote into the caller's directory"
    );
}

#[cfg(unix)]
#[test]
fn a_helper_without_version_support_does_not_fail_an_installed_crate() {
    let tmp = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    // `--version` unsupported: exit 2, nothing on stdout.
    let out = run_check("#!/bin/sh\nexit 2\n", cwd.path(), tmp.path());
    assert!(
        out.contains("installed:democrate"),
        "a correctly installed crate was reported missing because of its helper: {out}"
    );
}
