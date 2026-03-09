//! FJ-3404/095: Plugin dispatch and reproducible build verification.
//! Usage: cargo test --test falsification_plugin_dispatch_repro

use forjar::core::plugin_dispatch::{
    available_plugin_types, dispatch_apply, dispatch_check, dispatch_destroy, is_plugin_type,
    parse_plugin_type,
};
use forjar::core::repro_build::{
    check_cargo_profile, check_environment, generate_report, hash_binary, hash_source_dir,
    repro_ci_snippet, ReproReport,
};

// ── FJ-3404: parse_plugin_type ──

#[test]
fn parse_plugin_type_valid() {
    assert_eq!(parse_plugin_type("plugin:nginx"), Some("nginx"));
    assert_eq!(parse_plugin_type("plugin:my-custom"), Some("my-custom"));
    assert_eq!(parse_plugin_type("plugin:a"), Some("a"));
}

#[test]
fn parse_plugin_type_invalid() {
    assert_eq!(parse_plugin_type("package"), None);
    assert_eq!(parse_plugin_type("file"), None);
    assert_eq!(parse_plugin_type("plugin"), None);
    assert_eq!(parse_plugin_type(""), None);
}

#[test]
fn is_plugin_type_check() {
    assert!(is_plugin_type("plugin:foo"));
    assert!(is_plugin_type("plugin:"));
    assert!(!is_plugin_type("package"));
    assert!(!is_plugin_type("file"));
    assert!(!is_plugin_type(""));
}

// ── FJ-3404: dispatch with missing plugins ──

#[test]
fn dispatch_check_missing() {
    let dir = tempfile::tempdir().unwrap();
    let config = serde_json::json!({"key": "val"});
    let result = dispatch_check(dir.path(), "nonexistent", &config);
    assert!(!result.success);
    assert_eq!(result.operation, "check");
    assert_eq!(result.plugin_name, "nonexistent");
}

#[test]
fn dispatch_apply_missing() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_apply(dir.path(), "missing", &serde_json::json!({}));
    assert!(!result.success);
    assert_eq!(result.operation, "apply");
}

#[test]
fn dispatch_destroy_missing() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_destroy(dir.path(), "gone", &serde_json::json!({}));
    assert!(!result.success);
    assert_eq!(result.operation, "destroy");
}

// ── FJ-3404: available_plugin_types ──

#[test]
fn available_plugins_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    assert!(available_plugin_types(dir.path()).is_empty());
}

// ── FJ-3404: dispatch with real plugin ──

fn setup_plugin(dir: &std::path::Path, name: &str) {
    let pdir = dir.join(name);
    std::fs::create_dir_all(&pdir).unwrap();
    let wasm = b"fake wasm content";
    let hash = blake3::hash(wasm).to_hex().to_string();
    std::fs::write(pdir.join("plugin.wasm"), wasm).unwrap();
    std::fs::write(
        pdir.join("plugin.yaml"),
        format!(
            "name: {name}\nversion: \"0.1.0\"\nabi_version: 1\nwasm: plugin.wasm\nblake3: {hash}\npermissions:\n  fs: {{}}\n  net: {{}}\n  env: {{}}\n  exec: {{}}\n"
        ),
    )
    .unwrap();
}

#[test]
fn dispatch_check_real_plugin() {
    let dir = tempfile::tempdir().unwrap();
    setup_plugin(dir.path(), "test-plug");
    let r = dispatch_check(dir.path(), "test-plug", &serde_json::json!({}));
    assert!(r.success, "failed: {}", r.message);
    assert_eq!(r.operation, "check");
}

#[test]
fn dispatch_apply_real_plugin() {
    let dir = tempfile::tempdir().unwrap();
    setup_plugin(dir.path(), "test-plug");
    let r = dispatch_apply(dir.path(), "test-plug", &serde_json::json!({}));
    assert!(r.success, "failed: {}", r.message);
    assert_eq!(r.operation, "apply");
}

#[test]
fn dispatch_destroy_real_plugin() {
    let dir = tempfile::tempdir().unwrap();
    setup_plugin(dir.path(), "test-plug");
    let r = dispatch_destroy(dir.path(), "test-plug", &serde_json::json!({}));
    assert!(r.success, "failed: {}", r.message);
    assert_eq!(r.operation, "destroy");
}

#[test]
fn available_plugins_with_real() {
    let dir = tempfile::tempdir().unwrap();
    setup_plugin(dir.path(), "my-plugin");
    let types = available_plugin_types(dir.path());
    assert!(types.contains(&"plugin:my-plugin".to_string()));
}

// ── FJ-095: check_environment ──

#[test]
fn check_env_returns_checks() {
    let checks = check_environment();
    assert!(checks.len() >= 3);
    assert!(checks.iter().any(|c| c.name == "SOURCE_DATE_EPOCH"));
    assert!(checks.iter().any(|c| c.name == "CARGO_INCREMENTAL"));
    assert!(checks.iter().any(|c| c.name == "STRIP_SETTING"));
}

// ── FJ-095: check_cargo_profile ──

#[test]
fn cargo_profile_good() {
    let toml = "[profile.release]\npanic = \"abort\"\nlto = true\ncodegen-units = 1\n";
    let checks = check_cargo_profile(toml);
    assert!(checks.iter().all(|c| c.passed));
}

#[test]
fn cargo_profile_missing() {
    let checks = check_cargo_profile("[package]\nname = \"test\"\n");
    assert!(checks.iter().any(|c| !c.passed));
}

// ── FJ-095: hash_binary ──

#[test]
fn hash_binary_missing_file() {
    assert!(hash_binary(std::path::Path::new("/nonexistent")).is_err());
}

#[test]
fn hash_binary_real_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.bin");
    std::fs::write(&path, b"binary content").unwrap();
    let hash = hash_binary(&path).unwrap();
    assert_eq!(hash.len(), 64);
}

// ── FJ-095: hash_source_dir ──

#[test]
fn hash_source_dir_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
    let h1 = hash_source_dir(dir.path()).unwrap();
    let h2 = hash_source_dir(dir.path()).unwrap();
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64);
}

#[test]
fn hash_source_dir_skips_target_and_dotdirs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("lib.rs"), "pub fn f() {}").unwrap();
    std::fs::create_dir(dir.path().join("target")).unwrap();
    std::fs::write(dir.path().join("target/debug.rs"), "ignored").unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    std::fs::write(dir.path().join(".git/config.rs"), "ignored").unwrap();
    let hash = hash_source_dir(dir.path()).unwrap();
    assert_eq!(hash.len(), 64);
}

// ── FJ-095: generate_report ──

#[test]
fn generate_report_fields() {
    let r = generate_report("src-hash", "bin-hash", "[profile.release]\nlto = true\n");
    assert_eq!(r.source_hash, "src-hash");
    assert_eq!(r.binary_hash, "bin-hash");
    assert_eq!(r.profile, "release");
    assert!(!r.checks.is_empty());
}

#[test]
fn generate_report_serde() {
    let r = generate_report("a", "b", "");
    let json = serde_json::to_string(&r).unwrap();
    let parsed: ReproReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.source_hash, "a");
    assert_eq!(parsed.binary_hash, "b");
}

// ── FJ-095: repro_ci_snippet ──

#[test]
fn ci_snippet_contents() {
    let ci = repro_ci_snippet();
    assert!(ci.contains("SOURCE_DATE_EPOCH"));
    assert!(ci.contains("CARGO_INCREMENTAL"));
    assert!(ci.contains("b3sum"));
    assert!(ci.contains("cargo build --release"));
}
