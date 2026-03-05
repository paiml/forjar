//! Additional bundle tests — verify mode, source scanning, state inclusion.

use super::bundle::*;

fn make_config(dir: &std::path::Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

#[test]
fn bundle_verify_basic() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: verify-test\nmachines: {}\nresources: {}\n",
    );
    let result = cmd_bundle_verify(&file);
    assert!(result.is_ok());
}

#[test]
fn bundle_verify_with_store() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: verify-store\nmachines: {}\nresources: {}\n",
    );
    let store = dir.path().join("store");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("artifact.bin"), b"binary content").unwrap();
    let result = cmd_bundle_verify(&file);
    assert!(result.is_ok());
}

#[test]
fn bundle_verify_with_state() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: verify-state\nmachines: {}\nresources: {}\n",
    );
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("lock.yaml"), b"resources: {}").unwrap();
    let result = cmd_bundle_verify(&file);
    assert!(result.is_ok());
}

#[test]
fn bundle_with_source_files() {
    let dir = tempfile::tempdir().unwrap();
    // Create a source file referenced by a resource
    std::fs::write(dir.path().join("nginx.conf"), "server { }").unwrap();
    let file = make_config(
        dir.path(),
        r#"version: "1.0"
name: source-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/nginx.conf
    source: nginx.conf
"#,
    );
    let result = cmd_bundle(&file, None, false);
    assert!(result.is_ok());
}

#[test]
fn bundle_include_state_flag() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: state-flag\nmachines: {}\nresources: {}\n",
    );
    // Create state dir with some files
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("global.lock"), "global: true").unwrap();
    std::fs::write(state.join("machine.lock"), "resources: {}").unwrap();

    // Without include_state
    let result = cmd_bundle(&file, None, false);
    assert!(result.is_ok());

    // With include_state
    let result = cmd_bundle(&file, None, true);
    assert!(result.is_ok());
}

#[test]
fn bundle_with_output_path() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: output-path\nmachines: {}\nresources: {}\n",
    );
    let out = dir.path().join("output.tar.gz");
    let result = cmd_bundle(&file, Some(&out), false);
    assert!(result.is_ok());
}

#[test]
fn bundle_nonexistent_config() {
    let result = cmd_bundle(
        std::path::Path::new("/nonexistent/forjar.yaml"),
        None,
        false,
    );
    assert!(result.is_err());
}

#[test]
fn bundle_verify_nonexistent_config() {
    let result = cmd_bundle_verify(std::path::Path::new("/nonexistent/forjar.yaml"));
    assert!(result.is_err());
}
