//! Tests for FJ-3500 environment types.

use super::environment::*;
use super::Machine;
use indexmap::IndexMap;
use std::collections::HashMap;

#[test]
fn environment_serde_roundtrip() {
    let yaml = r#"
description: "Staging"
params:
  log_level: info
  replicas: 2
machines:
  web:
    addr: staging-web.internal
"#;
    let env: Environment = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(env.description.as_deref(), Some("Staging"));
    assert_eq!(env.params.len(), 2);
    assert_eq!(env.machines.len(), 1);
    assert_eq!(env.machines["web"].addr, "staging-web.internal");
}

#[test]
fn environment_default() {
    let env = Environment::default();
    assert!(env.description.is_none());
    assert!(env.params.is_empty());
    assert!(env.machines.is_empty());
    assert!(env.promotion.is_none());
}

#[test]
fn resolve_params_merges() {
    let mut base = HashMap::new();
    base.insert(
        "log_level".into(),
        serde_yaml_ng::Value::String("debug".into()),
    );
    base.insert("port".into(), serde_yaml_ng::Value::String("8080".into()));

    let mut env = Environment::default();
    env.params.insert(
        "log_level".into(),
        serde_yaml_ng::Value::String("info".into()),
    );

    let merged = resolve_env_params(&base, &env);
    assert_eq!(
        merged["log_level"],
        serde_yaml_ng::Value::String("info".into())
    );
    assert_eq!(merged["port"], serde_yaml_ng::Value::String("8080".into()));
}

#[test]
fn resolve_machines_overrides_addr() {
    let mut base = IndexMap::new();
    base.insert("web".into(), Machine::ssh("web-01", "10.0.0.1", "root"));
    base.insert("db".into(), Machine::ssh("db-01", "10.0.0.2", "root"));

    let mut env = Environment::default();
    env.machines.insert(
        "web".into(),
        MachineOverride {
            addr: "staging-web.internal".into(),
        },
    );

    let merged = resolve_env_machines(&base, &env);
    assert_eq!(merged["web"].addr, "staging-web.internal");
    assert_eq!(merged["db"].addr, "10.0.0.2"); // unchanged
}

#[test]
fn env_state_dir_construction() {
    let base = std::path::Path::new("/var/forjar/state");
    assert_eq!(
        env_state_dir(base, "staging"),
        std::path::PathBuf::from("/var/forjar/state/staging")
    );
}

#[test]
fn diff_environments_detects_param_change() {
    let mut base = HashMap::new();
    base.insert("port".into(), serde_yaml_ng::Value::String("8080".into()));

    let mut dev = Environment::default();
    dev.params.insert(
        "log_level".into(),
        serde_yaml_ng::Value::String("debug".into()),
    );

    let mut staging = Environment::default();
    staging.params.insert(
        "log_level".into(),
        serde_yaml_ng::Value::String("info".into()),
    );

    let base_machines = IndexMap::new();
    let diff = diff_environments("dev", &dev, "staging", &staging, &base, &base_machines);

    assert_eq!(diff.source, "dev");
    assert_eq!(diff.target, "staging");
    assert!(!diff.is_identical());
    assert_eq!(diff.param_diffs.len(), 1);
    assert_eq!(diff.param_diffs[0].key, "log_level");
}

#[test]
fn diff_environments_detects_machine_change() {
    let base_params = HashMap::new();
    let mut base_machines = IndexMap::new();
    base_machines.insert("web".into(), Machine::ssh("web-01", "10.0.0.1", "root"));

    let mut dev = Environment::default();
    dev.machines.insert(
        "web".into(),
        MachineOverride {
            addr: "dev-web.internal".into(),
        },
    );

    let mut staging = Environment::default();
    staging.machines.insert(
        "web".into(),
        MachineOverride {
            addr: "staging-web.internal".into(),
        },
    );

    let diff = diff_environments(
        "dev",
        &dev,
        "staging",
        &staging,
        &base_params,
        &base_machines,
    );
    assert_eq!(diff.machine_diffs.len(), 1);
    assert_eq!(diff.machine_diffs[0].machine, "web");
}

#[test]
fn diff_identical_environments() {
    let base_params = HashMap::new();
    let base_machines = IndexMap::new();
    let env = Environment::default();

    let diff = diff_environments("a", &env, "b", &env, &base_params, &base_machines);
    assert!(diff.is_identical());
    assert_eq!(diff.total_diffs(), 0);
}

#[test]
fn promotion_config_serde() {
    let yaml = r#"
from: dev
auto_approve: true
gates:
  - script: "curl -sf http://localhost/health"
"#;
    let config: PromotionConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.from, "dev");
    assert!(config.auto_approve);
    assert_eq!(config.gates.len(), 1);
    assert_eq!(config.gates[0].gate_type(), "script");
    assert_eq!(
        config.gates[0].script.as_deref(),
        Some("curl -sf http://localhost/health")
    );
}

#[test]
fn rollout_config_defaults() {
    let yaml = "strategy: canary\n";
    let config: RolloutConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.strategy, "canary");
    assert_eq!(config.canary_count, 1);
    assert!(config.health_check.is_none());
}

#[test]
fn environment_diff_total_diffs() {
    let diff = EnvironmentDiff {
        source: "a".into(),
        target: "b".into(),
        param_diffs: vec![ParamDiff {
            key: "k".into(),
            source_value: Some("1".into()),
            target_value: Some("2".into()),
        }],
        machine_diffs: vec![MachineDiff {
            machine: "m".into(),
            source_addr: Some("a".into()),
            target_addr: Some("b".into()),
        }],
    };
    assert_eq!(diff.total_diffs(), 2);
    assert!(!diff.is_identical());
}

#[test]
fn promotion_gate_types() {
    let gate = PromotionGate {
        validate: Some(ValidateGateOptions {
            deep: true,
            exhaustive: false,
        }),
        ..Default::default()
    };
    assert_eq!(gate.gate_type(), "validate");

    let gate = PromotionGate {
        script: Some("echo ok".into()),
        ..Default::default()
    };
    assert_eq!(gate.gate_type(), "script");
}

#[test]
fn full_environment_config_parse() {
    let yaml = r#"
version: "1.0"
name: test
environments:
  dev:
    description: "Dev"
    params:
      port: 8080
  prod:
    description: "Prod"
    params:
      port: 443
    promotion:
      from: dev
      auto_approve: false
"#;
    let config: super::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.environments.len(), 2);
    assert!(config.environments["prod"].promotion.is_some());
    assert_eq!(
        config.environments["prod"].promotion.as_ref().unwrap().from,
        "dev"
    );
}
