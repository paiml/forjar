//! Tests for FJ-1260: Output persistence and cross-stack data flow.

use super::*;
use crate::core::types::GlobalLock;
use tempfile::TempDir;

/// GlobalLock serde roundtrip with outputs.
#[test]
fn roundtrip_global_lock_with_outputs() {
    let mut lock = new_global_lock("test-config");
    lock.outputs
        .insert("db_host".to_string(), "10.0.0.5".to_string());
    lock.outputs
        .insert("api_port".to_string(), "8080".to_string());

    let yaml = serde_yaml_ng::to_string(&lock).expect("serialize");
    let parsed: GlobalLock = serde_yaml_ng::from_str(&yaml).expect("deserialize");

    assert_eq!(parsed.outputs.len(), 2);
    assert_eq!(parsed.outputs["db_host"], "10.0.0.5");
    assert_eq!(parsed.outputs["api_port"], "8080");
}

/// Backward compat: old lock files without outputs field deserialize fine.
#[test]
fn backward_compat_no_outputs_field() {
    let yaml = r#"
schema: "1.0"
name: old-config
last_apply: "2024-01-01T00:00:00Z"
generator: forjar 1.0.0
machines: {}
"#;
    let lock: GlobalLock = serde_yaml_ng::from_str(yaml).expect("deserialize");
    assert!(lock.outputs.is_empty());
}

/// GlobalLock with empty outputs does not serialize the outputs key.
#[test]
fn empty_outputs_skipped_in_serialization() {
    let lock = new_global_lock("test");
    let yaml = serde_yaml_ng::to_string(&lock).expect("serialize");
    assert!(
        !yaml.contains("outputs:"),
        "empty outputs should be skipped"
    );
}

/// persist_outputs + load_global_lock roundtrip.
#[test]
fn persist_and_load_outputs() {
    let tmp = TempDir::new().expect("tempdir");
    let state_dir = tmp.path();

    // First update the global lock so it exists
    update_global_lock(state_dir, "myconfig", &[]).expect("update global lock");

    // Persist outputs
    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("vip_addr".to_string(), "192.168.1.100".to_string());
    outputs.insert("cluster_name".to_string(), "prod-east".to_string());
    persist_outputs(state_dir, "myconfig", &outputs, false).expect("persist outputs");

    // Load and verify
    let lock = load_global_lock(state_dir)
        .expect("load")
        .expect("lock exists");
    assert_eq!(lock.outputs.len(), 2);
    assert_eq!(lock.outputs["vip_addr"], "192.168.1.100");
    assert_eq!(lock.outputs["cluster_name"], "prod-east");
}

/// persist_outputs creates a global lock if one doesn't exist yet.
#[test]
fn persist_outputs_creates_lock() {
    let tmp = TempDir::new().expect("tempdir");
    let state_dir = tmp.path();

    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("key".to_string(), "value".to_string());
    persist_outputs(state_dir, "new-config", &outputs, false).expect("persist");

    let lock = load_global_lock(state_dir).expect("load").expect("exists");
    assert_eq!(lock.name, "new-config");
    assert_eq!(lock.outputs["key"], "value");
}

/// Overwriting outputs replaces previous values.
#[test]
fn persist_outputs_overwrites_previous() {
    let tmp = TempDir::new().expect("tempdir");
    let state_dir = tmp.path();

    let mut v1 = indexmap::IndexMap::new();
    v1.insert("port".to_string(), "8080".to_string());
    persist_outputs(state_dir, "cfg", &v1, false).expect("persist v1");

    let mut v2 = indexmap::IndexMap::new();
    v2.insert("port".to_string(), "9090".to_string());
    v2.insert("host".to_string(), "new-host".to_string());
    persist_outputs(state_dir, "cfg", &v2, false).expect("persist v2");

    let lock = load_global_lock(state_dir).expect("load").expect("exists");
    assert_eq!(lock.outputs.len(), 2);
    assert_eq!(lock.outputs["port"], "9090");
    assert_eq!(lock.outputs["host"], "new-host");
}

/// FJ-3300: Ephemeral mode redacts all outputs to BLAKE3 hashes.
#[test]
fn persist_outputs_ephemeral_redacts() {
    let tmp = TempDir::new().expect("tempdir");
    let state_dir = tmp.path();

    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("db_password".to_string(), "super-secret".to_string());
    outputs.insert("app_port".to_string(), "8080".to_string());

    persist_outputs(state_dir, "cfg", &outputs, true).expect("persist ephemeral");

    let lock = load_global_lock(state_dir).expect("load").expect("exists");
    // All values should be ephemeral markers, not cleartext
    for (_k, v) in &lock.outputs {
        assert!(
            crate::core::state::ephemeral::is_ephemeral_marker(v),
            "expected ephemeral marker, got: {v}"
        );
    }
    // Original cleartext should NOT appear
    assert!(!lock.outputs["db_password"].contains("super-secret"));
    assert!(!lock.outputs["app_port"].contains("8080"));
}

/// FJ-3300: Non-ephemeral mode preserves cleartext.
#[test]
fn persist_outputs_non_ephemeral_preserves() {
    let tmp = TempDir::new().expect("tempdir");
    let state_dir = tmp.path();

    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("data_dir".to_string(), "/var/data".to_string());

    persist_outputs(state_dir, "cfg", &outputs, false).expect("persist");

    let lock = load_global_lock(state_dir).expect("load").expect("exists");
    assert_eq!(lock.outputs["data_dir"], "/var/data");
}

/// FJ-3300: Ephemeral drift detection via hash comparison.
#[test]
fn ephemeral_drift_detection() {
    use crate::core::state::ephemeral;

    let secret = "my-db-password-2026";
    let marker = ephemeral::redact_to_hash(secret);

    // Same secret: no drift
    assert!(ephemeral::verify_drift(secret, &marker));

    // Changed secret: drift detected
    assert!(!ephemeral::verify_drift("changed-password", &marker));
}
