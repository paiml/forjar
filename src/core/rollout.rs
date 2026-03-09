//! FJ-3507: Progressive rollout executor.
//!
//! Implements canary and percentage-based rollout strategies with
//! health checks and auto-rollback.

use crate::core::types::environment::RolloutConfig;

/// A single rollout step.
#[derive(Debug, Clone)]
pub struct RolloutStep {
    /// Step index (0-based).
    pub index: usize,
    /// Percentage of machines to include.
    pub percentage: u32,
    /// Machine indices selected for this step.
    pub machine_indices: Vec<usize>,
    /// Whether the health check passed for this step.
    pub health_passed: bool,
    /// Health check output or error.
    pub message: String,
}

/// Result of a complete rollout execution.
#[derive(Debug, Clone)]
pub struct RolloutResult {
    /// Strategy used.
    pub strategy: String,
    /// Steps executed.
    pub steps: Vec<RolloutStep>,
    /// Whether the rollout completed successfully.
    pub completed: bool,
    /// Step index where rollback was triggered (if any).
    pub rollback_at: Option<usize>,
}

impl RolloutResult {
    /// Total machines deployed across all passed steps.
    pub fn deployed_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.health_passed)
            .flat_map(|s| &s.machine_indices)
            .collect::<std::collections::HashSet<_>>()
            .len()
    }
}

/// Plan rollout steps from a RolloutConfig and total machine count.
pub fn plan_rollout(config: &RolloutConfig, total_machines: usize) -> Vec<RolloutStep> {
    if total_machines == 0 {
        return vec![];
    }

    match config.strategy.as_str() {
        "canary" => plan_canary(config, total_machines),
        "percentage" => plan_percentage(config, total_machines),
        _ => plan_all_at_once(total_machines),
    }
}

/// Canary strategy: deploy to canary_count machines first, then percentage steps.
fn plan_canary(config: &RolloutConfig, total: usize) -> Vec<RolloutStep> {
    let mut steps = Vec::new();
    let canary = config.canary_count.min(total);

    // Step 0: canary machines
    let canary_pct = ((canary as f64 / total as f64) * 100.0).ceil() as u32;
    steps.push(RolloutStep {
        index: 0,
        percentage: canary_pct,
        machine_indices: (0..canary).collect(),
        health_passed: false,
        message: String::new(),
    });

    // Remaining steps from percentage_steps
    let remaining = total - canary;
    if remaining > 0 && !config.percentage_steps.is_empty() {
        for (i, &pct) in config.percentage_steps.iter().enumerate() {
            if pct <= canary_pct {
                continue; // Skip steps already covered by canary
            }
            let count = ((pct as f64 / 100.0) * total as f64).ceil() as usize;
            let count = count.min(total);
            steps.push(RolloutStep {
                index: i + 1,
                percentage: pct,
                machine_indices: (0..count).collect(),
                health_passed: false,
                message: String::new(),
            });
        }
    }

    // Ensure we have a 100% step
    if steps.last().is_none_or(|s| s.percentage < 100) {
        steps.push(RolloutStep {
            index: steps.len(),
            percentage: 100,
            machine_indices: (0..total).collect(),
            health_passed: false,
            message: String::new(),
        });
    }

    steps
}

/// Percentage strategy: deploy in percentage steps.
fn plan_percentage(config: &RolloutConfig, total: usize) -> Vec<RolloutStep> {
    let steps_pct = if config.percentage_steps.is_empty() {
        vec![25, 50, 75, 100]
    } else {
        config.percentage_steps.clone()
    };

    steps_pct
        .iter()
        .enumerate()
        .map(|(i, &pct)| {
            let count = ((pct as f64 / 100.0) * total as f64).ceil() as usize;
            let count = count.min(total);
            RolloutStep {
                index: i,
                percentage: pct,
                machine_indices: (0..count).collect(),
                health_passed: false,
                message: String::new(),
            }
        })
        .collect()
}

/// All-at-once strategy: deploy to all machines in one step.
fn plan_all_at_once(total: usize) -> Vec<RolloutStep> {
    vec![RolloutStep {
        index: 0,
        percentage: 100,
        machine_indices: (0..total).collect(),
        health_passed: false,
        message: String::new(),
    }]
}

/// Run a health check command and return (passed, message).
pub fn run_health_check(health_check: &str, timeout_str: Option<&str>) -> (bool, String) {
    let timeout_secs = parse_timeout(timeout_str);

    match std::process::Command::new("sh")
        .args(["-c", health_check])
        .output()
    {
        Ok(output) => {
            let _ = timeout_secs; // Advisory for now
            if output.status.success() {
                (true, "health check passed".into())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                (
                    false,
                    format!(
                        "health check failed (exit {}): {}",
                        output.status,
                        stderr.trim()
                    ),
                )
            }
        }
        Err(e) => (false, format!("health check error: {e}")),
    }
}

/// Parse a timeout string like "30s", "5m" to seconds.
fn parse_timeout(timeout_str: Option<&str>) -> u64 {
    let Some(s) = timeout_str else {
        return 30;
    };
    let s = s.trim();
    if let Some(num) = s.strip_suffix('s') {
        num.parse().unwrap_or(30)
    } else if let Some(num) = s.strip_suffix('m') {
        num.parse::<u64>().unwrap_or(1) * 60
    } else {
        s.parse().unwrap_or(30)
    }
}

/// Execute a rollout plan with health checks between steps.
pub fn execute_rollout(
    config: &RolloutConfig,
    total_machines: usize,
    dry_run: bool,
) -> RolloutResult {
    let mut steps = plan_rollout(config, total_machines);
    let mut rollback_at = None;

    for step in &mut steps {
        if dry_run {
            step.health_passed = true;
            step.message = "dry-run: skipped".into();
            continue;
        }

        if let Some(ref hc) = config.health_check {
            let (passed, msg) = run_health_check(hc, config.health_timeout.as_deref());
            step.health_passed = passed;
            step.message = msg;

            if !passed {
                rollback_at = Some(step.index);
                break;
            }
        } else {
            step.health_passed = true;
            step.message = "no health check configured".into();
        }
    }

    let completed = rollback_at.is_none();
    RolloutResult {
        strategy: config.strategy.clone(),
        steps,
        completed,
        rollback_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> RolloutConfig {
        RolloutConfig {
            strategy: "canary".into(),
            canary_count: 1,
            health_check: None,
            health_timeout: None,
            percentage_steps: vec![10, 25, 50, 100],
        }
    }

    #[test]
    fn plan_canary_basic() {
        let config = default_config();
        let steps = plan_rollout(&config, 10);
        assert!(!steps.is_empty());
        assert_eq!(steps[0].machine_indices.len(), 1); // canary=1
        assert_eq!(steps.last().unwrap().percentage, 100);
    }

    #[test]
    fn plan_canary_single_machine() {
        let config = default_config();
        let steps = plan_rollout(&config, 1);
        assert!(!steps.is_empty());
        assert_eq!(steps[0].machine_indices.len(), 1);
    }

    #[test]
    fn plan_percentage() {
        let config = RolloutConfig {
            strategy: "percentage".into(),
            canary_count: 0,
            health_check: None,
            health_timeout: None,
            percentage_steps: vec![25, 50, 100],
        };
        let steps = plan_rollout(&config, 8);
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].percentage, 25);
        assert_eq!(steps[0].machine_indices.len(), 2); // 25% of 8
        assert_eq!(steps[2].percentage, 100);
        assert_eq!(steps[2].machine_indices.len(), 8);
    }

    #[test]
    fn plan_all_at_once() {
        let config = RolloutConfig {
            strategy: "all-at-once".into(),
            canary_count: 0,
            health_check: None,
            health_timeout: None,
            percentage_steps: vec![],
        };
        let steps = plan_rollout(&config, 5);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].percentage, 100);
        assert_eq!(steps[0].machine_indices.len(), 5);
    }

    #[test]
    fn plan_zero_machines() {
        let config = default_config();
        let steps = plan_rollout(&config, 0);
        assert!(steps.is_empty());
    }

    #[test]
    fn plan_percentage_default_steps() {
        let config = RolloutConfig {
            strategy: "percentage".into(),
            canary_count: 0,
            health_check: None,
            health_timeout: None,
            percentage_steps: vec![],
        };
        let steps = plan_rollout(&config, 4);
        assert_eq!(steps.len(), 4); // default: 25, 50, 75, 100
    }

    #[test]
    fn parse_timeout_seconds() {
        assert_eq!(parse_timeout(Some("30s")), 30);
        assert_eq!(parse_timeout(Some("5m")), 300);
        assert_eq!(parse_timeout(Some("60")), 60);
        assert_eq!(parse_timeout(None), 30);
    }

    #[test]
    fn health_check_passes() {
        let (passed, msg) = run_health_check("true", None);
        assert!(passed);
        assert!(msg.contains("passed"));
    }

    #[test]
    fn health_check_fails() {
        let (passed, _msg) = run_health_check("false", None);
        assert!(!passed);
    }

    #[test]
    fn execute_dry_run() {
        let config = default_config();
        let result = execute_rollout(&config, 5, true);
        assert!(result.completed);
        assert!(result.rollback_at.is_none());
        assert!(result.steps.iter().all(|s| s.health_passed));
    }

    #[test]
    fn execute_no_health_check() {
        let config = RolloutConfig {
            strategy: "all-at-once".into(),
            canary_count: 0,
            health_check: None,
            health_timeout: None,
            percentage_steps: vec![],
        };
        let result = execute_rollout(&config, 3, false);
        assert!(result.completed);
    }

    #[test]
    fn execute_with_passing_health() {
        let config = RolloutConfig {
            strategy: "all-at-once".into(),
            canary_count: 0,
            health_check: Some("true".into()),
            health_timeout: Some("10s".into()),
            percentage_steps: vec![],
        };
        let result = execute_rollout(&config, 2, false);
        assert!(result.completed);
        assert_eq!(result.deployed_count(), 2);
    }

    #[test]
    fn execute_with_failing_health() {
        let config = RolloutConfig {
            strategy: "canary".into(),
            canary_count: 1,
            health_check: Some("false".into()),
            health_timeout: None,
            percentage_steps: vec![50, 100],
        };
        let result = execute_rollout(&config, 4, false);
        assert!(!result.completed);
        assert_eq!(result.rollback_at, Some(0));
    }

    #[test]
    fn rollout_result_deployed_count() {
        let result = RolloutResult {
            strategy: "canary".into(),
            steps: vec![
                RolloutStep {
                    index: 0,
                    percentage: 25,
                    machine_indices: vec![0],
                    health_passed: true,
                    message: String::new(),
                },
                RolloutStep {
                    index: 1,
                    percentage: 50,
                    machine_indices: vec![0, 1],
                    health_passed: true,
                    message: String::new(),
                },
            ],
            completed: true,
            rollback_at: None,
        };
        assert_eq!(result.deployed_count(), 2); // deduped: {0, 1}
    }
}
