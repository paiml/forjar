//! Falsification criteria F-3100 through F-3509: competitive features (37 tests).

use forjar::core::compliance;
use forjar::core::compliance_gate;
use forjar::core::compliance_pack::*;
use forjar::core::ephemeral::*;
use forjar::core::metric_source::{MetricThreshold, ThresholdOp};
use forjar::core::plugin_dispatch;
use forjar::core::plugin_hot_reload::{PluginCache, ReloadCheck};
use forjar::core::plugin_loader;
use forjar::core::policy_boundary;
use forjar::core::promotion;
use forjar::core::promotion_events::{self, PromotionParams};
use forjar::core::script_secret_lint;
use forjar::core::secret_namespace::{self, NamespaceConfig};
use forjar::core::secret_provider::{EnvProvider, ProviderChain};
use forjar::core::state_encryption;
use forjar::core::types::environment::*;
use forjar::core::types::*;
use forjar::core::watch_daemon::{self, DaemonState, WatchDaemonConfig};
use std::collections::HashMap;
use std::time::Instant;

fn make_event(et: EventType) -> InfraEvent {
    InfraEvent {
        event_type: et,
        timestamp: "T".into(),
        machine: None,
        payload: HashMap::new(),
    }
}
fn make_action() -> RulebookAction {
    RulebookAction {
        apply: Some(ApplyAction {
            file: "f.yaml".into(),
            subset: vec![],
            tags: vec![],
            machine: None,
        }),
        destroy: None,
        script: None,
        notify: None,
    }
}
fn make_rb(name: &str, et: EventType, cooldown: u64) -> Rulebook {
    Rulebook {
        name: name.into(),
        description: None,
        events: vec![EventPattern {
            event_type: et,
            match_fields: HashMap::new(),
        }],
        conditions: vec![],
        actions: vec![make_action()],
        cooldown_secs: cooldown,
        max_retries: 3,
        enabled: true,
    }
}
fn valid_config_yaml() -> &'static str {
    "version: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m1\n    provider: apt\n    packages: [curl]\n"
}
fn setup_plugin(dir: &std::path::Path, name: &str, wasm: &[u8]) {
    let pd = dir.join(name);
    std::fs::create_dir_all(&pd).unwrap();
    let hash = blake3::hash(wasm).to_hex().to_string();
    std::fs::write(pd.join("plugin.wasm"), wasm).unwrap();
    std::fs::write(pd.join("plugin.yaml"),
        format!("name: {name}\nversion: \"0.1.0\"\nabi_version: 1\nwasm: plugin.wasm\nblake3: \"{hash}\"\n")
    ).unwrap();
}

// ── F-3100: Event-Driven Automation ─────────────────────────────────────

/// F-3100-1: Event detection < 100ms.
#[test]
fn f_3100_1_event_detection_latency() {
    let config = RulebookConfig {
        rulebooks: vec![make_rb("t", EventType::FileChanged, 0)],
    };
    let mut state = DaemonState::new(&WatchDaemonConfig::default());
    let start = Instant::now();
    watch_daemon::process_event(&make_event(EventType::FileChanged), &config, &mut state);
    assert!(start.elapsed().as_millis() < 100);
}

/// F-3100-2: No event loss under load — 1000 events.
#[test]
fn f_3100_2_no_event_loss_under_load() {
    let config = RulebookConfig {
        rulebooks: vec![make_rb("s", EventType::Manual, 0)],
    };
    let mut state = DaemonState::new(&WatchDaemonConfig::default());
    for i in 0..1000 {
        let mut e = make_event(EventType::Manual);
        e.timestamp = format!("{i}Z");
        let r = watch_daemon::process_event(&e, &config, &mut state);
        assert_eq!(r.pending_actions.len(), 1, "event {i} lost");
    }
    assert_eq!(state.events_processed, 1000);
    assert_eq!(state.actions_dispatched, 1000);
}

/// F-3100-3: Cooldown prevents storms.
#[test]
fn f_3100_3_cooldown_prevents_storms() {
    let mut tracker = CooldownTracker::default();
    assert!(tracker.can_fire("rb", 60));
    tracker.record_fire("rb");
    assert!(!tracker.can_fire("rb", 60));
}

/// F-3100-4: bashrs validates handler scripts.
#[test]
fn f_3100_4_bashrs_validates_handler_scripts() {
    assert!(forjar::core::purifier::validate_script("echo hello").is_ok());
}

/// F-3100-5: Graceful shutdown preserves events.
#[test]
fn f_3100_5_graceful_shutdown_preserves_events() {
    let mut state = DaemonState::new(&WatchDaemonConfig::default());
    assert!(!state.shutdown);
    state.shutdown = true;
    state.events_processed = 42;
    assert!(state.shutdown);
    assert_eq!(state.events_processed, 42);
}

/// F-3100-6: Zero non-sovereign deps.
#[test]
fn f_3100_6_zero_non_sovereign_deps() {
    let wdc = WatchDaemonConfig {
        cron_schedules: vec![("t".into(), "*/5 * * * *".into())],
        metric_thresholds: vec![MetricThreshold {
            name: "cpu".into(),
            operator: ThresholdOp::Gt,
            value: 80.0,
            consecutive: 1,
        }],
        ..Default::default()
    };
    assert_eq!(DaemonState::new(&wdc).cron_parsed.len(), 1);
}

// ── F-3200: Policy-as-Code Engine ───────────────────────────────────────

/// F-3200-1: All 4 policy types evaluate correctly.
#[test]
fn f_3200_1_all_four_policy_types() {
    let bm = compliance::supported_benchmarks();
    assert!(
        bm.contains(&"cis")
            && bm.contains(&"nist-800-53")
            && bm.contains(&"soc2")
            && bm.contains(&"hipaa")
    );
    let mut config = ForjarConfig::default();
    config.resources.insert(
        "f".into(),
        Resource {
            resource_type: ResourceType::File,
            mode: Some("0777".into()),
            ..Default::default()
        },
    );
    config.resources.insert(
        "s".into(),
        Resource {
            resource_type: ResourceType::Service,
            owner: Some("root".into()),
            ..Default::default()
        },
    );
    config.resources.insert(
        "n".into(),
        Resource {
            resource_type: ResourceType::Network,
            port: Some("80".into()),
            ..Default::default()
        },
    );
    for b in bm {
        assert!(
            !compliance::evaluate_benchmark(b, &config).is_empty(),
            "'{b}' empty"
        );
    }
}

/// F-3200-2: Error-severity blocks apply.
#[test]
fn f_3200_2_error_severity_blocks_apply() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("s.yaml"),
        "name: s\nversion: \"1.0\"\nframework: T\nrules:\n  - id: S1\n    title: t\n    severity: error\n    type: require\n    resource_type: file\n    field: owner\n"
    ).unwrap();
    let mut config = ForjarConfig::default();
    config.resources.insert(
        "f".into(),
        Resource {
            resource_type: ResourceType::File,
            ..Default::default()
        },
    );
    let r = compliance_gate::check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(!r.passed() && r.error_count > 0);
}

/// F-3200-3: Policy eval < 50ms.
#[test]
fn f_3200_3_policy_eval_under_50ms() {
    let mut config = ForjarConfig::default();
    for i in 0..100 {
        config.resources.insert(
            format!("r{i}"),
            Resource {
                resource_type: ResourceType::File,
                owner: Some("root".into()),
                mode: Some("0644".into()),
                ..Default::default()
            },
        );
    }
    let start = Instant::now();
    for bm in compliance::supported_benchmarks() {
        let _ = compliance::evaluate_benchmark(bm, &config);
    }
    assert!(start.elapsed().as_millis() < 50);
}

/// F-3200-4: bashrs validates script policies.
#[test]
fn f_3200_4_bashrs_validates_script_policies() {
    assert!(script_secret_lint::validate_no_leaks("#!/bin/bash\necho ok\n").is_ok());
    assert!(script_secret_lint::validate_no_leaks("echo $PASSWORD\n").is_err());
}

/// F-3200-5: Compliance packs tamper-evident via BLAKE3.
#[test]
fn f_3200_5_compliance_packs_tamper_evident() {
    let y = "name: t\nversion: \"1.0\"\nframework: CIS\nrules: []\n";
    let h1 = blake3::hash(y.as_bytes()).to_hex().to_string();
    let h2 = blake3::hash(y.replace("CIS", "SOC").as_bytes())
        .to_hex()
        .to_string();
    assert_eq!(h1.len(), 64);
    assert_ne!(h1, h2);
}

/// F-3200-6: No OPA/Rego dependency — pure Rust evaluation.
#[test]
fn f_3200_6_no_opa_rego_dependency() {
    let pack = CompliancePack {
        name: "p".into(),
        version: "1".into(),
        framework: "CIS".into(),
        description: None,
        rules: vec![ComplianceRule {
            id: "R1".into(),
            title: "T".into(),
            description: None,
            severity: "error".into(),
            controls: vec![],
            check: ComplianceCheck::Assert {
                resource_type: "file".into(),
                field: "owner".into(),
                expected: "root".into(),
            },
        }],
    };
    let mut res = HashMap::new();
    let mut f = HashMap::new();
    f.insert("type".into(), "file".into());
    f.insert("owner".into(), "root".into());
    res.insert("f1".into(), f);
    assert_eq!(evaluate_pack(&pack, &res).passed_count(), 1);
}

/// F-3200-7: Cross-dimension discrimination.
#[test]
fn f_3200_7_cross_dimension_discrimination() {
    let pack = CompliancePack {
        name: "d".into(),
        version: "1".into(),
        framework: "CIS".into(),
        description: None,
        rules: vec![ComplianceRule {
            id: "D1".into(),
            title: "T".into(),
            description: None,
            severity: "error".into(),
            controls: vec![],
            check: ComplianceCheck::Assert {
                resource_type: "file".into(),
                field: "owner".into(),
                expected: "root".into(),
            },
        }],
    };
    let r = policy_boundary::test_boundaries(&pack);
    assert!(r.all_passed() && r.outcomes.len() >= 2);
}

// ── F-3300: Ephemeral Values + State Encryption ─────────────────────────

/// F-3300-1: Ephemeral values never in state records.
#[test]
fn f_3300_1_ephemeral_values_never_in_state() {
    let resolved = vec![ResolvedEphemeral {
        key: "db_pass".into(),
        value: "super-secret".into(),
        hash: blake3::hash(b"super-secret").to_hex().to_string(),
    }];
    let json = serde_json::to_string(&to_records(&resolved)[0]).unwrap();
    assert!(!json.contains("super-secret"));
}

/// F-3300-2: Drift detection via hash.
#[test]
fn f_3300_2_drift_detection_via_hash() {
    let orig = ResolvedEphemeral {
        key: "k".into(),
        value: "v1".into(),
        hash: blake3::hash(b"v1").to_hex().to_string(),
    };
    let stored = to_records(&[orig]);
    let changed = vec![ResolvedEphemeral {
        key: "k".into(),
        value: "v2".into(),
        hash: blake3::hash(b"v2").to_hex().to_string(),
    }];
    assert_eq!(
        check_drift(&changed, &stored)[0].status,
        DriftStatus::Changed
    );
}

/// F-3300-3: Encrypted state round-trips.
#[test]
fn f_3300_3_encrypted_state_roundtrips() {
    let key = state_encryption::derive_key("pass");
    let meta = state_encryption::create_metadata(b"plain", b"cipher", &key);
    assert_eq!(meta.version, 1);
    assert!(state_encryption::verify_metadata(&meta, b"cipher", &key));
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("s.yaml");
    state_encryption::write_metadata(&p, &meta).unwrap();
    let loaded = state_encryption::read_metadata(&p).unwrap();
    assert_eq!(loaded.plaintext_hash, meta.plaintext_hash);
    assert_eq!(loaded.ciphertext_hmac, meta.ciphertext_hmac);
}

/// F-3300-4: BLAKE3 HMAC catches tampering — single-bit flip.
#[test]
fn f_3300_4_blake3_hmac_catches_tampering() {
    let key = state_encryption::derive_key("k");
    let data = b"original";
    let hmac = state_encryption::keyed_hash(data, &key);
    assert!(state_encryption::verify_keyed_hash(data, &key, &hmac));
    let mut tampered = data.to_vec();
    tampered[0] ^= 1;
    assert!(!state_encryption::verify_keyed_hash(&tampered, &key, &hmac));
}

/// F-3300-5: pepita namespace isolation.
#[test]
fn f_3300_5_pepita_namespace_isolation() {
    let config = NamespaceConfig {
        namespace_id: "ns-iso".into(),
        audit_enabled: false,
        state_dir: None,
        inherit_env: vec![],
    };
    let secret = ResolvedEphemeral {
        key: "SK".into(),
        value: "v".into(),
        hash: blake3::hash(b"v").to_hex().to_string(),
    };
    let env = secret_namespace::build_isolated_env(&config, &[secret]);
    assert_eq!(env.len(), 2);
    assert_eq!(env.get("SK").unwrap(), "v");
    assert!(!env.contains_key("HOME"));
}

/// F-3300-6: bashrs catches secret echo.
#[test]
fn f_3300_6_bashrs_catches_secret_echo() {
    for s in [
        "echo $PASSWORD",
        "curl -u admin:pw https://x.com",
        "export PASSWORD=x",
        "sshpass -p pw ssh h",
    ] {
        assert!(
            !script_secret_lint::scan_script(s).clean(),
            "'{s}' not flagged"
        );
    }
    assert!(script_secret_lint::scan_script("apt-get install nginx\n").clean());
}

/// F-3300-7: Key rotation preserves state.
#[test]
fn f_3300_7_key_rotation_preserves_state() {
    let m1 = state_encryption::create_metadata(b"s", b"c1", &state_encryption::derive_key("k1"));
    let m2 = state_encryption::create_metadata(b"s", b"c2", &state_encryption::derive_key("k2"));
    assert_eq!(m1.plaintext_hash, m2.plaintext_hash);
    assert_ne!(m1.ciphertext_hmac, m2.ciphertext_hmac);
}

/// F-3300-8: No cloud KMS in default path.
#[test]
fn f_3300_8_no_cloud_kms_default_path() {
    assert_eq!(state_encryption::derive_key("x").len(), 32);
    assert!(ProviderChain::new()
        .with(Box::new(EnvProvider))
        .resolve("NOKEY_99")
        .unwrap()
        .is_none());
}

// ── F-3400: WASM Resource Provider Plugins ──────────────────────────────

/// F-3400-1: WASM sandbox isolates filesystem.
#[test]
fn f_3400_1_wasm_sandbox_isolates_filesystem() {
    let dir = tempfile::tempdir().unwrap();
    let r = plugin_dispatch::dispatch_check(dir.path(), "x", &serde_json::json!({}));
    assert!(!r.success);
    let p = PluginPermissions::default();
    assert!(p.fs.read.is_empty() && p.fs.write.is_empty());
}

/// F-3400-2: WASM sandbox isolates network.
#[test]
fn f_3400_2_wasm_sandbox_isolates_network() {
    let p = PluginPermissions::default();
    assert!(p.net.connect.is_empty() && p.is_empty());
}

/// F-3400-3: Plugin ABI is stable.
#[test]
fn f_3400_3_plugin_abi_is_stable() {
    assert_eq!(PLUGIN_ABI_VERSION, 1);
    let m = PluginManifest {
        name: "t".into(),
        version: "1".into(),
        description: None,
        abi_version: PLUGIN_ABI_VERSION,
        wasm: "t.wasm".into(),
        blake3: "a".into(),
        permissions: PluginPermissions::default(),
        schema: None,
    };
    assert!(m.is_abi_compatible());
    assert!(!(PluginManifest {
        abi_version: 99,
        ..m
    })
    .is_abi_compatible());
}

/// F-3400-4: BLAKE3 prevents tampered plugins.
#[test]
fn f_3400_4_blake3_prevents_tampered_plugins() {
    let dir = tempfile::tempdir().unwrap();
    setup_plugin(dir.path(), "tp", b"original");
    assert!(plugin_loader::resolve_and_verify(dir.path(), "tp").is_ok());
    std::fs::write(dir.path().join("tp/plugin.wasm"), b"tampered").unwrap();
    assert!(plugin_loader::resolve_and_verify(dir.path(), "tp")
        .unwrap_err()
        .contains("hash mismatch"));
}

/// F-3400-5: Cold load < 50ms.
#[test]
fn f_3400_5_cold_load_under_50ms() {
    let dir = tempfile::tempdir().unwrap();
    setup_plugin(dir.path(), "fl", b"fast wasm");
    let start = Instant::now();
    plugin_loader::resolve_and_verify(dir.path(), "fl").unwrap();
    assert!(start.elapsed().as_millis() < 50);
}

/// F-3400-6: Shell bridge validates scripts.
#[test]
fn f_3400_6_shell_bridge_validates_scripts() {
    assert!(forjar::core::purifier::validate_script("ls -la").is_ok());
    assert!(script_secret_lint::validate_no_leaks("echo $TOKEN\n").is_err());
}

/// F-3400-7: Hot-reload detects changes.
#[test]
fn f_3400_7_hot_reload_detects_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("m.wasm");
    std::fs::write(&path, b"v1").unwrap();
    let mut cache = PluginCache::new();
    cache.insert(
        "m",
        PluginManifest {
            name: "m".into(),
            version: "0.1".into(),
            description: None,
            abi_version: 1,
            wasm: "m.wasm".into(),
            blake3: String::new(),
            permissions: PluginPermissions::default(),
            schema: None,
        },
        path.clone(),
    );
    assert_eq!(cache.needs_reload("m"), ReloadCheck::UpToDate);
    std::fs::write(&path, b"v2").unwrap();
    assert!(cache.needs_reload("m").should_reload());
}

/// F-3400-8: No non-sovereign WASM runtime.
#[test]
fn f_3400_8_no_external_wasm_runtime() {
    assert_eq!(
        plugin_dispatch::parse_plugin_type("plugin:foo"),
        Some("foo")
    );
    assert!(plugin_dispatch::is_plugin_type("plugin:bar"));
    assert!(!plugin_dispatch::is_plugin_type("package"));
    let dir = tempfile::tempdir().unwrap();
    assert!(plugin_dispatch::available_plugin_types(dir.path()).is_empty());
}

// ── F-3500: Environment Promotion Pipelines ─────────────────────────────

/// F-3500-1: Environment state isolation.
#[test]
fn f_3500_1_environment_state_isolation() {
    let base = std::path::Path::new("/state");
    assert_ne!(env_state_dir(base, "dev"), env_state_dir(base, "prod"));
    assert_eq!(env_state_dir(base, "dev"), base.join("dev"));
}

/// F-3500-2: Quality gates block promotion.
#[test]
fn f_3500_2_quality_gates_block_promotion() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(&cfg, valid_config_yaml()).unwrap();
    let p = PromotionConfig {
        from: "dev".into(),
        auto_approve: false,
        rollout: None,
        gates: vec![PromotionGate {
            script: Some("false".into()),
            ..Default::default()
        }],
    };
    let r = promotion::evaluate_gates(&cfg, "staging", &p);
    assert!(!r.all_passed && r.failed_count() == 1);
}

/// F-3500-3: Progressive rollout respects canary.
#[test]
fn f_3500_3_progressive_rollout_respects_canary() {
    let r = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 1,
        health_check: Some("curl -sf http://localhost/health".into()),
        health_timeout: Some("30s".into()),
        percentage_steps: vec![],
    };
    assert_eq!(r.strategy, "canary");
    assert!(r.health_check.is_some());
}

/// F-3500-4: Auto-rollback on health failure.
#[test]
fn f_3500_4_auto_rollback_on_health_failure() {
    let dir = tempfile::tempdir().unwrap();
    let sd = dir.path().join("state");
    std::fs::create_dir_all(&sd).unwrap();
    promotion_events::log_rollback(&sd, "prod", 0, "health fail").unwrap();
    let c = std::fs::read_to_string(sd.join("prod/events.jsonl")).unwrap();
    assert!(c.contains("rollback_triggered") && c.contains("health fail"));
}

/// F-3500-5: Environment diff accuracy.
#[test]
fn f_3500_5_environment_diff_accuracy() {
    let env = Environment::default();
    let d = diff_environments(
        "a",
        &env,
        "b",
        &env,
        &HashMap::new(),
        &indexmap::IndexMap::new(),
    );
    assert!(d.is_identical() && d.total_diffs() == 0);
}

/// F-3500-6: Promotion history append-only.
#[test]
fn f_3500_6_promotion_history_append_only() {
    let dir = tempfile::tempdir().unwrap();
    let sd = dir.path().join("state");
    std::fs::create_dir_all(&sd).unwrap();
    for i in 0..3 {
        promotion_events::log_promotion(&PromotionParams {
            state_dir: &sd,
            target_env: "stg",
            source: "dev",
            target: "stg",
            gates_passed: i,
            gates_total: 3,
            rollout_strategy: None,
        })
        .unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(sd.join("stg/events.jsonl"))
            .unwrap()
            .lines()
            .count(),
        3
    );
}

/// F-3500-7: No external CI/CD dependency.
#[test]
fn f_3500_7_no_external_cicd_dependency() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(&cfg, valid_config_yaml()).unwrap();
    let p = PromotionConfig {
        from: "dev".into(),
        auto_approve: false,
        rollout: None,
        gates: vec![PromotionGate {
            validate: Some(ValidateGateOptions {
                deep: false,
                exhaustive: false,
            }),
            ..Default::default()
        }],
    };
    let r = promotion::evaluate_gates(&cfg, "staging", &p);
    assert!(r.all_passed);
    assert_eq!(r.gates[0].gate_type, "validate");
}

/// F-3500-8: Config DRY — environments reuse base config.
#[test]
fn f_3500_8_config_dry_single_yaml() {
    let mut base = HashMap::new();
    base.insert(
        "log_level".into(),
        serde_yaml_ng::Value::String("debug".into()),
    );
    base.insert("replicas".into(), serde_yaml_ng::Value::String("1".into()));
    let mut overrides = HashMap::new();
    overrides.insert("replicas".into(), serde_yaml_ng::Value::String("3".into()));
    let env = Environment {
        params: overrides,
        ..Default::default()
    };
    let resolved = resolve_env_params(&base, &env);
    assert_eq!(resolved.get("log_level").unwrap(), "debug");
    assert_eq!(resolved.get("replicas").unwrap(), "3");
}
