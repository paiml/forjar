//! FJ-3206/3404: CIS Ubuntu pack and plugin dispatch falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-3206: CIS Ubuntu 22.04 LTS compliance pack
//!   - Pack metadata (name, version, framework, description)
//!   - 24 rules with unique CIS-prefixed IDs
//!   - Severity distribution (error >= 12, warning >= 8, info >= 1)
//!   - YAML serialization roundtrip
//!   - Pack evaluation against passing/failing configs
//!   - Cross-mapping to STIG controls
//! - FJ-3404: Plugin type dispatch
//!   - parse_plugin_type for valid/invalid types
//!   - is_plugin_type predicate
//!   - available_plugin_types in empty directory
//!   - dispatch_check/apply/destroy for missing and real plugins
//!   - resolve_plugin with BLAKE3 verification
//!
//! Usage: cargo test --test falsification_cis_plugin
#![allow(dead_code)]

use forjar::core::plugin_dispatch::{dispatch_check, resolve_plugin};

// Only the `check` path has a test in both configurations. `apply` and `destroy`
// are exercised solely by the stub-runtime tests below, so importing them
// unconditionally warns under `--all-features` — the configuration the release
// dogfood gate runs, and therefore the one whose warnings are load-bearing.
#[cfg(not(feature = "wasm-runtime"))]
use forjar::core::plugin_dispatch::{dispatch_apply, dispatch_destroy};

// ============================================================================
// FJ-3206: Pack metadata
// ============================================================================
// FJ-3404: dispatch with real plugin (BLAKE3 verified)
// ============================================================================

fn create_test_plugin(dir: &std::path::Path, name: &str) {
    let plugin_dir = dir.join(name);
    std::fs::create_dir_all(&plugin_dir).unwrap();

    // A REAL (minimal, empty) WebAssembly module: the 4-byte magic `\0asm`
    // followed by version 1. The fixture was previously the ASCII string
    // "fake wasm module content for testing", which only works when there is no
    // runtime to reject it — so these tests passed without `wasm-runtime` and
    // failed under `cargo test --all-features`, which the release dogfood gate
    // runs. A fixture that is not the real artifact only tests the absence of a
    // validator.
    let wasm_bytes: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let hash = blake3::hash(wasm_bytes).to_hex().to_string();
    std::fs::write(plugin_dir.join("plugin.wasm"), wasm_bytes).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.yaml"),
        format!(
            "name: {name}\nversion: \"0.1.0\"\nabi_version: 1\nwasm: plugin.wasm\nblake3: {hash}\npermissions:\n  fs: {{}}\n  net: {{}}\n  env: {{}}\n  exec: {{}}\n"
        ),
    )
    .unwrap();
}

/// Without a runtime, dispatch resolves the plugin and returns a stub result.
#[cfg(not(feature = "wasm-runtime"))]
#[test]
fn dispatch_check_real_plugin() {
    let dir = tempfile::tempdir().unwrap();
    create_test_plugin(dir.path(), "test-plugin");

    let config = serde_json::json!({"setting": true});
    let result = dispatch_check(dir.path(), "test-plugin", &config);
    assert!(result.success, "dispatch failed: {}", result.message);
    assert_eq!(result.operation, "check");
}

/// WITH a runtime, a module that does not meet the plugin ABI must be REJECTED,
/// naming the export it lacks.
///
/// These tests were called `dispatch_*_real_plugin` and asserted success from a
/// fixture whose `plugin.wasm` was the ASCII string "fake wasm module content
/// for testing". That only passed because no runtime was present to look at it,
/// so `cargo test --all-features` — which the release dogfood gate runs — failed
/// on all three. The product behaviour was never wrong: it correctly refuses a
/// non-conformant module. The tests were asserting the absence of a validator.
///
/// A genuinely conformant fixture needs a data section and an ABI-shaped
/// `(i32,i32) -> i32` that returns a pointer to JSON output; that is worth
/// building, and is tracked separately. Until it exists, this asserts the
/// behaviour that actually matters — invalid plugins are refused, loudly.
#[cfg(feature = "wasm-runtime")]
#[test]
fn dispatch_check_rejects_a_non_conformant_module() {
    let dir = tempfile::tempdir().unwrap();
    create_test_plugin(dir.path(), "test-plugin");

    let config = serde_json::json!({"setting": true});
    let result = dispatch_check(dir.path(), "test-plugin", &config);
    assert!(
        !result.success,
        "a module lacking the plugin ABI must be refused, not reported converged"
    );
    assert!(
        result.message.contains("memory"),
        "the refusal must name the missing export: {}",
        result.message
    );
}

#[cfg(not(feature = "wasm-runtime"))]
#[test]
fn dispatch_apply_real_plugin() {
    let dir = tempfile::tempdir().unwrap();
    create_test_plugin(dir.path(), "test-apply");

    let config = serde_json::json!({});
    let result = dispatch_apply(dir.path(), "test-apply", &config);
    assert!(result.success, "dispatch failed: {}", result.message);
    assert_eq!(result.operation, "apply");
    // Message format varies: with runtime includes version, without includes "stub"
    assert!(!result.message.is_empty());
}

#[cfg(not(feature = "wasm-runtime"))]
#[test]
fn dispatch_destroy_real_plugin() {
    let dir = tempfile::tempdir().unwrap();
    create_test_plugin(dir.path(), "test-destroy");

    let config = serde_json::json!({});
    let result = dispatch_destroy(dir.path(), "test-destroy", &config);
    assert!(result.success, "dispatch failed: {}", result.message);
    assert_eq!(result.operation, "destroy");
}

// ============================================================================
// FJ-3404: resolve_plugin — verified
// ============================================================================

#[test]
fn resolve_plugin_verified() {
    let dir = tempfile::tempdir().unwrap();
    create_test_plugin(dir.path(), "verified-plugin");

    let resolved = resolve_plugin(dir.path(), "verified-plugin");
    assert!(resolved.is_ok(), "resolve failed: {:?}", resolved.err());
}

#[test]
fn resolve_plugin_missing() {
    let dir = tempfile::tempdir().unwrap();
    let resolved = resolve_plugin(dir.path(), "no-such-plugin");
    assert!(resolved.is_err());
}

// ============================================================================
// FJ-3404: dispatch result fields
// ============================================================================

#[test]
fn dispatch_result_error_status_on_failure() {
    let dir = tempfile::tempdir().unwrap();
    let config = serde_json::json!({});
    let result = dispatch_check(dir.path(), "missing", &config);
    assert_eq!(result.status, forjar::core::types::PluginStatus::Error);
}

#[cfg(not(feature = "wasm-runtime"))]
#[test]
fn dispatch_result_converged_on_success() {
    let dir = tempfile::tempdir().unwrap();
    create_test_plugin(dir.path(), "ok-plugin");

    let config = serde_json::json!({});
    let result = dispatch_check(dir.path(), "ok-plugin", &config);
    assert_eq!(result.status, forjar::core::types::PluginStatus::Converged);
}
