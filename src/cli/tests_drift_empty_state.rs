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
