use super::*;

#[test]
fn list_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    assert!(cmd_plugin_list(dir.path(), false).is_ok());
}
#[test]
fn list_empty_dir_json() {
    let dir = tempfile::tempdir().unwrap();
    assert!(cmd_plugin_list(dir.path(), true).is_ok());
}
#[test]
fn verify_missing_manifest() {
    assert!(cmd_plugin_verify(Path::new("/nonexistent/plugin"), false).is_err());
}
#[test]
fn list_nonexistent_dir() {
    assert!(cmd_plugin_list(Path::new("/nonexistent/plugins"), false).is_ok());
}
#[test]
fn dispatch_list() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = PluginCmd::List {
        plugin_dir: dir.path().to_path_buf(),
        json: false,
    };
    assert!(dispatch_plugin(cmd).is_ok());
}
#[test]
fn init_creates_scaffold() {
    let dir = tempfile::tempdir().unwrap();
    assert!(cmd_plugin_init("my-plugin", Some(dir.path()), false).is_ok());
    assert!(dir.path().join("my-plugin/plugin.yaml").exists());
    assert!(dir.path().join("my-plugin/plugin.wasm").exists());
    let m = std::fs::read_to_string(dir.path().join("my-plugin/plugin.yaml")).unwrap();
    assert!(m.contains("name: my-plugin"));
    assert!(m.contains("blake3:"));
}
#[test]
fn init_json_output() {
    let dir = tempfile::tempdir().unwrap();
    assert!(cmd_plugin_init("test-plugin", Some(dir.path()), true).is_ok());
}
#[test]
fn init_already_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("existing")).unwrap();
    let r = cmd_plugin_init("existing", Some(dir.path()), false);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("already exists"));
}
#[test]
fn init_verify_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    cmd_plugin_init("roundtrip", Some(dir.path()), false).unwrap();
    let v = resolve_and_verify(dir.path(), "roundtrip");
    assert!(v.is_ok(), "verify failed: {:?}", v.err());
    assert_eq!(v.unwrap().status, PluginStatus::Converged);
}
#[test]
fn dispatch_init() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = PluginCmd::Init {
        name: "d-test".into(),
        output: Some(dir.path().into()),
        json: false,
    };
    assert!(dispatch_plugin(cmd).is_ok());
}
#[test]
fn install_from_local_dir() {
    let src = tempfile::tempdir().unwrap();
    cmd_plugin_init("src-plug", Some(src.path()), false).unwrap();
    let dest = tempfile::tempdir().unwrap();
    let r = cmd_plugin_install(
        src.path().join("src-plug").to_str().unwrap(),
        dest.path(),
        false,
    );
    assert!(r.is_ok(), "install failed: {:?}", r.err());
    assert!(dest.path().join("src-plug/plugin.yaml").exists());
}
#[test]
fn install_already_exists() {
    let src = tempfile::tempdir().unwrap();
    cmd_plugin_init("dup", Some(src.path()), false).unwrap();
    let dest = tempfile::tempdir().unwrap();
    cmd_plugin_install(src.path().join("dup").to_str().unwrap(), dest.path(), false).unwrap();
    let r = cmd_plugin_install(src.path().join("dup").to_str().unwrap(), dest.path(), false);
    assert!(r.is_err());
}
#[test]
fn install_missing_source() {
    let dest = tempfile::tempdir().unwrap();
    assert!(cmd_plugin_install("/nonexistent/path", dest.path(), false).is_err());
}
#[test]
fn install_no_manifest() {
    let src = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let r = cmd_plugin_install(src.path().to_str().unwrap(), dest.path(), false);
    assert!(r.unwrap_err().contains("no plugin.yaml"));
}
#[test]
fn remove_plugin() {
    let dir = tempfile::tempdir().unwrap();
    cmd_plugin_init("removable", Some(dir.path()), false).unwrap();
    cmd_plugin_remove("removable", dir.path(), true, false).unwrap();
    assert!(!dir.path().join("removable").exists());
}
#[test]
fn remove_without_yes() {
    let dir = tempfile::tempdir().unwrap();
    cmd_plugin_init("keep", Some(dir.path()), false).unwrap();
    cmd_plugin_remove("keep", dir.path(), false, false).unwrap();
    assert!(dir.path().join("keep").exists());
}
#[test]
fn remove_nonexistent() {
    let dir = tempfile::tempdir().unwrap();
    assert!(cmd_plugin_remove("nope", dir.path(), true, false).is_err());
}
#[test]
fn remove_no_manifest() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("bare")).unwrap();
    let r = cmd_plugin_remove("bare", dir.path(), true, false);
    assert!(r.unwrap_err().contains("plugin.yaml"));
}
#[test]
fn build_no_cargo_toml() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_plugin_build(dir.path(), None, false);
    assert!(r.unwrap_err().contains("Cargo.toml"));
}
#[test]
fn build_invalid_cargo_toml() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[[[bad").unwrap();
    assert!(cmd_plugin_build(dir.path(), None, false)
        .unwrap_err()
        .contains("parse"));
}
#[test]
fn build_no_package_name() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert!(cmd_plugin_build(dir.path(), None, false)
        .unwrap_err()
        .contains("[package] name"));
}
#[test]
fn run_check_with_plugin() {
    let dir = tempfile::tempdir().unwrap();
    cmd_plugin_init("test-run", Some(dir.path()), false).unwrap();
    let r = cmd_plugin_run("test-run", "check", dir.path(), "{}", false);
    if crate::core::plugin_runtime::is_runtime_available() {
        // Stub WASM bytes fail real parsing — expected error
        assert!(r.is_err());
    } else {
        assert!(r.is_ok(), "run check failed: {:?}", r.err());
    }
}
#[test]
fn run_apply_with_plugin() {
    let dir = tempfile::tempdir().unwrap();
    cmd_plugin_init("test-apply", Some(dir.path()), false).unwrap();
    let r = cmd_plugin_run("test-apply", "apply", dir.path(), "{}", true);
    if crate::core::plugin_runtime::is_runtime_available() {
        assert!(r.is_err());
    } else {
        assert!(r.is_ok(), "run apply failed: {:?}", r.err());
    }
}
#[test]
fn run_invalid_operation() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_plugin_run("test", "invalid", dir.path(), "{}", false);
    assert!(r.unwrap_err().contains("invalid operation"));
}
#[test]
fn run_invalid_config() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_plugin_run("test", "check", dir.path(), "bad json", false);
    assert!(r.unwrap_err().contains("parse config"));
}
#[test]
fn run_missing_plugin() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_plugin_run("nonexistent", "check", dir.path(), "{}", false);
    assert!(r.is_err());
}
