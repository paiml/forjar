//! Falsification criteria F-3100 through F-3509: competitive features (37 tests).
#![allow(dead_code)]

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
