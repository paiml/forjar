//! FJ-3500: Environment resolution, diffing, and promotion gates.
//!
//! Popperian rejection criteria for:
//! - FJ-3501: resolve_env_params (merge, override, disjoint, empty)
//! - FJ-3501: resolve_env_machines (addr override, unknown machine, empty)
//! - FJ-3502: env_state_dir path construction
//! - FJ-3504: diff_environments (identical, param diff, machine diff, both,
//!   one-sided params, serde roundtrip)
//! - PromotionGate::gate_type (validate, policy, coverage, script, unknown)
//! - EnvironmentDiff::total_diffs, is_identical
//!
//! Usage: cargo test --test falsification_env_promotion

use forjar::core::types::environment::*;
use forjar::core::types::Machine;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// Helpers
// ============================================================================

fn base_params() -> HashMap<String, serde_yaml_ng::Value> {
    let mut m = HashMap::new();
    m.insert(
        "region".into(),
        serde_yaml_ng::Value::String("us-east-1".into()),
    );
    m.insert(
        "tier".into(),
        serde_yaml_ng::Value::String("standard".into()),
    );
    m
}

fn base_machines() -> IndexMap<String, Machine> {
    let mut m = IndexMap::new();
    m.insert("web1".into(), Machine::ssh("web1", "10.0.0.1", "deploy"));
    m.insert("db1".into(), Machine::ssh("db1", "10.0.0.2", "deploy"));
    m
}

fn staging_env() -> Environment {
    let mut params = HashMap::new();
    params.insert(
        "tier".into(),
        serde_yaml_ng::Value::String("staging".into()),
    );
    let mut machines = IndexMap::new();
    machines.insert(
        "web1".into(),
        MachineOverride {
            addr: "10.1.0.1".into(),
        },
    );
    Environment {
        description: Some("Staging environment".into()),
        params,
        machines,
        promotion: None,
    }
}

fn prod_env() -> Environment {
    let mut params = HashMap::new();
    params.insert(
        "tier".into(),
        serde_yaml_ng::Value::String("production".into()),
    );
    params.insert("replicas".into(), serde_yaml_ng::Value::Number(3.into()));
    let mut machines = IndexMap::new();
    machines.insert(
        "web1".into(),
        MachineOverride {
            addr: "10.2.0.1".into(),
        },
    );
    machines.insert(
        "db1".into(),
        MachineOverride {
            addr: "10.2.0.2".into(),
        },
    );
    Environment {
        description: Some("Production environment".into()),
        params,
        machines,
        promotion: None,
    }
}

// ============================================================================
// FJ-3501: resolve_env_params
// ============================================================================

#[test]
fn env_params_merge_override() {
    let merged = resolve_env_params(&base_params(), &staging_env());
    assert_eq!(
        merged["region"],
        serde_yaml_ng::Value::String("us-east-1".into())
    );
    assert_eq!(
        merged["tier"],
        serde_yaml_ng::Value::String("staging".into())
    );
}

#[test]
fn env_params_adds_new_keys() {
    let merged = resolve_env_params(&base_params(), &prod_env());
    assert_eq!(merged["replicas"], serde_yaml_ng::Value::Number(3.into()));
    assert_eq!(merged.len(), 3); // region + tier + replicas
}

#[test]
fn env_params_empty_env() {
    let empty = Environment::default();
    let merged = resolve_env_params(&base_params(), &empty);
    assert_eq!(merged.len(), 2);
    assert_eq!(
        merged["region"],
        serde_yaml_ng::Value::String("us-east-1".into())
    );
}

#[test]
fn env_params_empty_base() {
    let merged = resolve_env_params(&HashMap::new(), &staging_env());
    assert_eq!(merged.len(), 1);
    assert_eq!(
        merged["tier"],
        serde_yaml_ng::Value::String("staging".into())
    );
}

#[test]
fn env_params_both_empty() {
    let merged = resolve_env_params(&HashMap::new(), &Environment::default());
    assert!(merged.is_empty());
}

// ============================================================================
// FJ-3501: resolve_env_machines
// ============================================================================

#[test]
fn env_machines_override_addr() {
    let merged = resolve_env_machines(&base_machines(), &staging_env());
    assert_eq!(merged["web1"].addr, "10.1.0.1");
    assert_eq!(merged["db1"].addr, "10.0.0.2"); // unchanged
}

#[test]
fn env_machines_override_all() {
    let merged = resolve_env_machines(&base_machines(), &prod_env());
    assert_eq!(merged["web1"].addr, "10.2.0.1");
    assert_eq!(merged["db1"].addr, "10.2.0.2");
}

#[test]
fn env_machines_unknown_override_ignored() {
    let mut env = Environment::default();
    env.machines.insert(
        "unknown_host".into(),
        MachineOverride {
            addr: "1.2.3.4".into(),
        },
    );
    let merged = resolve_env_machines(&base_machines(), &env);
    assert_eq!(merged.len(), 2); // no new machine added
    assert_eq!(merged["web1"].addr, "10.0.0.1"); // unchanged
}

#[test]
fn env_machines_empty_env() {
    let merged = resolve_env_machines(&base_machines(), &Environment::default());
    assert_eq!(merged.len(), 2);
    assert_eq!(merged["web1"].addr, "10.0.0.1");
}

#[test]
fn env_machines_preserves_user() {
    let merged = resolve_env_machines(&base_machines(), &staging_env());
    assert_eq!(merged["web1"].user, "deploy");
}

// ============================================================================
// FJ-3502: env_state_dir
// ============================================================================

#[test]
fn state_dir_basic() {
    let dir = env_state_dir(Path::new("/var/forjar/state"), "production");
    assert_eq!(dir, Path::new("/var/forjar/state/production"));
}

#[test]
fn state_dir_nested() {
    let dir = env_state_dir(Path::new("/state"), "us-east/prod");
    assert_eq!(dir, Path::new("/state/us-east/prod"));
}

// ============================================================================
// FJ-3504: diff_environments
// ============================================================================

#[test]
fn diff_identical_envs() {
    let env = staging_env();
    let diff = diff_environments("a", &env, "b", &env, &base_params(), &base_machines());
    assert!(diff.is_identical());
    assert_eq!(diff.total_diffs(), 0);
}

#[test]
fn diff_param_changes() {
    let diff = diff_environments(
        "staging",
        &staging_env(),
        "prod",
        &prod_env(),
        &base_params(),
        &base_machines(),
    );
    assert!(!diff.is_identical());
    // tier differs (staging vs production) + replicas only in prod
    assert!(diff.param_diffs.iter().any(|d| d.key == "tier"));
    assert!(diff.param_diffs.iter().any(|d| d.key == "replicas"));
}

#[test]
fn diff_machine_changes() {
    let diff = diff_environments(
        "staging",
        &staging_env(),
        "prod",
        &prod_env(),
        &base_params(),
        &base_machines(),
    );
    // web1: 10.1.0.1 vs 10.2.0.1; db1: 10.0.0.2 vs 10.2.0.2
    assert!(diff.machine_diffs.iter().any(|d| d.machine == "web1"));
    assert!(diff.machine_diffs.iter().any(|d| d.machine == "db1"));
}

#[test]
fn diff_names_populated() {
    let diff = diff_environments(
        "staging",
        &staging_env(),
        "prod",
        &prod_env(),
        &base_params(),
        &base_machines(),
    );
    assert_eq!(diff.source, "staging");
    assert_eq!(diff.target, "prod");
}

#[test]
fn diff_total_counts() {
    let diff = diff_environments(
        "staging",
        &staging_env(),
        "prod",
        &prod_env(),
        &base_params(),
        &base_machines(),
    );
    assert_eq!(
        diff.total_diffs(),
        diff.param_diffs.len() + diff.machine_diffs.len()
    );
    assert!(diff.total_diffs() >= 3); // at least tier + replicas + web1 or db1
}

#[test]
fn diff_empty_envs() {
    let e = Environment::default();
    let diff = diff_environments("a", &e, "b", &e, &HashMap::new(), &IndexMap::new());
    assert!(diff.is_identical());
    assert_eq!(diff.total_diffs(), 0);
}

#[test]
fn diff_serde_roundtrip() {
    let diff = diff_environments(
        "staging",
        &staging_env(),
        "prod",
        &prod_env(),
        &base_params(),
        &base_machines(),
    );
    let json = serde_json::to_string(&diff).unwrap();
    let parsed: EnvironmentDiff = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.source, diff.source);
    assert_eq!(parsed.target, diff.target);
    assert_eq!(parsed.total_diffs(), diff.total_diffs());
}

// ============================================================================
// PromotionGate::gate_type
// ============================================================================

#[test]
fn gate_type_validate() {
    let gate = PromotionGate {
        validate: Some(ValidateGateOptions {
            deep: true,
            exhaustive: false,
        }),
        ..Default::default()
    };
    assert_eq!(gate.gate_type(), "validate");
}

#[test]
fn gate_type_policy() {
    let gate = PromotionGate {
        policy: Some(PolicyGateOptions { strict: true }),
        ..Default::default()
    };
    assert_eq!(gate.gate_type(), "policy");
}

#[test]
fn gate_type_coverage() {
    let gate = PromotionGate {
        coverage: Some(CoverageGateOptions { min: 90 }),
        ..Default::default()
    };
    assert_eq!(gate.gate_type(), "coverage");
}

#[test]
fn gate_type_script() {
    let gate = PromotionGate {
        script: Some("curl -sf http://localhost/health".into()),
        ..Default::default()
    };
    assert_eq!(gate.gate_type(), "script");
}

#[test]
fn gate_type_unknown() {
    let gate = PromotionGate::default();
    assert_eq!(gate.gate_type(), "unknown");
}

#[test]
fn gate_type_priority_validate_over_policy() {
    let gate = PromotionGate {
        validate: Some(ValidateGateOptions::default()),
        policy: Some(PolicyGateOptions::default()),
        ..Default::default()
    };
    assert_eq!(gate.gate_type(), "validate"); // validate wins
}

// ============================================================================
// Environment serde
// ============================================================================

#[test]
fn environment_default() {
    let env = Environment::default();
    assert!(env.description.is_none());
    assert!(env.params.is_empty());
    assert!(env.machines.is_empty());
    assert!(env.promotion.is_none());
}

#[test]
fn environment_serde_roundtrip() {
    let env = staging_env();
    let json = serde_json::to_string(&env).unwrap();
    let parsed: Environment = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.description, env.description);
    assert_eq!(parsed.params.len(), env.params.len());
}
