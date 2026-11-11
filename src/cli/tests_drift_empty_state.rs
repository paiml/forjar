//! forjar#852: `drift --offline` over an EMPTY state dir (no lock) must still
//! compare a declared pin with the live binary, like `plan --offline`.

use super::drift::cmd_drift_offline;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn write_config(dir: &Path, bin_dir: &Path) -> std::path::PathBuf {
    let yaml = format!(
        r#"version: "1.0"
name: t
machines:
  box:
    hostname: box
    addr: 127.0.0.1
resources:
  ollama:
    type: github_release
    machine: box
    repo: ollama/ollama
    tag: v0.33.2
    asset_pattern: "*linux-amd64*"
    binary: ollama
    install_dir: {}
"#,
        bin_dir.display()
    );
    let p = dir.join("f.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn run(version: &str) -> Result<(), String> {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let b = bin.join("ollama");
    std::fs::write(&b, format!("#!/bin/sh\nprintf '%s\\n' 'ollama version is {version}'\n")).unwrap();
    std::fs::set_permissions(&b, std::fs::Permissions::from_mode(0o755)).unwrap();
    let cfg = write_config(tmp.path(), &bin);
    let state = tmp.path().join("empty-state");
    std::fs::create_dir_all(&state).unwrap();
    cmd_drift_offline(
        &cfg, &state, None, false, None, false, false, false, false, None, false, true, true,
    )
}

#[test]
fn f852_empty_state_dir_pin_mismatch_is_drift() {
    let e = run("0.34.2").expect_err("empty state dir must not report clean");
    assert!(!e.contains("parse error"), "{e}");
}

#[test]
fn f852_empty_state_dir_matching_pin_is_clean() {
    run("0.33.2").expect("matching pin is clean");
}

/// forjar#488 vs #852: a state dir that holds ANOTHER stack's lock is not an
/// empty one. A declared machine that was never applied there has "no state
/// yet", which is not an error, even when it declares nothing a lockless run
/// could ask the host about (a `file`).
#[test]
fn f852_state_dir_with_another_stacks_lock_is_not_lockless() {
    let tmp = tempfile::tempdir().unwrap();
    let state = tmp.path().join("state");
    let other = crate::core::state::new_lock("elsewhere", "elsewhere");
    crate::core::state::save_lock(&state, &other).unwrap();
    let cfg = tmp.path().join("f.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: t\nmachines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: box\n    path: /tmp/forjar-f852-never\n    content: x\n",
    )
    .unwrap();
    cmd_drift_offline(
        &cfg, &state, None, false, None, false, false, false, false, None, false, true, true,
    )
    .expect("a never-applied machine beside another stack's locks is not a drift failure");
}

#[test]
fn f852_holds_no_lock_tells_empty_from_populated() {
    use super::drift_state::holds_no_lock;
    let tmp = tempfile::tempdir().unwrap();
    assert!(holds_no_lock(&tmp.path().join("absent")), "absent");
    assert!(holds_no_lock(tmp.path()), "empty");
    std::fs::create_dir_all(tmp.path().join("m-no-lock")).unwrap();
    assert!(holds_no_lock(tmp.path()), "a machine dir without a lock");
    let lock = crate::core::state::new_lock("m1", "m1");
    crate::core::state::save_lock(tmp.path(), &lock).unwrap();
    assert!(!holds_no_lock(tmp.path()), "one lock anywhere");
}
