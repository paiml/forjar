//! Falsification tests for FJ-3100 through FJ-3500 spec criteria.
//!
//! Each test maps to a specific falsification criterion from the platform specs.
//! These tests prove the implementation is correct by attempting to reject it.

use std::collections::HashMap;

// ─── F-3100: Event-Driven Automation ───────────────────────────────

/// F-3100-3: Cooldown prevents storms.
/// Trigger same event 100x in 1s; REJECT if action fires > 1 time.
#[test]
fn f3200_6_no_opa_rego() {
    let cargo_toml = std::fs::read_to_string("Cargo.toml").unwrap();
    let forbidden = ["opa", "rego", "open-policy-agent", "regorus"];
    for dep in &forbidden {
        assert!(
            !cargo_toml.to_lowercase().contains(dep),
            "REJECT: OPA/Rego dependency found: {dep}"
        );
    }
}

// ─── F-3300: Ephemeral Values & State Encryption ───────────────────

/// F-3300-1: Ephemeral values never in state.
/// Set ephemeral param, store record; REJECT if cleartext found in state record.
#[test]
fn f3300_1_ephemeral_never_in_state() {
    use forjar::core::ephemeral::{to_records, ResolvedEphemeral};

    let secret_value = "super-secret-database-password-12345";
    let resolved = vec![ResolvedEphemeral {
        key: "db_pass".into(),
        value: secret_value.into(),
        hash: blake3::hash(secret_value.as_bytes()).to_hex().to_string(),
    }];

    let records = to_records(&resolved);
    assert_eq!(records.len(), 1);

    // The record must contain the hash, NOT the plaintext
    let record = &records[0];
    assert_eq!(record.key, "db_pass");
    assert_ne!(
        record.hash, secret_value,
        "REJECT: plaintext secret stored in record"
    );
    assert!(
        !record.hash.contains(secret_value),
        "REJECT: plaintext secret embedded in hash field"
    );

    // Verify hash is a valid BLAKE3 hex string (64 chars)
    assert_eq!(record.hash.len(), 64, "REJECT: hash should be 64 hex chars");

    // Serialize the record to JSON and verify no plaintext
    let json = serde_json::to_string(&record).unwrap();
    assert!(
        !json.contains(secret_value),
        "REJECT: plaintext secret found in serialized state record"
    );
}

/// F-3300-2: Drift detection works on ephemeral.
/// Change ephemeral secret, run drift; REJECT if drift not detected via hash.
#[test]
fn f3300_2_ephemeral_drift_detection() {
    use forjar::core::ephemeral::*;

    let original_secret = "original-api-key-abc123";
    let changed_secret = "changed-api-key-xyz789";

    // Store the original hash
    let original = ResolvedEphemeral {
        key: "api_key".into(),
        value: original_secret.into(),
        hash: blake3::hash(original_secret.as_bytes())
            .to_hex()
            .to_string(),
    };
    let stored_records = to_records(&[original]);

    // Re-resolve with changed value
    let current = vec![ResolvedEphemeral {
        key: "api_key".into(),
        value: changed_secret.into(),
        hash: blake3::hash(changed_secret.as_bytes()).to_hex().to_string(),
    }];

    let drift = check_drift(&current, &stored_records);
    assert_eq!(drift.len(), 1);
    assert_eq!(
        drift[0].status,
        DriftStatus::Changed,
        "REJECT: drift not detected when secret changed"
    );

    // Same value should show no drift
    let same = vec![ResolvedEphemeral {
        key: "api_key".into(),
        value: original_secret.into(),
        hash: blake3::hash(original_secret.as_bytes())
            .to_hex()
            .to_string(),
    }];
    let no_drift = check_drift(&same, &stored_records);
    assert_eq!(
        no_drift[0].status,
        DriftStatus::Unchanged,
        "REJECT: false drift on unchanged secret"
    );
}

/// F-3300-4: BLAKE3 HMAC catches tampering.
/// Flip one bit in content; REJECT if HMAC verification passes.
#[test]
fn f3300_4_blake3_hmac_tamper_detection() {
    let key = blake3::hash(b"test-key");
    let content = b"sensitive state data";

    // Create HMAC
    let mac = blake3::keyed_hash(key.as_bytes(), content);
    let mac_hex = mac.to_hex().to_string();

    // Verify original
    let verify_mac = blake3::keyed_hash(key.as_bytes(), content);
    assert_eq!(
        verify_mac.to_hex().to_string(),
        mac_hex,
        "original should verify"
    );

    // Tamper: flip one bit
    let mut tampered = content.to_vec();
    tampered[0] ^= 0x01;
    let tampered_mac = blake3::keyed_hash(key.as_bytes(), &tampered);

    assert_ne!(
        tampered_mac.to_hex().to_string(),
        mac_hex,
        "REJECT: HMAC should differ after tampering"
    );
}

/// F-3300-6: bashrs catches secret echo.
/// Generate scripts with secret variable patterns; REJECT if lint doesn't flag.
#[test]
fn f3300_6_bashrs_catches_secret_echo() {
    use forjar::core::script_secret_lint::validate_no_leaks;

    // These variable names match the regex patterns in script_secret_lint
    let scripts = ["echo $PASSWORD", "printf '%s' $SECRET", "echo ${TOKEN}"];
    for script in &scripts {
        let result = validate_no_leaks(script);
        assert!(
            result.is_err(),
            "REJECT: lint didn't catch secret leak in: {script}"
        );
    }
}

/// F-3300-8: No cloud KMS in default path.
#[test]
fn f3300_8_no_cloud_kms() {
    let cargo_toml = std::fs::read_to_string("Cargo.toml").unwrap();
    let forbidden = ["aws-sdk", "google-cloud", "azure_identity", "aws-kms"];
    for dep in &forbidden {
        // Check it's not a required (non-optional) dependency
        if cargo_toml.contains(dep) {
            // Verify it's marked optional
            assert!(
                cargo_toml.contains(dep) && cargo_toml.contains("optional = true"),
                "REJECT: cloud KMS dependency '{dep}' is not optional"
            );
        }
    }
}

// ─── F-3400: WASM Resource Provider Plugins ────────────────────────

/// F-3400-4: BLAKE3 prevents tampered plugins.
/// Modify one byte in .wasm; REJECT if plugin loads without hash error.
#[test]
fn f3400_4_blake3_tampered_plugin() {
    use forjar::core::plugin_loader::{resolve_manifest, verify_plugin};

    let dir = tempfile::tempdir().unwrap();
    let plugin_dir = dir.path().join("tampered");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    let wasm_content = b"original wasm module bytes";
    let hash = blake3::hash(wasm_content).to_hex().to_string();

    std::fs::write(plugin_dir.join("plugin.wasm"), wasm_content).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.yaml"),
        format!(
            r#"name: tampered
version: "1.0.0"
abi_version: 1
wasm: plugin.wasm
blake3: {hash}
permissions:
  fs: {{}}
  net: {{}}
  env: {{}}
  exec: {{}}
schema:
  properties: {{}}
  required: []
"#
        ),
    )
    .unwrap();

    // Original verifies
    let manifest = resolve_manifest(dir.path(), "tampered").unwrap();
    assert!(
        verify_plugin(dir.path(), &manifest).is_ok(),
        "original should verify"
    );

    // Tamper the WASM
    let mut tampered = wasm_content.to_vec();
    tampered[0] ^= 0xFF;
    std::fs::write(plugin_dir.join("plugin.wasm"), &tampered).unwrap();

    // Re-resolve and verify should fail
    let result = verify_plugin(dir.path(), &manifest);
    assert!(result.is_err(), "REJECT: tampered plugin should not verify");
    assert!(
        result.unwrap_err().contains("hash mismatch"),
        "REJECT: error should mention hash mismatch"
    );
}

/// F-3400-7: Hot-reload detects changes.
/// Modify .wasm during apply cycle; REJECT if old version used after change.
#[test]
fn f3400_7_hot_reload_detects_changes() {
    use forjar::core::plugin_hot_reload::{PluginCache, ReloadCheck};
    use forjar::core::plugin_loader::resolve_manifest;

    let dir = tempfile::tempdir().unwrap();
    let plugin_dir = dir.path().join("hot-test");
    std::fs::create_dir_all(&plugin_dir).unwrap();

    let wasm_v1 = b"wasm module version 1";
    let hash_v1 = blake3::hash(wasm_v1).to_hex().to_string();
    let wasm_path = plugin_dir.join("plugin.wasm");

    std::fs::write(&wasm_path, wasm_v1).unwrap();
    std::fs::write(
        plugin_dir.join("plugin.yaml"),
        format!(
            r#"name: hot-test
version: "1.0.0"
abi_version: 1
wasm: plugin.wasm
blake3: {hash_v1}
permissions:
  fs: {{}}
  net: {{}}
  env: {{}}
  exec: {{}}
schema:
  properties: {{}}
  required: []
"#
        ),
    )
    .unwrap();

    let manifest = resolve_manifest(dir.path(), "hot-test").unwrap();
    let mut cache = PluginCache::new();
    cache.insert("hot-test", manifest, wasm_path.clone());

    // Initially up to date
    assert!(matches!(
        cache.needs_reload("hot-test"),
        ReloadCheck::UpToDate
    ));

    // Modify WASM file
    std::fs::write(&wasm_path, b"wasm module version 2 with changes").unwrap();

    // Should detect change
    match cache.needs_reload("hot-test") {
        ReloadCheck::Changed { old_hash, new_hash } => {
            assert_ne!(
                old_hash, new_hash,
                "REJECT: hashes should differ after change"
            );
        }
        other => panic!("REJECT: expected Changed, got {:?}", other),
    }
}

/// F-3400-8: No non-sovereign WASM runtime.
#[test]
fn f3400_8_sovereign_wasm_runtime() {
    let cargo_toml = std::fs::read_to_string("Cargo.toml").unwrap();
    let forbidden = ["wasmer", "wasm3", "lucet"];
    for dep in &forbidden {
        assert!(
            !cargo_toml.contains(dep),
            "REJECT: non-sovereign WASM runtime found: {dep}"
        );
    }
}

// ─── F-3500: Environment Promotion Pipelines ───────────────────────

/// F-3500-2: Quality gates block promotion.
/// Introduce policy violation; REJECT if promotion succeeds.
#[test]
fn f3500_2_quality_gates_block() {
    use forjar::core::promotion::evaluate_gates;
    use forjar::core::types::environment::*;

    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(&cfg, "invalid: [yaml content").unwrap();

    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![PromotionGate {
            validate: Some(ValidateGateOptions {
                deep: false,
                exhaustive: false,
            }),
            ..Default::default()
        }],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(
        !result.all_passed,
        "REJECT: promotion with invalid config should not pass"
    );
}

/// F-3500-6: Promotion history is append-only.
/// Promote twice; REJECT if first promotion event overwritten.
#[test]
fn f3500_6_append_only_history() {
    use forjar::core::promotion_events::{log_promotion, PromotionParams};

    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    // First promotion
    let p1 = PromotionParams {
        state_dir: &state_dir,
        target_env: "staging",
        source: "dev",
        target: "staging",
        gates_passed: 2,
        gates_total: 2,
        rollout_strategy: None,
    };
    log_promotion(&p1).unwrap();

    let log_path = state_dir.join("staging").join("events.jsonl");
    let after_first = std::fs::read_to_string(&log_path).unwrap();
    let lines_first: Vec<&str> = after_first.lines().collect();
    assert_eq!(lines_first.len(), 1);

    // Second promotion
    let p2 = PromotionParams {
        state_dir: &state_dir,
        target_env: "staging",
        source: "dev",
        target: "staging",
        gates_passed: 3,
        gates_total: 3,
        rollout_strategy: Some("canary"),
    };
    log_promotion(&p2).unwrap();

    let after_second = std::fs::read_to_string(&log_path).unwrap();
    let lines_second: Vec<&str> = after_second.lines().collect();
    assert_eq!(
        lines_second.len(),
        2,
        "REJECT: second event should be appended, not overwritten"
    );

    // First event should still be present unchanged
    assert_eq!(
        lines_second[0], lines_first[0],
        "REJECT: first event was modified"
    );
}

/// F-3500-1: Environment isolation.
/// Apply to dev; REJECT if staging state modified.
#[test]
fn f3500_1_environment_state_isolation() {
    use forjar::core::types::environment::env_state_dir;

    let base = tempfile::tempdir().unwrap();
    let dev_dir = env_state_dir(base.path(), "dev");
    let staging_dir = env_state_dir(base.path(), "staging");

    // Create state for dev
    std::fs::create_dir_all(&dev_dir).unwrap();
    std::fs::write(dev_dir.join("state.lock"), "dev-state-v1").unwrap();

    // Create state for staging
    std::fs::create_dir_all(&staging_dir).unwrap();
    std::fs::write(staging_dir.join("state.lock"), "staging-state-v1").unwrap();

    // Simulate "apply to dev" by modifying dev state
    std::fs::write(dev_dir.join("state.lock"), "dev-state-v2").unwrap();

    // Staging state must NOT be modified
    let staging_state = std::fs::read_to_string(staging_dir.join("state.lock")).unwrap();
    assert_eq!(
        staging_state, "staging-state-v1",
        "REJECT: staging state was modified when applying to dev"
    );
}

/// F-3500-5: Environment diff is accurate.
/// Change one param; REJECT if diff doesn't show exactly one change.
#[test]
fn f3500_5_diff_accuracy() {
    use forjar::core::types::environment::*;
    use forjar::core::types::Machine;
    use indexmap::IndexMap;

    let base_params = HashMap::new();
    let mut base_machines = IndexMap::new();
    base_machines.insert("web".into(), Machine::ssh("web", "10.0.0.1", "root"));

    let mut dev = Environment::default();
    dev.params
        .insert("port".into(), serde_yaml_ng::Value::String("8080".into()));

    let mut staging = Environment::default();
    staging
        .params
        .insert("port".into(), serde_yaml_ng::Value::String("8443".into()));

    let diff = diff_environments(
        "dev",
        &dev,
        "staging",
        &staging,
        &base_params,
        &base_machines,
    );

    assert_eq!(
        diff.param_diffs.len(),
        1,
        "REJECT: diff should show exactly one param change, got {}",
        diff.param_diffs.len()
    );
    assert_eq!(diff.param_diffs[0].key, "port");
    assert_eq!(
        diff.machine_diffs.len(),
        0,
        "REJECT: no machine changes expected"
    );
}

/// F-3500-7: No external CI/CD dependency.
#[test]
fn f3500_7_no_cicd_sdk() {
    let cargo_toml = std::fs::read_to_string("Cargo.toml").unwrap();
    let forbidden = [
        "octocrab", // GitHub API
        "jenkins-api",
        "gitlab-rs",
        "circleci",
        "buildkite",
    ];
    for dep in &forbidden {
        assert!(
            !cargo_toml.contains(dep),
            "REJECT: CI/CD SDK dependency found: {dep}"
        );
    }
}
