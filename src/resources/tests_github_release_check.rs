//! forjar#1049: the `github_release` check script asserts version and checksum.
//!
//! Every test EXECUTES the generated check with a fake installed binary, so the
//! verdict is the real exit status, not a substring.

use super::github_release::{apply_script, check_script};
use crate::core::purifier::validate_script;
use crate::core::types::{MachineTarget, Resource, ResourceType};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// sha256 of the fake binary written by `install` below with `--version` = 1.2.3.
const OTHER_SUM: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn resource(dir: &Path, tag: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::GithubRelease,
        machine: MachineTarget::Single("local".to_string()),
        repo: Some("ollama/ollama".to_string()),
        tag: tag.map(str::to_string),
        asset_pattern: Some("*linux-amd64*".to_string()),
        binary: Some("ollama".to_string()),
        install_dir: Some(dir.display().to_string()),
        ..Default::default()
    }
}

fn install(dir: &Path, version_line: &str) {
    let p = dir.join("ollama");
    std::fs::write(&p, format!("#!/bin/sh\necho '{version_line}'\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Run a check script: (exit success, stdout).
fn run(script: &str) -> (bool, String) {
    let out = std::process::Command::new("bash")
        .arg("-c")
        .arg(script)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

fn sha256_of(path: &Path) -> String {
    let out = std::process::Command::new("sha256sum")
        .stdin(std::fs::File::open(path).unwrap())
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)[..64].to_string()
}

#[test]
fn pinned_tag_differs_from_installed_version_needs_apply() {
    let d = tempfile::tempdir().unwrap();
    install(d.path(), "ollama version 0.33.2");
    let (ok, out) = run(&check_script(&resource(d.path(), Some("v0.34.2"))));
    assert!(!ok, "old version must not read as converged: {out}");
    assert!(out.contains("version-mismatch"), "{out}");
}

#[test]
fn pinned_tag_equal_to_installed_version_is_converged() {
    let d = tempfile::tempdir().unwrap();
    install(d.path(), "ollama version 0.34.2");
    let (ok, out) = run(&check_script(&resource(d.path(), Some("v0.34.2"))));
    assert!(ok, "{out}");
    assert!(out.contains("installed:ollama/ollama"), "{out}");
}

/// Review lane 1: a zero-padded (CalVer) pin parses to 2024.8.6; the live
/// `2024.08.06` must compare numerically, not as text.
#[test]
fn zero_padded_calver_pin_matches_the_same_live_version() {
    let d = tempfile::tempdir().unwrap();
    install(d.path(), "tool 2024.08.06");
    let (ok, out) = run(&check_script(&resource(d.path(), Some("v2024.08.06"))));
    assert!(ok, "{out}");
    let (ok, out) = run(&check_script(&resource(d.path(), Some("v2024.08.07"))));
    assert!(!ok, "{out}");
}

#[test]
fn pinned_tag_with_unreadable_live_version_needs_apply() {
    let d = tempfile::tempdir().unwrap();
    install(d.path(), "no version here");
    let (ok, out) = run(&check_script(&resource(d.path(), Some("v0.34.2"))));
    assert!(!ok, "{out}");
    assert!(out.contains("unreadable"), "{out}");
}

#[test]
fn wrong_sha256_is_not_converged_and_right_one_is() {
    let d = tempfile::tempdir().unwrap();
    install(d.path(), "ollama version 0.34.2");
    let mut r = resource(d.path(), Some("v0.34.2"));
    r.checksum = Some(OTHER_SUM.to_string());
    let (ok, out) = run(&check_script(&r));
    assert!(!ok, "{out}");
    assert!(out.contains("checksum-mismatch"), "{out}");
    r.checksum = Some(format!("sha256:{}", sha256_of(&d.path().join("ollama"))));
    let (ok, out) = run(&check_script(&r));
    assert!(ok, "{out}");
}

#[test]
fn checksum_is_asserted_even_for_an_unpinned_tag() {
    let d = tempfile::tempdir().unwrap();
    install(d.path(), "whatever");
    let mut r = resource(d.path(), Some("nightly"));
    r.checksum = Some(OTHER_SUM.to_string());
    assert!(!run(&check_script(&r)).0);
}

#[test]
fn malformed_checksum_diverges_and_never_reaches_the_script() {
    let d = tempfile::tempdir().unwrap();
    install(d.path(), "ollama version 0.34.2");
    let mut r = resource(d.path(), None);
    r.checksum = Some("abc'; rm -rf /; '".to_string());
    let script = check_script(&r);
    assert!(!script.contains("rm -rf"), "{script}");
    let (ok, out) = run(&script);
    assert!(!ok && out.contains("checksum-invalid"), "{out}");
}

/// Unpinned behaviour is unchanged: presence-only, whatever version is live.
#[test]
fn latest_nightly_and_no_tag_stay_presence_only() {
    for tag in [None, Some("latest"), Some("nightly")] {
        let d = tempfile::tempdir().unwrap();
        let r = resource(d.path(), tag);
        let (ok, out) = run(&check_script(&r));
        assert!(
            !ok && out.contains("missing:ollama/ollama"),
            "{tag:?}: {out}"
        );
        install(d.path(), "ollama version 9.9.9");
        let (ok, out) = run(&check_script(&r));
        assert!(ok, "{tag:?} must stay presence-only: {out}");
        assert!(!check_script(&r).contains("version-mismatch"), "{tag:?}");
    }
}

/// The invalid-checksum and invalid-repo branches emit scripts too.
#[test]
fn rejection_branch_scripts_pass_the_purifier() {
    let d = tempfile::tempdir().unwrap();
    let mut r = resource(d.path(), None);
    r.checksum = Some("not-hex".to_string());
    validate_script(&check_script(&r)).unwrap();
    let mut r = resource(d.path(), None);
    r.repo = Some("x/y$(reboot)".to_string());
    validate_script(&check_script(&r)).unwrap();
}

#[test]
fn every_generated_script_passes_the_purifier() {
    let d = tempfile::tempdir().unwrap();
    for tag in [None, Some("nightly"), Some("v0.34.2")] {
        for sum in [None, Some(OTHER_SUM.to_string())] {
            for state in [None, Some("absent".to_string())] {
                let mut r = resource(d.path(), tag);
                r.checksum = sum.clone();
                r.state = state;
                for (name, s) in [("check", check_script(&r)), ("apply", apply_script(&r))] {
                    validate_script(&s).unwrap_or_else(|e| panic!("{name} {tag:?}: {e}\n{s}"));
                }
            }
        }
    }
}

/// Known limit, same as the drift detector (`tripwire/drift/semver.rs`:
/// "Pre-release and build suffixes are not compared"): an `-rc1` binary reads
/// as converged against the GA tag, and a `-rc1` tag against the GA binary.
#[test]
fn pre_release_suffix_is_ignored_like_the_drift_detector() {
    let d = tempfile::tempdir().unwrap();
    install(d.path(), "ollama version 1.2.3-rc1");
    let (ok, out) = run(&check_script(&resource(d.path(), Some("v1.2.3"))));
    assert!(ok, "rc binary compares as its N.N.N: {out}");
    install(d.path(), "ollama version 1.2.3");
    let (ok, out) = run(&check_script(&resource(d.path(), Some("v1.2.3-rc1"))));
    assert!(ok, "rc tag compares as its N.N.N: {out}");
}

/// Review lane 2: sha256sum prints a leading backslash for a path containing a
/// backslash; the file is read from stdin so the hash line is never escaped.
#[test]
fn checksum_matches_when_the_install_path_contains_a_backslash() {
    let d = tempfile::tempdir().unwrap();
    let sub = d.path().join("a\\b");
    std::fs::create_dir(&sub).unwrap();
    install(&sub, "ollama version 0.34.2");
    let mut r = resource(&sub, Some("v0.34.2"));
    r.checksum = Some(sha256_of(&sub.join("ollama")));
    let (ok, out) = run(&check_script(&r));
    assert!(ok, "{out}");
}

/// Review lane 2: the prefix is stripped once and case-insensitively.
#[test]
fn checksum_prefix_is_stripped_once_and_case_insensitively() {
    let d = tempfile::tempdir().unwrap();
    install(d.path(), "ollama version 0.34.2");
    let sum = sha256_of(&d.path().join("ollama"));
    let mut r = resource(d.path(), None);
    r.checksum = Some(format!("SHA256:{sum}"));
    assert!(run(&check_script(&r)).0);
    r.checksum = Some(format!("sha256:sha256:{sum}"));
    let (ok, out) = run(&check_script(&r));
    assert!(!ok && out.contains("checksum-invalid"), "{out}");
}

/// Review lane 2: a non-ASCII byte next to the version reads as in `parse_semver`.
#[test]
fn version_next_to_a_non_ascii_byte_is_read_bytewise() {
    let d = tempfile::tempdir().unwrap();
    install(d.path(), "ollama \u{e9}0.34.2");
    let (ok, out) = run(&check_script(&resource(d.path(), Some("v0.34.2"))));
    assert!(ok, "{out}");
}

/// Review lane 3: a NUL byte is a token boundary, as in `parse_semver`; it must
/// not be deleted (that would glue `tool` to `0.34.2`).
#[test]
fn version_after_a_nul_byte_is_read_like_parse_semver() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("ollama");
    std::fs::write(&p, "#!/bin/sh\nprintf 'tool\\000 0.34.2\\n'\n").unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    let (ok, out) = run(&check_script(&resource(d.path(), Some("v0.34.2"))));
    assert!(ok, "{out}");
}
