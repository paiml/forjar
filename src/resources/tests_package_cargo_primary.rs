//! forjar#662: the cargo check asks the crate's PRIMARY binary for its version,
//! never a helper. A helper that ignores `--version` would otherwise do its
//! normal job (write files) and, lacking `--version`, fail a good install.

use super::package_check::check_script;
use super::tests_package::make_apt_resource;
use std::os::unix::fs::PermissionsExt;

fn script(dir: &std::path::Path, name: &str, body: &str) {
    let p = dir.join(name);
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Run the generated check against a fake CARGO_HOME recording `bins` under
/// `quietcrate v1.2.3`; returns (stdout, helper-ran).
fn run(bins: &[(&str, &str)]) -> (String, bool) {
    let home = tempfile::tempdir().unwrap();
    let bin = home.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let marker = home.path().join("helper-ran");
    let mut listing = String::from("quietcrate v1.2.3:\\n");
    for (n, body) in bins {
        listing.push_str(&format!("    {n}\\n"));
        script(
            &bin,
            n,
            &body.replace("MARK", &marker.display().to_string()),
        );
    }
    script(&bin, "cargo", &format!("printf '{listing}'"));
    let mut r = make_apt_resource(&["quietcrate"]);
    r.provider = Some("cargo".to_string());
    r.version = Some("1.2.3".to_string());
    let out = std::process::Command::new("bash")
        .arg("-c")
        .arg(check_script(&r))
        .env("CARGO_HOME", home.path())
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .output()
        .unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        marker.exists(),
    )
}

#[test]
fn a_helper_without_version_does_not_fail_a_good_install_and_never_runs() {
    let (out, ran) = run(&[
        ("quietcrate", "echo quietcrate 1.2.3"),
        ("quietcrate-helper", "touch MARK; exit 2"),
    ]);
    assert!(out.contains("installed:quietcrate"), "{out}");
    assert!(!ran, "the helper binary was executed");
}

#[test]
fn without_a_same_named_binary_only_the_first_listed_one_is_asked() {
    let (out, ran) = run(&[
        ("qc-main", "echo qc 1.2.3"),
        ("qc-helper", "touch MARK; exit 2"),
    ]);
    assert!(out.contains("installed:quietcrate"), "{out}");
    assert!(!ran);
}

#[test]
fn a_wrong_version_from_the_primary_is_still_missing() {
    let (out, _) = run(&[("quietcrate", "echo quietcrate 9.9.9")]);
    assert!(out.contains("missing:quietcrate"), "{out}");
}
