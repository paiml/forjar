//! forjar#613: a `github_release` apply never installs a pin OLDER than the
//! live binary.
//!
//! paiml/infra pinned ollama to `v0.33.2` on a box running `0.34.2`. The apply
//! script downloaded whatever the pin named and installed it over the newer
//! binary. These tests EXECUTE the generated script. A fake binary prints the
//! live version, and a fake `curl` fails, so no test touches the network. A
//! refusal is the guard; reaching curl means the guard let the install through.

use super::github_release::apply_script;
use crate::core::types::{MachineTarget, Resource, ResourceType};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn resource(install_dir: &Path, tag: &str) -> Resource {
    Resource {
        resource_type: ResourceType::GithubRelease,
        machine: MachineTarget::Single("local".to_string()),
        repo: Some("ollama/ollama".to_string()),
        tag: Some(tag.to_string()),
        asset_pattern: Some("*linux-amd64*".to_string()),
        binary: Some("ollama".to_string()),
        install_dir: Some(install_dir.display().to_string()),
        ..Default::default()
    }
}

fn exe(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Run the apply script with the live binary printing `version_output`.
/// Returns (stderr, exit success, whether curl was reached).
fn run(tag: &str, version_output: Option<&str>) -> (String, bool, bool) {
    let dir = tempfile::tempdir().unwrap();
    let bin_dir = dir.path().join("bin");
    let fake = dir.path().join("fake");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::create_dir_all(&fake).unwrap();
    if let Some(out) = version_output {
        exe(&bin_dir.join("ollama"), &format!("printf '%s\\n' '{out}'"));
    }
    let reached = dir.path().join("curl-reached");
    exe(
        &fake.join("curl"),
        &format!("touch '{}'; exit 22", reached.display()),
    );
    let script = apply_script(&resource(&bin_dir, tag));
    let out = std::process::Command::new("bash")
        .arg("-c")
        .arg(&script)
        .env(
            "PATH",
            format!("{}:{}", fake.display(), std::env::var("PATH").unwrap()),
        )
        .output()
        .expect("bash runs");
    (
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.success(),
        reached.exists(),
    )
}

#[test]
fn fj613_apply_refuses_a_pin_older_than_the_live_binary() {
    // THE DEFECT: without the guard this reaches curl and installs 0.33.2.
    let (err, ok, reached) = run("v0.33.2", Some("ollama version is 0.34.2"));
    assert!(!ok, "apply must fail rather than downgrade:\n{err}");
    assert!(
        !reached,
        "the download started: the guard let a downgrade through"
    );
    assert!(err.contains("refusing to downgrade"), "{err}");
    assert!(
        err.contains("0.34.2"),
        "the refusal must name the live version: {err}"
    );
}

#[test]
fn fj613_apply_refuses_across_major_and_minor_too() {
    for live in ["1.0.0", "0.34.0", "v0.33.3"] {
        let (err, _, reached) = run("v0.33.2", Some(live));
        assert!(!reached, "live {live} over pin 0.33.2 reached curl:\n{err}");
    }
}

#[test]
fn fj613_apply_proceeds_for_an_upgrade_an_equal_version_or_no_binary() {
    // The controls: the guard stands aside, so the install reaches curl.
    for live in [
        Some("ollama version is 0.33.1"),
        Some("0.33.2"),
        Some("0.9.99"),
        None,
    ] {
        let (err, _, reached) = run("v0.33.2", live);
        assert!(
            reached,
            "live {live:?} under pin 0.33.2 was refused:\n{err}"
        );
        assert!(!err.contains("refusing to downgrade"), "{err}");
    }
}

#[test]
fn fj613_apply_reads_the_live_version_the_way_drift_does() {
    // ollama's daemon-down shape carries the version after a warning line.
    let (_, _, reached) = run(
        "v0.33.2",
        Some("Warning: could not connect to a running Ollama instance\nWarning: client version is 0.34.2"),
    );
    assert!(
        !reached,
        "a warning line hid the live version from the guard"
    );
    // A sentence-final period: drift's parse_semver reads 0.34.2 here, so the
    // guard must too, or the two disagree and the downgrade goes through.
    let (_, _, reached) = run("v0.33.2", Some("client version is 0.34.2."));
    assert!(
        !reached,
        "a trailing period hid the live version from the guard"
    );
    // An IP is not a version, and neither is a four-part run; a broken binary
    // whose output has no version is repaired, not refused.
    for noise in ["listening on 10.42.0.11", "build 99.1.2.3", "segfault"] {
        let (err, _, reached) = run("v0.33.2", Some(noise));
        assert!(reached, "{noise:?} was read as a version:\n{err}");
    }
}

#[test]
fn fj613_unversioned_pins_are_not_ordered_against_the_box() {
    for tag in ["latest", "nightly"] {
        let (err, _, reached) = run(tag, Some("ollama version is 99.0.0"));
        assert!(reached, "tag {tag} was refused:\n{err}");
    }
}

#[test]
fn fj613_the_guarded_script_survives_forjars_own_bashrs_gate() {
    let dir = tempfile::tempdir().unwrap();
    let script = apply_script(&resource(dir.path(), "v0.33.2"));
    assert!(script.contains("refusing to downgrade"), "guard missing");
    crate::core::purifier::validate_script(&script)
        .unwrap_or_else(|e| panic!("bashrs refused the guarded apply script: {e}\n{script}"));
}
