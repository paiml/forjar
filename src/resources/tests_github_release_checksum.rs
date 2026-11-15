//! forjar#1065: the `github_release` check asserts a declared `checksum:`
//! and verifies it BEFORE the binary runs. Every test executes the script.

use super::github_release::check_script;
use crate::core::purifier::validate_script;
use crate::core::types::{MachineTarget, Resource, ResourceType};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Install `dir/tool` (touches `dir/ran` when executed), run the check with
/// `checksum`, return (converged, the binary was executed, stdout).
fn check(dir: &Path, checksum: Option<String>) -> (bool, bool, String) {
    let (bin, mark) = (dir.join("tool"), dir.join("ran"));
    let body = format!(
        "#!/bin/sh\ntouch '{}'\necho 'tool 0.34.2'\n",
        mark.display()
    );
    std::fs::write(&bin, body).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let _ = std::fs::remove_file(&mark);
    let r = Resource {
        resource_type: ResourceType::GithubRelease,
        machine: MachineTarget::Single("local".to_string()),
        repo: Some("acme/tool".to_string()),
        tag: Some("nightly".to_string()),
        binary: Some("tool".to_string()),
        install_dir: Some(dir.display().to_string()),
        checksum,
        ..Default::default()
    };
    let script = check_script(&r);
    validate_script(&script).expect("generated script passes the purifier");
    let out = std::process::Command::new("bash")
        .args(["-c", &script])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    (out.status.success(), mark.exists(), stdout)
}

fn sha256_of(path: &Path) -> String {
    let f = std::fs::File::open(path).unwrap();
    let out = std::process::Command::new("sha256sum")
        .stdin(f)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)[..64].to_string()
}

#[test]
fn matching_checksum_converges_with_or_without_prefix_and_case() {
    let d = tempfile::tempdir().unwrap();
    check(d.path(), None);
    let sum = sha256_of(&d.path().join("tool"));
    for decl in [
        sum.clone(),
        format!("sha256:{sum}"),
        format!("SHA256:{}", sum.to_uppercase()),
    ] {
        let (ok, ran, out) = check(d.path(), Some(decl.clone()));
        assert!(ok && ran, "{decl}: {out}");
    }
}

#[test]
fn wrong_checksum_diverges_and_the_binary_is_not_run() {
    let d = tempfile::tempdir().unwrap();
    let (ok, ran, out) = check(d.path(), Some("0".repeat(64)));
    assert!(!ok && out.contains("checksum-mismatch"), "{out}");
    assert!(!ran, "binary ran before its checksum was verified");
}

#[test]
fn malformed_checksum_diverges_and_never_reaches_the_script() {
    let d = tempfile::tempdir().unwrap();
    let evil = "'; touch /tmp/fj-1065-pwned; '".to_string();
    let (ok, ran, out) = check(d.path(), Some(evil));
    assert!(!ok && !ran && out.contains("checksum-invalid"), "{out}");
    assert!(!Path::new("/tmp/fj-1065-pwned").exists());
}

#[test]
fn no_checksum_keeps_the_presence_only_check() {
    let d = tempfile::tempdir().unwrap();
    let (ok, ran, out) = check(d.path(), None);
    assert!(ok && ran, "{out}");
}
