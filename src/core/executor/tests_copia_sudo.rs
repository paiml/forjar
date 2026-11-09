//! Refs forjar#927: the copia delta-sync path (file sources > 1 MiB) must run
//! its signature and transfer scripts inside the resource's declared privilege
//! context, exactly as `apply_script` does. A `sudo: true` file resource whose
//! parent mkdir and temp write ran unprivileged failed "Permission denied".

use super::helpers::copia_apply_file;
use super::test_fixtures::local_machine;
use crate::core::types::{Resource, ResourceType};
use std::os::unix::fs::PermissionsExt;

/// A `sudo` shim on PATH that records each call, then runs the script as the
/// current user. Non-root only: as root the wrapper skips sudo by design.
#[test]
#[allow(clippy::disallowed_methods)] // PATH shim for sudo; test-only, same as tests_phase60
fn copia_scripts_run_through_sudo_when_resource_is_sudo() {
    let uid = std::process::Command::new("id").arg("-u").output().unwrap();
    if uid.stdout.starts_with(b"0") {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let log = dir.path().join("sudo.log");
    let shim = bin.join("sudo");
    std::fs::write(
        &shim,
        format!("#!/bin/sh\necho call >> '{}'\nexec \"$@\"\n", log.display()),
    )
    .unwrap();
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();

    let src = dir.path().join("src.bin");
    std::fs::write(&src, b"payload").unwrap();
    let dest = dir.path().join("out/new/x");

    let resource = Resource {
        resource_type: ResourceType::File,
        path: Some(dest.display().to_string()),
        source: Some(src.display().to_string()),
        sudo: true,
        ..Default::default()
    };
    let old = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{old}", bin.display()));
    let out = copia_apply_file(&local_machine(), &resource, None);
    std::env::set_var("PATH", old);

    assert!(out.unwrap().success());
    assert_eq!(std::fs::read(&dest).unwrap(), b"payload");
    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    assert_eq!(
        calls.lines().count(),
        2,
        "signature and transfer must each cross sudo: {calls:?}"
    );
}
