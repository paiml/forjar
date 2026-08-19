//! FJ-3409: Integration test — WASM plugin check/apply/destroy lifecycle.
//!
//! Tests the full plugin lifecycle: manifest resolution → BLAKE3 verification →
//! schema validation → dispatch check → dispatch apply → dispatch destroy.

use forjar::core::plugin_dispatch::{
    available_plugin_types, dispatch_check, is_plugin_type, parse_plugin_type,
};

// `apply` and `destroy` are only dispatched by the stub-runtime lifecycle
// tests. Importing them unconditionally warns under `--all-features`, which is
// the configuration the release dogfood gate runs — so its warnings matter.
#[cfg(not(feature = "wasm-runtime"))]
use forjar::core::plugin_dispatch::{dispatch_apply, dispatch_destroy};
use forjar::core::plugin_hot_reload::{PluginCache, ReloadCheck};
use forjar::core::plugin_loader::{
    list_plugins, resolve_and_verify, resolve_manifest, verify_plugin,
};
use forjar::core::types::PluginStatus;
use tempfile::TempDir;

fn create_plugin(dir: &std::path::Path, name: &str, wasm_content: &[u8]) -> String {
    let plugin_dir = dir.join(name);
    std::fs::create_dir_all(&plugin_dir).unwrap();

    let hash = blake3::hash(wasm_content).to_hex().to_string();
    std::fs::write(plugin_dir.join("plugin.wasm"), wasm_content).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.yaml"),
        format!(
            r#"
name: {name}
version: "1.0.0"
abi_version: 1
description: "Test plugin {name}"
wasm: plugin.wasm
blake3: {hash}
permissions:
  fs: {{}}
  net: {{}}
  env: {{}}
  exec: {{}}
schema:
  properties:
    port:
      type: integer
    host:
      type: string
  required:
    - host
"#
        ),
    )
    .unwrap();
    hash
}

/// A REAL (minimal, empty) WebAssembly module: the `\0asm` magic plus version 1.
///
/// Only the *dispatch* tests need this. The verification and hot-reload tests
/// below deliberately keep arbitrary bytes — they assert on BLAKE3 identity and
/// change detection, for which the payload is opaque and a real module would
/// prove nothing extra.
///
/// The dispatch tests used arbitrary bytes too, which passes only when no
/// runtime exists to reject them. Under `--all-features` the real wasmtime
/// runtime loads the file and fails with `magic header not detected`. A fixture
/// that is not the real artifact tests only the absence of a validator.
const REAL_EMPTY_WASM: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

/// Full check -> apply -> destroy lifecycle, stub-runtime configuration.
///
/// With `wasm-runtime` enabled the empty module exports no `check`, so success
/// is not the right assertion — see `dispatch_rejects_a_module_without_exports`.
#[cfg(not(feature = "wasm-runtime"))]
#[test]
fn plugin_lifecycle_check_apply_destroy() {
    let dir = TempDir::new().unwrap();
    create_plugin(dir.path(), "nginx-config", REAL_EMPTY_WASM);
    let config = serde_json::json!({"host": "web-01", "port": 8080});

    // Check
    let check = dispatch_check(dir.path(), "nginx-config", &config);
    assert!(check.success, "check failed: {}", check.message);
    assert_eq!(check.operation, "check");
    assert_eq!(check.status, PluginStatus::Converged);

    // Apply
    let apply = dispatch_apply(dir.path(), "nginx-config", &config);
    assert!(apply.success, "apply failed: {}", apply.message);
    assert_eq!(apply.operation, "apply");
    assert_eq!(apply.status, PluginStatus::Converged);

    // Destroy
    let destroy = dispatch_destroy(dir.path(), "nginx-config", &config);
    assert!(destroy.success, "destroy failed: {}", destroy.message);
    assert_eq!(destroy.operation, "destroy");
}

/// Test: manifest resolution and BLAKE3 verification.
#[test]
fn manifest_resolution_and_verification() {
    let dir = TempDir::new().unwrap();
    let expected_hash = create_plugin(dir.path(), "verified-plugin", b"wasm content");

    let manifest = resolve_manifest(dir.path(), "verified-plugin").unwrap();
    assert_eq!(manifest.name, "verified-plugin");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.blake3, expected_hash);

    // verify_plugin returns Ok(()) on success
    verify_plugin(dir.path(), &manifest).unwrap();
}

/// Test: BLAKE3 hash mismatch detected.
#[test]
fn blake3_hash_mismatch_detected() {
    let dir = TempDir::new().unwrap();
    create_plugin(dir.path(), "tampered", b"original content");

    // Tamper with the WASM file
    let wasm_path = dir.path().join("tampered").join("plugin.wasm");
    std::fs::write(&wasm_path, b"tampered content").unwrap();

    let manifest = resolve_manifest(dir.path(), "tampered").unwrap();
    let result = verify_plugin(dir.path(), &manifest);
    assert!(result.is_err(), "should detect hash mismatch");
    assert!(result.unwrap_err().contains("hash mismatch"));
}

/// Test: schema validation passes for valid config.
#[test]
fn schema_validation_valid_config() {
    let dir = TempDir::new().unwrap();
    create_plugin(dir.path(), "schema-test", b"wasm bytes");

    let manifest = resolve_manifest(dir.path(), "schema-test").unwrap();
    let mut props = indexmap::IndexMap::new();
    props.insert(
        "host".to_string(),
        serde_yaml_ng::Value::String("web-01".into()),
    );
    props.insert(
        "port".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(8080)),
    );
    let errors = forjar::core::plugin_loader::validate_resource_schema(&manifest, &props);
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
}

/// Test: schema validation fails for missing required field.
#[test]
fn schema_validation_missing_required() {
    let dir = TempDir::new().unwrap();
    create_plugin(dir.path(), "schema-fail", b"wasm bytes");

    let manifest = resolve_manifest(dir.path(), "schema-fail").unwrap();
    let mut props = indexmap::IndexMap::new();
    props.insert(
        "port".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(8080)),
    );
    // missing required 'host'
    let errors = forjar::core::plugin_loader::validate_resource_schema(&manifest, &props);
    assert!(!errors.is_empty(), "should have validation errors");
}

/// Test: dispatch for non-existent plugin.
#[test]
fn dispatch_nonexistent_plugin() {
    let dir = TempDir::new().unwrap();
    let config = serde_json::json!({});

    let result = dispatch_check(dir.path(), "does-not-exist", &config);
    assert!(!result.success);
    assert_eq!(result.status, PluginStatus::Error);
}

/// Test: list plugins in directory.
#[test]
fn list_plugins_in_dir() {
    let dir = TempDir::new().unwrap();
    create_plugin(dir.path(), "alpha", b"alpha wasm");
    create_plugin(dir.path(), "beta", b"beta wasm");

    let plugins = list_plugins(dir.path());
    assert!(plugins.contains(&"alpha".to_string()));
    assert!(plugins.contains(&"beta".to_string()));
}

/// Test: available_plugin_types returns prefixed names.
#[test]
fn plugin_types_prefixed() {
    let dir = TempDir::new().unwrap();
    create_plugin(dir.path(), "custom", b"custom wasm");

    let types = available_plugin_types(dir.path());
    assert!(types.contains(&"plugin:custom".to_string()));
}

/// Test: parse_plugin_type and is_plugin_type.
#[test]
fn plugin_type_parsing() {
    assert_eq!(parse_plugin_type("plugin:nginx"), Some("nginx"));
    assert_eq!(parse_plugin_type("file"), None);
    assert!(is_plugin_type("plugin:custom"));
    assert!(!is_plugin_type("package"));
}

/// Test: hot-reload detects WASM file change.
#[test]
fn hot_reload_detects_change() {
    let dir = TempDir::new().unwrap();
    let hash = create_plugin(dir.path(), "hot-plugin", b"original wasm v1");
    let wasm_path = dir.path().join("hot-plugin").join("plugin.wasm");

    let manifest = resolve_manifest(dir.path(), "hot-plugin").unwrap();
    let mut cache = PluginCache::new();
    cache.insert("hot-plugin", manifest, wasm_path.clone());

    // Initially up to date
    assert!(matches!(
        cache.needs_reload("hot-plugin"),
        ReloadCheck::UpToDate
    ));

    // Modify the WASM file
    std::fs::write(&wasm_path, b"updated wasm v2").unwrap();

    // Should detect change
    match cache.needs_reload("hot-plugin") {
        ReloadCheck::Changed { old_hash, new_hash } => {
            assert_eq!(old_hash, hash);
            assert_ne!(old_hash, new_hash);
        }
        other => panic!("expected Changed, got {:?}", other),
    }
}

/// Test: hot-reload detects deleted plugin.
#[test]
fn hot_reload_detects_deletion() {
    let dir = TempDir::new().unwrap();
    create_plugin(dir.path(), "del-plugin", b"wasm data");
    let wasm_path = dir.path().join("del-plugin").join("plugin.wasm");

    let manifest = resolve_manifest(dir.path(), "del-plugin").unwrap();
    let mut cache = PluginCache::new();
    cache.insert("del-plugin", manifest, wasm_path.clone());

    // Delete the WASM file
    std::fs::remove_file(&wasm_path).unwrap();

    assert!(matches!(
        cache.needs_reload("del-plugin"),
        ReloadCheck::FileGone
    ));
}

/// Test: resolve_and_verify full pipeline.
#[test]
fn resolve_and_verify_pipeline() {
    let dir = TempDir::new().unwrap();
    create_plugin(dir.path(), "pipeline", b"pipeline wasm");

    let resolved = resolve_and_verify(dir.path(), "pipeline").unwrap();
    assert_eq!(resolved.manifest.name, "pipeline");
    assert_eq!(resolved.status, PluginStatus::Converged);
}

/// Multiple operations on the same plugin, stub-runtime configuration.
#[cfg(not(feature = "wasm-runtime"))]
#[test]
fn multiple_operations_same_plugin() {
    let dir = TempDir::new().unwrap();
    create_plugin(dir.path(), "multi-op", REAL_EMPTY_WASM);
    let config = serde_json::json!({"host": "app-01"});

    // Check twice
    let r1 = dispatch_check(dir.path(), "multi-op", &config);
    let r2 = dispatch_check(dir.path(), "multi-op", &config);
    assert!(r1.success);
    assert!(r2.success);

    // Apply then destroy
    let apply = dispatch_apply(dir.path(), "multi-op", &config);
    assert!(apply.success);
    let destroy = dispatch_destroy(dir.path(), "multi-op", &config);
    assert!(destroy.success);
}

/// With a runtime, a structurally valid module that exports no entrypoint is
/// rejected rather than reported as a successful dispatch.
#[cfg(feature = "wasm-runtime")]
#[test]
fn dispatch_rejects_a_module_without_exports() {
    let dir = TempDir::new().unwrap();
    create_plugin(dir.path(), "nginx-config", REAL_EMPTY_WASM);
    let config = serde_json::json!({"host": "web-01", "port": 8080});

    let check = dispatch_check(dir.path(), "nginx-config", &config);
    assert!(
        !check.success,
        "an empty module exports no `check`; dispatch must not report success"
    );
    // It must fail on the missing export, NOT the magic number — a magic-number
    // failure would mean the fixture stopped being a real module.
    assert!(
        !check.message.contains("magic header"),
        "fixture is not a real wasm module: {}",
        check.message
    );
}
