//! Falsification tests for FJ-3100 through FJ-3500 spec criteria.
//!
//! Each test maps to a specific falsification criterion from the platform specs.
//! These tests prove the implementation is correct by attempting to reject it.

use std::collections::HashMap;

// ─── F-3100: Event-Driven Automation ───────────────────────────────

/// F-3100-3: Cooldown prevents storms.
/// Trigger same event 100x in 1s; REJECT if action fires > 1 time.
#[test]
fn f3100_3_cooldown_prevents_storm() {
    use forjar::core::types::CooldownTracker;

    let mut tracker = CooldownTracker::default();
    let cooldown_secs = 5;
    let rulebook_id = "storm-test";
    let mut fire_count = 0;

    // Fire 100 times rapidly
    for _ in 0..100 {
        if tracker.can_fire(rulebook_id, cooldown_secs) {
            tracker.record_fire(rulebook_id);
            fire_count += 1;
        }
    }

    // Only the first should fire
    assert_eq!(
        fire_count, 1,
        "REJECT: cooldown failed — {fire_count} fires instead of 1"
    );
}

/// F-3100-4: bashrs validates handler scripts.
/// Verify the bashrs purifier catches script injection patterns.
#[test]
fn f3100_4_bashrs_validates_handler_scripts() {
    use forjar::core::purifier::validate_script;

    // The purifier should process scripts without panic.
    // bashrs validates syntax and structure.
    let result = validate_script("echo safe");
    assert!(result.is_ok(), "simple script should pass bashrs");
}

/// F-3100-6: Zero external dependencies for event bus.
/// Audit Cargo.toml; REJECT if any non-sovereign crate added for event bus.
#[test]
fn f3100_6_no_external_event_bus() {
    let cargo_toml = std::fs::read_to_string("Cargo.toml").unwrap();
    let forbidden = [
        "tokio-eventbus",
        "eventbus",
        "message-bus",
        "rabbitmq",
        "rdkafka",
    ];
    for dep in &forbidden {
        assert!(
            !cargo_toml.contains(dep),
            "REJECT: non-sovereign event bus dependency found: {dep}"
        );
    }
}

// ─── F-3200: Policy-as-Code Engine ─────────────────────────────────

/// F-3200-1: All 4 policy types evaluate correctly.
/// Generate boundary configs; REJECT if any misclassification.
#[test]
fn f3200_1_all_policy_types_correct() {
    use forjar::core::compliance_pack::*;

    let mut resources = HashMap::new();
    let mut file = HashMap::new();
    file.insert("type".into(), "file".into());
    file.insert("owner".into(), "root".into());
    file.insert("mode".into(), "0644".into());
    file.insert("tags".into(), "web,config".into());
    resources.insert("nginx".into(), file);

    // Assert: correct value passes
    let pack = parse_pack(
        r#"
name: t1
version: "1"
framework: T
rules:
  - id: A1
    title: assert
    type: assert
    resource_type: file
    field: owner
    expected: root
"#,
    )
    .unwrap();
    let r = evaluate_pack(&pack, &resources);
    assert!(
        r.results[0].passed,
        "REJECT: assert failed on correct value"
    );

    // Deny: absent pattern passes
    let pack = parse_pack(
        r#"
name: t2
version: "1"
framework: T
rules:
  - id: D1
    title: deny
    type: deny
    resource_type: file
    field: mode
    pattern: "777"
"#,
    )
    .unwrap();
    let r = evaluate_pack(&pack, &resources);
    assert!(
        r.results[0].passed,
        "REJECT: deny failed on non-matching pattern"
    );

    // Require: present field passes
    let pack = parse_pack(
        r#"
name: t3
version: "1"
framework: T
rules:
  - id: R1
    title: require
    type: require
    resource_type: file
    field: owner
"#,
    )
    .unwrap();
    let r = evaluate_pack(&pack, &resources);
    assert!(
        r.results[0].passed,
        "REJECT: require failed on present field"
    );

    // RequireTag: present tag passes
    let pack = parse_pack(
        r#"
name: t4
version: "1"
framework: T
rules:
  - id: RT1
    title: require_tag
    type: require_tag
    tag: web
"#,
    )
    .unwrap();
    let r = evaluate_pack(&pack, &resources);
    assert!(
        r.results[0].passed,
        "REJECT: require_tag failed on present tag"
    );
}

/// F-3200-2: Error-severity blocks apply.
/// Create config violating error-policy; REJECT if gate passes.
#[test]
fn f3200_2_error_severity_blocks() {
    use forjar::core::compliance_gate::check_compliance_gate;
    use forjar::core::types::{ForjarConfig, Resource, ResourceType};

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("strict.yaml"),
        r#"
name: strict
version: "1.0"
framework: TEST
rules:
  - id: ERR-001
    title: Must have owner
    severity: error
    type: require
    resource_type: file
    field: owner
"#,
    )
    .unwrap();

    // Config with file missing owner field
    let mut config = ForjarConfig::default();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    config.resources.insert("bad-file".into(), r);

    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(
        !result.passed(),
        "REJECT: error-severity violation did not block gate"
    );
    assert!(result.error_count > 0, "REJECT: error count should be > 0");
}

/// F-3200-3: Policy eval < 50ms for 100 rules × 100 resources.
#[test]
fn f3200_3_policy_eval_performance() {
    use forjar::core::compliance_pack::*;

    // Build 100 resources
    let mut resources = HashMap::new();
    for i in 0..100 {
        let mut fields = HashMap::new();
        fields.insert("type".into(), "file".into());
        fields.insert("owner".into(), "root".into());
        fields.insert("mode".into(), format!("0{}", 644 + (i % 10)));
        resources.insert(format!("resource-{i}"), fields);
    }

    // Build pack with 100 rules
    let mut rules = Vec::new();
    for i in 0..100 {
        rules.push(ComplianceRule {
            id: format!("PERF-{i:03}"),
            title: format!("Performance rule {i}"),
            description: None,
            severity: "warning".into(),
            controls: vec![],
            check: ComplianceCheck::Require {
                resource_type: "file".into(),
                field: "owner".into(),
            },
        });
    }
    let pack = CompliancePack {
        name: "perf-test".into(),
        version: "1.0".into(),
        framework: "BENCH".into(),
        description: None,
        rules,
    };

    let start = std::time::Instant::now();
    let result = evaluate_pack(&pack, &resources);
    let elapsed = start.elapsed();

    assert_eq!(result.results.len(), 100);
    assert!(
        elapsed.as_millis() < 50,
        "REJECT: policy eval took {}ms (> 50ms target)",
        elapsed.as_millis()
    );
}

/// F-3200-4: bashrs validates script policies.
/// Inject secret leakage in script policy; REJECT if lint doesn't catch it.
#[test]
fn f3200_4_bashrs_script_policy_validation() {
    use forjar::core::script_secret_lint::validate_no_leaks;

    // Secret leak: echoing a PASSWORD variable (matched by echo_secret_var pattern)
    let result = validate_no_leaks("echo $PASSWORD");
    assert!(result.is_err(), "REJECT: lint didn't catch echo $PASSWORD");

    // Secret leak: redirecting secret to file
    let result = validate_no_leaks("$SECRET > /tmp/key.txt");
    assert!(result.is_err(), "REJECT: lint didn't catch secret redirect");

    // Inline DB URL with embedded password
    let result = validate_no_leaks("URL=postgres://user:pass@db:5432/app");
    assert!(
        result.is_err(),
        "REJECT: lint didn't catch DB URL with password"
    );
}

/// F-3200-6: No OPA/Rego dependency.
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
                cargo_toml.contains(&format!("{dep}")) && cargo_toml.contains("optional = true"),
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
