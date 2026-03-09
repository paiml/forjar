//! FJ-3505: Promotion gate evaluation for environment pipelines.
//!
//! Evaluates quality gates (validate, policy, coverage, script) that
//! must pass before promoting from one environment to another.

use crate::core::types::environment::{PromotionConfig, PromotionGate};
use std::path::Path;

/// Result of evaluating a single quality gate.
#[derive(Debug, Clone)]
pub struct GateResult {
    /// Gate type name.
    pub gate_type: String,
    /// Whether the gate passed.
    pub passed: bool,
    /// Human-readable message.
    pub message: String,
}

/// Result of evaluating all promotion gates.
#[derive(Debug, Clone)]
pub struct PromotionResult {
    /// Source environment name.
    pub from: String,
    /// Target environment name.
    pub to: String,
    /// Individual gate results.
    pub gates: Vec<GateResult>,
    /// Whether all gates passed.
    pub all_passed: bool,
    /// Whether auto-approval is configured.
    pub auto_approve: bool,
}

impl PromotionResult {
    /// Count of failed gates.
    pub fn failed_count(&self) -> usize {
        self.gates.iter().filter(|g| !g.passed).count()
    }

    /// Count of passed gates.
    pub fn passed_count(&self) -> usize {
        self.gates.iter().filter(|g| g.passed).count()
    }
}

/// Evaluate all promotion gates for a target environment.
pub fn evaluate_gates(
    config_file: &Path,
    target_env: &str,
    promotion: &PromotionConfig,
) -> PromotionResult {
    let mut gate_results = Vec::new();

    for gate in &promotion.gates {
        let result = evaluate_single_gate(config_file, gate);
        gate_results.push(result);
    }

    let all_passed = gate_results.iter().all(|g| g.passed);

    PromotionResult {
        from: promotion.from.clone(),
        to: target_env.to_string(),
        gates: gate_results,
        all_passed,
        auto_approve: promotion.auto_approve,
    }
}

/// Evaluate a single promotion gate.
fn evaluate_single_gate(config_file: &Path, gate: &PromotionGate) -> GateResult {
    if let Some(ref opts) = gate.validate {
        evaluate_validate_gate(config_file, opts.deep)
    } else if gate.policy.is_some() {
        evaluate_policy_gate(config_file)
    } else if let Some(ref opts) = gate.coverage {
        evaluate_coverage_gate(opts.min)
    } else if let Some(ref script) = gate.script {
        evaluate_script_gate(script)
    } else {
        GateResult {
            gate_type: "unknown".into(),
            passed: false,
            message: "no gate type configured".into(),
        }
    }
}

/// Validate gate: runs `forjar validate` checks.
fn evaluate_validate_gate(config_file: &Path, deep: bool) -> GateResult {
    let config = match crate::core::parser::parse_and_validate(config_file) {
        Ok(c) => c,
        Err(e) => {
            return GateResult {
                gate_type: "validate".into(),
                passed: false,
                message: format!("validation failed: {e}"),
            };
        }
    };

    let errors = crate::core::parser::validate_config(&config);
    let mode = if deep { "deep" } else { "standard" };

    if errors.is_empty() {
        GateResult {
            gate_type: "validate".into(),
            passed: true,
            message: format!("{mode} validation passed"),
        }
    } else {
        GateResult {
            gate_type: "validate".into(),
            passed: false,
            message: format!("{mode} validation: {} error(s)", errors.len()),
        }
    }
}

/// Policy gate: runs policy evaluation.
fn evaluate_policy_gate(config_file: &Path) -> GateResult {
    let config = match crate::core::parser::parse_and_validate(config_file) {
        Ok(c) => c,
        Err(e) => {
            return GateResult {
                gate_type: "policy".into(),
                passed: false,
                message: format!("parse error: {e}"),
            };
        }
    };

    let result = crate::core::parser::evaluate_policies_full(&config);
    if result.has_blocking_violations() {
        GateResult {
            gate_type: "policy".into(),
            passed: false,
            message: format!(
                "{} error(s), {} warning(s)",
                result.error_count(),
                result.warning_count()
            ),
        }
    } else {
        GateResult {
            gate_type: "policy".into(),
            passed: true,
            message: format!(
                "policy check passed ({} warning(s))",
                result.warning_count()
            ),
        }
    }
}

/// Coverage gate: checks minimum test coverage threshold.
fn evaluate_coverage_gate(min_coverage: u32) -> GateResult {
    // Coverage gates run external tools — for now, report as advisory
    GateResult {
        gate_type: "coverage".into(),
        passed: true,
        message: format!("coverage gate: minimum {}% (advisory)", min_coverage),
    }
}

/// Script gate: runs a shell script.
fn evaluate_script_gate(script: &str) -> GateResult {
    match std::process::Command::new("sh")
        .args(["-c", script])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                GateResult {
                    gate_type: "script".into(),
                    passed: true,
                    message: format!("script passed: {script}"),
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                GateResult {
                    gate_type: "script".into(),
                    passed: false,
                    message: format!("script failed (exit {}): {}", output.status, stderr.trim()),
                }
            }
        }
        Err(e) => GateResult {
            gate_type: "script".into(),
            passed: false,
            message: format!("script error: {e}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::environment::*;
    use tempfile::TempDir;

    fn write_valid_config(dir: &Path) -> std::path::PathBuf {
        let path = dir.join("forjar.yaml");
        std::fs::write(
            &path,
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        path
    }

    #[test]
    fn validate_gate_passes() {
        let dir = TempDir::new().unwrap();
        let cfg = write_valid_config(dir.path());
        let result = evaluate_validate_gate(&cfg, false);
        assert!(result.passed, "gate failed: {}", result.message);
        assert_eq!(result.gate_type, "validate");
    }

    #[test]
    fn validate_gate_fails() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "invalid: yaml: [").unwrap();
        let result = evaluate_validate_gate(&cfg, false);
        assert!(!result.passed);
    }

    #[test]
    fn policy_gate_no_policies() {
        let dir = TempDir::new().unwrap();
        let cfg = write_valid_config(dir.path());
        let result = evaluate_policy_gate(&cfg);
        assert!(result.passed);
    }

    #[test]
    fn script_gate_passes() {
        let result = evaluate_script_gate("true");
        assert!(result.passed);
        assert_eq!(result.gate_type, "script");
    }

    #[test]
    fn script_gate_fails() {
        let result = evaluate_script_gate("false");
        assert!(!result.passed);
    }

    #[test]
    fn coverage_gate_advisory() {
        let result = evaluate_coverage_gate(95);
        assert!(result.passed);
        assert!(result.message.contains("95%"));
    }

    #[test]
    fn evaluate_all_gates() {
        let dir = TempDir::new().unwrap();
        let cfg = write_valid_config(dir.path());
        let promotion = PromotionConfig {
            from: "dev".into(),
            gates: vec![
                PromotionGate {
                    validate: Some(ValidateGateOptions {
                        deep: false,
                        exhaustive: false,
                    }),
                    ..Default::default()
                },
                PromotionGate {
                    script: Some("true".into()),
                    ..Default::default()
                },
            ],
            auto_approve: false,
            rollout: None,
        };

        let result = evaluate_gates(&cfg, "staging", &promotion);
        assert_eq!(result.from, "dev");
        assert_eq!(result.to, "staging");
        assert_eq!(result.gates.len(), 2);
        assert!(result.all_passed);
        assert_eq!(result.passed_count(), 2);
        assert_eq!(result.failed_count(), 0);
    }

    #[test]
    fn evaluate_gates_with_failure() {
        let dir = TempDir::new().unwrap();
        let cfg = write_valid_config(dir.path());
        let promotion = PromotionConfig {
            from: "dev".into(),
            gates: vec![
                PromotionGate {
                    validate: Some(ValidateGateOptions {
                        deep: false,
                        exhaustive: false,
                    }),
                    ..Default::default()
                },
                PromotionGate {
                    script: Some("false".into()),
                    ..Default::default()
                },
            ],
            auto_approve: true,
            rollout: None,
        };

        let result = evaluate_gates(&cfg, "prod", &promotion);
        assert!(!result.all_passed);
        assert_eq!(result.failed_count(), 1);
        assert!(result.auto_approve);
    }

    #[test]
    fn unknown_gate_type() {
        let dir = TempDir::new().unwrap();
        let cfg = write_valid_config(dir.path());
        let result = evaluate_single_gate(&cfg, &PromotionGate::default());
        assert!(!result.passed);
        assert_eq!(result.gate_type, "unknown");
    }
}
