//! FJ-3500: Environment types for promotion pipelines.
//!
//! Environments provide dev/staging/prod abstractions over a single
//! forjar.yaml config. Each environment overrides params and machine
//! addresses while sharing the base resource definitions.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An environment definition within the `environments:` config block.
///
/// Environments override params and machine addresses from the base config.
/// State is isolated per-environment in `state/<env-name>/`.
///
/// # Examples
///
/// ```
/// use forjar::core::types::Environment;
///
/// let yaml = r#"
/// description: "Staging environment"
/// params:
///   log_level: info
///   replicas: 2
/// machines:
///   web:
///     addr: staging-web.internal
/// "#;
/// let env: Environment = serde_yaml_ng::from_str(yaml).unwrap();
/// assert_eq!(env.description.as_deref(), Some("Staging environment"));
/// assert_eq!(env.params["log_level"], "info");
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Environment {
    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Parameter overrides for this environment.
    /// These replace base config params with the same key.
    #[serde(default)]
    pub params: HashMap<String, serde_yaml_ng::Value>,

    /// Machine address overrides.
    /// Keys are machine names; values contain the overridden `addr`.
    #[serde(default)]
    pub machines: IndexMap<String, MachineOverride>,

    /// Promotion configuration: how to promote from another environment.
    #[serde(default)]
    pub promotion: Option<PromotionConfig>,
}

/// Machine address override within an environment.
///
/// Only `addr` is overridable per-environment. Other machine fields
/// (user, arch, ssh_key, etc.) inherit from the base config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MachineOverride {
    /// Network address override.
    pub addr: String,
}

/// Promotion configuration for an environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionConfig {
    /// Source environment to promote from.
    pub from: String,

    /// Quality gates that must pass before promotion.
    #[serde(default)]
    pub gates: Vec<PromotionGate>,

    /// Whether to auto-approve after all gates pass (default: false).
    #[serde(default)]
    pub auto_approve: bool,

    /// Progressive rollout configuration.
    #[serde(default)]
    pub rollout: Option<RolloutConfig>,
}

/// A quality gate for environment promotion.
///
/// Deserializes from YAML maps like `- validate: { deep: true }` or
/// `- script: "curl -sf http://localhost/health"`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromotionGate {
    /// Run `forjar validate` with options.
    #[serde(default)]
    pub validate: Option<ValidateGateOptions>,
    /// Run `forjar policy` check.
    #[serde(default)]
    pub policy: Option<PolicyGateOptions>,
    /// Check test coverage meets threshold.
    #[serde(default)]
    pub coverage: Option<CoverageGateOptions>,
    /// Run a custom script (must exit 0).
    #[serde(default)]
    pub script: Option<String>,
}

impl PromotionGate {
    /// Which gate type is configured.
    pub fn gate_type(&self) -> &str {
        if self.validate.is_some() {
            "validate"
        } else if self.policy.is_some() {
            "policy"
        } else if self.coverage.is_some() {
            "coverage"
        } else if self.script.is_some() {
            "script"
        } else {
            "unknown"
        }
    }
}

/// Options for the validate quality gate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidateGateOptions {
    /// Run deep validation.
    #[serde(default)]
    pub deep: bool,
    /// Run exhaustive validation.
    #[serde(default)]
    pub exhaustive: bool,
}

/// Options for the policy quality gate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyGateOptions {
    /// Strict mode: all warnings are errors.
    #[serde(default)]
    pub strict: bool,
}

/// Options for the coverage quality gate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageGateOptions {
    /// Minimum coverage percentage.
    #[serde(default)]
    pub min: u32,
}

/// Progressive rollout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutConfig {
    /// Rollout strategy: "canary", "percentage", "all-at-once".
    #[serde(default = "default_strategy")]
    pub strategy: String,

    /// Number of machines in canary wave.
    #[serde(default = "default_canary_count")]
    pub canary_count: usize,

    /// Health check command template.
    #[serde(default)]
    pub health_check: Option<String>,

    /// Health check timeout (e.g., "30s").
    #[serde(default)]
    pub health_timeout: Option<String>,

    /// Percentage steps for progressive rollout.
    #[serde(default)]
    pub percentage_steps: Vec<u32>,
}

fn default_strategy() -> String {
    "canary".to_string()
}

fn default_canary_count() -> usize {
    1
}

/// Result of comparing two environments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentDiff {
    /// Source environment name.
    pub source: String,
    /// Target environment name.
    pub target: String,
    /// Param differences.
    pub param_diffs: Vec<ParamDiff>,
    /// Machine address differences.
    pub machine_diffs: Vec<MachineDiff>,
}

impl EnvironmentDiff {
    /// Total number of differences.
    pub fn total_diffs(&self) -> usize {
        self.param_diffs.len() + self.machine_diffs.len()
    }

    /// Whether the environments are identical.
    pub fn is_identical(&self) -> bool {
        self.param_diffs.is_empty() && self.machine_diffs.is_empty()
    }
}

/// A parameter difference between two environments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDiff {
    /// Parameter name.
    pub key: String,
    /// Value in source environment.
    pub source_value: Option<String>,
    /// Value in target environment.
    pub target_value: Option<String>,
}

/// A machine address difference between two environments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineDiff {
    /// Machine name.
    pub machine: String,
    /// Address in source environment.
    pub source_addr: Option<String>,
    /// Address in target environment.
    pub target_addr: Option<String>,
}

/// FJ-3502: Derive state directory for an environment.
pub fn env_state_dir(base_state_dir: &std::path::Path, env_name: &str) -> std::path::PathBuf {
    base_state_dir.join(env_name)
}

/// FJ-3501: Resolve an environment's effective params by merging overrides.
pub fn resolve_env_params(
    base_params: &HashMap<String, serde_yaml_ng::Value>,
    env: &Environment,
) -> HashMap<String, serde_yaml_ng::Value> {
    let mut merged = base_params.clone();
    for (k, v) in &env.params {
        merged.insert(k.clone(), v.clone());
    }
    merged
}

/// FJ-3501: Resolve an environment's effective machine addresses.
pub fn resolve_env_machines(
    base_machines: &IndexMap<String, super::Machine>,
    env: &Environment,
) -> IndexMap<String, super::Machine> {
    let mut merged = base_machines.clone();
    for (name, override_cfg) in &env.machines {
        if let Some(m) = merged.get_mut(name) {
            m.addr = override_cfg.addr.clone();
        }
    }
    merged
}

/// FJ-3504: Diff two environments' effective params and machines.
pub fn diff_environments(
    source_name: &str,
    source_env: &Environment,
    target_name: &str,
    target_env: &Environment,
    base_params: &HashMap<String, serde_yaml_ng::Value>,
    base_machines: &IndexMap<String, super::Machine>,
) -> EnvironmentDiff {
    let src_params = resolve_env_params(base_params, source_env);
    let tgt_params = resolve_env_params(base_params, target_env);
    let src_machines = resolve_env_machines(base_machines, source_env);
    let tgt_machines = resolve_env_machines(base_machines, target_env);

    let mut param_diffs = Vec::new();
    let all_keys: std::collections::HashSet<&String> =
        src_params.keys().chain(tgt_params.keys()).collect();
    for key in all_keys {
        let sv = src_params.get(key).map(|v| format!("{v:?}"));
        let tv = tgt_params.get(key).map(|v| format!("{v:?}"));
        if sv != tv {
            param_diffs.push(ParamDiff {
                key: key.clone(),
                source_value: sv,
                target_value: tv,
            });
        }
    }

    let mut machine_diffs = Vec::new();
    let all_machines: std::collections::HashSet<&String> =
        src_machines.keys().chain(tgt_machines.keys()).collect();
    for name in all_machines {
        let sa = src_machines.get(name).map(|m| m.addr.clone());
        let ta = tgt_machines.get(name).map(|m| m.addr.clone());
        if sa != ta {
            machine_diffs.push(MachineDiff {
                machine: name.clone(),
                source_addr: sa,
                target_addr: ta,
            });
        }
    }

    EnvironmentDiff {
        source: source_name.to_string(),
        target: target_name.to_string(),
        param_diffs,
        machine_diffs,
    }
}
