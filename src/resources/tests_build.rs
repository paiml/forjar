//! FJ-33: Build resource handler tests.

use super::build::*;
use super::tests_package::make_apt_resource;
use crate::core::types::ResourceType;

fn make_build_resource() -> crate::core::types::Resource {
    let mut r = make_apt_resource(&[]);
    r.resource_type = ResourceType::Build;
    r.build_machine = Some("intel".to_string());
    r.command = Some("cargo build --release --target aarch64-unknown-linux-gnu".to_string());
    r.source = Some("/tmp/cross/release/apr".to_string());
    r.target = Some("/home/noah/.cargo/bin/apr".to_string());
    r.completion_check = Some("apr --version".to_string());
    r
}

#[test]
fn test_fj33_build_check_with_completion_check() {
    let r = make_build_resource();
    let script = check_script(&r);
    assert!(
        script.contains("apr --version"),
        "check must use completion_check: {script}"
    );
    assert!(
        script.contains("installed:build"),
        "check must report installed: {script}"
    );
    assert!(
        script.contains("missing:build"),
        "check must report missing: {script}"
    );
}

#[test]
fn test_fj33_build_check_without_completion_check() {
    let mut r = make_build_resource();
    r.completion_check = None;
    let script = check_script(&r);
    assert!(
        script.contains("test -x"),
        "check without completion_check must test executable: {script}"
    );
}

#[test]
fn test_fj33_build_apply_phases() {
    let r = make_build_resource();
    let script = apply_script(&r);
    assert!(script.contains("Phase 1"), "must have Phase 1: {script}");
    assert!(script.contains("Phase 2"), "must have Phase 2: {script}");
    assert!(script.contains("Phase 3"), "must have Phase 3: {script}");
}

#[test]
fn test_fj33_build_apply_ssh_to_build_machine() {
    let r = make_build_resource();
    let script = apply_script(&r);
    assert!(
        script.contains("ssh -o BatchMode=yes"),
        "must SSH to build machine: {script}"
    );
    assert!(
        script.contains("'intel'"),
        "must target build machine: {script}"
    );
}

#[test]
fn test_fj33_build_apply_scp_transfer() {
    let r = make_build_resource();
    let script = apply_script(&r);
    assert!(
        script.contains("scp -o BatchMode=yes"),
        "must SCP artifact: {script}"
    );
    assert!(
        script.contains("intel:/tmp/cross/release/apr"),
        "must transfer from build machine artifact path: {script}"
    );
    assert!(
        script.contains("/home/noah/.cargo/bin/apr"),
        "must deploy to target path: {script}"
    );
}

#[test]
fn test_fj33_build_apply_completion_check() {
    let r = make_build_resource();
    let script = apply_script(&r);
    assert!(
        script.contains("apr --version"),
        "must run completion check: {script}"
    );
}

#[test]
fn test_fj33_build_apply_working_dir() {
    let mut r = make_build_resource();
    r.working_dir = Some("~/src/aprender".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("cd '~/src/aprender'"),
        "must cd to working_dir on build machine: {script}"
    );
}

#[test]
fn test_fj33_build_apply_localhost_no_ssh() {
    let mut r = make_build_resource();
    r.build_machine = Some("localhost".to_string());
    let script = apply_script(&r);
    assert!(
        !script.contains("ssh -o BatchMode"),
        "localhost build must not SSH: {script}"
    );
    assert!(
        !script.contains("scp"),
        "localhost build must not SCP: {script}"
    );
    assert!(
        script.contains("cp "),
        "localhost build must use cp: {script}"
    );
}

#[test]
fn test_fj33_build_state_query() {
    let r = make_build_resource();
    let script = state_query_script(&r);
    assert!(
        script.contains("sha256sum"),
        "state query must hash artifact: {script}"
    );
    assert!(
        script.contains("/home/noah/.cargo/bin/apr"),
        "state query must reference deploy path: {script}"
    );
    assert!(
        script.contains("MISSING"),
        "state query must handle missing file: {script}"
    );
}

#[test]
fn test_fj33_build_apply_chmod_executable() {
    let r = make_build_resource();
    let script = apply_script(&r);
    assert!(
        script.contains("chmod +x"),
        "must make artifact executable: {script}"
    );
}

#[test]
fn test_fj33_build_apply_mkdir_parent() {
    let r = make_build_resource();
    let script = apply_script(&r);
    assert!(
        script.contains("mkdir -p"),
        "must create parent directories: {script}"
    );
}
