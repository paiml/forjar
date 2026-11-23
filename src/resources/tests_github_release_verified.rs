//! forjar#1049 (part 1): EXECUTES the generated check script against fake
//! binaries. A converged check exits 0; any divergence exits non-zero.

use super::github_release::check_script;
use crate::core::purifier::validate_script;
use crate::core::types::{MachineTarget, Resource, ResourceType};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Whether the check for `dir/tool` pinned at `tag` reads converged. The
/// script must pass the purifier first.
fn converged(dir: &Path, tag: Option<&str>) -> bool {
    let r = Resource {
        resource_type: ResourceType::GithubRelease,
        machine: MachineTarget::Single("local".to_string()),
        repo: Some("acme/tool".to_string()),
        tag: tag.map(str::to_string),
        binary: Some("tool".to_string()),
        install_dir: Some(dir.display().to_string()),
        ..Default::default()
    };
    let script = check_script(&r);
    validate_script(&script).expect("generated script passes the purifier");
    let st = std::process::Command::new("bash")
        .args(["-c", &script])
        .status();
    st.expect("bash runs").success()
}

fn exe(dir: &Path, body: &str) {
    let p = dir.join("tool");
    std::fs::write(&p, body).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn pinned_version_must_equal_the_live_version() {
    let d = tempfile::tempdir().unwrap();
    exe(d.path(), "#!/bin/sh\necho 'tool 0.34.2'\n");
    assert!(converged(d.path(), Some("v0.34.2")));
    assert!(!converged(d.path(), Some("v0.33.2")), "old pin converged");
    exe(d.path(), "#!/bin/sh\necho 'tool 2024.08.06'\n");
    assert!(
        converged(d.path(), Some("v2024.8.6")),
        "CalVer zero padding"
    );
}

#[test]
fn unrunnable_file_in_versioned_dir_is_not_converged() {
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("tool-0.34.2/bin");
    std::fs::create_dir_all(&bin).unwrap();
    exe(&bin, "\x7fELF-not-runnable");
    assert!(!converged(&bin, Some("v0.34.2")), "path read as version");
}

#[test]
fn failed_run_or_stderr_only_version_is_unknown() {
    let d = tempfile::tempdir().unwrap();
    exe(d.path(), "#!/bin/sh\necho 'tool 0.34.2'\nexit 3\n");
    assert!(!converged(d.path(), Some("v0.34.2")), "failed run was read");
    exe(d.path(), "#!/bin/sh\necho 'tool 0.34.2' >&2\n");
    assert!(!converged(d.path(), Some("v0.34.2")), "stderr was read");
}

#[test]
fn directory_is_not_converged() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("tool")).unwrap();
    assert!(!converged(d.path(), None));
}

#[test]
fn no_pin_keeps_presence_only_behaviour() {
    let d = tempfile::tempdir().unwrap();
    exe(d.path(), "#!/bin/sh\nexit 1\n");
    for tag in [None, Some("latest"), Some("nightly")] {
        assert!(converged(d.path(), tag), "{tag:?}");
    }
}

#[test]
fn prerelease_pin_with_final_live_is_not_converged() {
    let d = tempfile::tempdir().unwrap();
    exe(d.path(), "#!/bin/sh\necho 'tool 1.2.3'\n");
    assert!(!converged(d.path(), Some("v1.2.3-rc1")));
}

#[test]
fn final_pin_with_prerelease_live_is_not_converged() {
    let d = tempfile::tempdir().unwrap();
    exe(d.path(), "#!/bin/sh\necho 'tool 1.2.3-rc1'\n");
    assert!(!converged(d.path(), Some("v1.2.3")));
    exe(d.path(), "#!/bin/sh\necho 'tool v1.2.3-rc1 (abc)'\n");
    assert!(converged(d.path(), Some("v1.2.3-rc1")));
}

#[test]
fn dash_before_the_version_is_not_a_prerelease() {
    let d = tempfile::tempdir().unwrap();
    exe(d.path(), "#!/bin/sh\necho 'cli 1.2.3'\n");
    assert!(converged(d.path(), Some("cli2-v1.2.3")));
    exe(d.path(), "#!/bin/sh\necho 'cli 1.2.3-rc1'\n");
    assert!(!converged(d.path(), Some("cli2-v1.2.3")));
}

#[test]
fn target_suffix_or_trailing_dot_in_live_version_converges() {
    let d = tempfile::tempdir().unwrap();
    exe(d.path(), "#!/bin/sh\necho 'tool 1.2.3-linux-amd64'\n");
    assert!(converged(d.path(), Some("v1.2.3")));
    exe(d.path(), "#!/bin/sh\necho 'tool 1.2.3-rc1.'\n");
    assert!(converged(d.path(), Some("v1.2.3-rc1")));
}
