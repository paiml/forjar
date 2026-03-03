//! Execution policy types: Policy, FailurePolicy, NotifyConfig.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::{default_one, default_true};

// ============================================================================
// Policy
// ============================================================================

/// Execution policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Failure handling
    #[serde(default)]
    pub failure: FailurePolicy,

    /// Apply to independent machines concurrently
    #[serde(default)]
    pub parallel_machines: bool,

    /// Enable provenance tracing on every apply
    #[serde(default = "default_true")]
    pub tripwire: bool,

    /// Persist BLAKE3 state after apply
    #[serde(default = "default_true")]
    pub lock_file: bool,

    /// FJ-216: Execute independent resources within a machine concurrently
    #[serde(default)]
    pub parallel_resources: bool,

    /// Command to run locally before apply (exit non-zero aborts)
    #[serde(default)]
    pub pre_apply: Option<String>,

    /// Command to run locally after successful apply
    #[serde(default)]
    pub post_apply: Option<String>,

    /// FJ-222: Rolling deploys — apply to N machines at a time, waiting for
    /// convergence before advancing to the next batch. When combined with
    /// `parallel_machines: true`, `serial` controls the batch size.
    #[serde(default)]
    pub serial: Option<usize>,

    /// FJ-222: Abort rollout if cumulative failure rate exceeds this percentage.
    /// Checked after each batch. Range: 0–100.
    #[serde(default)]
    pub max_fail_percentage: Option<u8>,

    /// FJ-261: SSH retry attempts on transient failures (connection refused, timeout, broken pipe).
    /// Total attempt count: 1 = no retry (default), 3 = up to 3 attempts.
    /// Backoff: 200ms × 2^attempt. Capped at 4 attempts max.
    #[serde(default = "default_one")]
    pub ssh_retries: u32,

    /// FJ-1380: Convergence budget in seconds — warn/fail if apply exceeds this.
    #[serde(default)]
    pub convergence_budget: Option<u64>,

    /// FJ-1381: Number of pre-apply state snapshots to retain (0 = disabled).
    #[serde(default)]
    pub snapshot_generations: Option<u32>,

    /// FJ-225: Notification hooks — shell commands run after apply/drift
    #[serde(default)]
    pub notify: NotifyConfig,
}

/// FJ-225: Notification hooks configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotifyConfig {
    /// Command to run after successful apply.
    /// Template variables: `{{machine}}`, `{{converged}}`, `{{unchanged}}`, `{{failed}}`
    #[serde(default)]
    pub on_success: Option<String>,

    /// Command to run after apply with failures.
    /// Template variables: `{{machine}}`, `{{converged}}`, `{{unchanged}}`, `{{failed}}`
    #[serde(default)]
    pub on_failure: Option<String>,

    /// Command to run when drift is detected.
    /// Template variables: `{{machine}}`, `{{drift_count}}`
    #[serde(default)]
    pub on_drift: Option<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            failure: FailurePolicy::default(),
            parallel_machines: false,
            parallel_resources: false,
            tripwire: true,
            lock_file: true,
            pre_apply: None,
            post_apply: None,
            serial: None,
            max_fail_percentage: None,
            ssh_retries: 1,
            convergence_budget: None,
            snapshot_generations: None,
            notify: NotifyConfig::default(),
        }
    }
}

/// Failure handling strategy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailurePolicy {
    #[default]
    StopOnFirst,
    ContinueIndependent,
}

impl fmt::Display for FailurePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StopOnFirst => write!(f, "stop_on_first"),
            Self::ContinueIndependent => write!(f, "continue_independent"),
        }
    }
}
