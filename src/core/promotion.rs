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

/// Outcome of probing `cargo llvm-cov`.
///
/// Bug-hunt #5 (Refs #154): the old `run_llvm_cov() -> Option<String>` collapsed
/// two very different situations into `None` — (a) the tool is not installed
/// (spawn/ENOENT failure) and (b) the tool ran but exited non-zero because the
/// build or tests actually failed. `evaluate_coverage_gate_inner` then treated
/// any `None` as an advisory PASS, so a broken build silently passed the gate.
/// Keeping them distinct lets the gate advisory-pass only when coverage is
/// genuinely unavailable, and FAIL when the present tool reported failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CovProbe {
    /// Tool could not be spawned (not installed) — advisory pass.
    Unavailable,
    /// Tool ran and exited 0; carries stdout for coverage parsing.
    Ran(String),
    /// Tool ran but exited non-zero (build/test failure) — gate failure.
    Failed { code: Option<i32> },
}

/// Coverage gate: checks minimum test coverage threshold.
///
/// Attempts to parse coverage from `cargo llvm-cov --summary-only`.
/// Falls back to advisory mode if the tool is not available or too slow.
fn evaluate_coverage_gate(min_coverage: u32) -> GateResult {
    evaluate_coverage_gate_inner(min_coverage, run_llvm_cov)
}

/// Inner coverage gate logic, injectable for testing.
fn evaluate_coverage_gate_inner(min_coverage: u32, runner: fn() -> CovProbe) -> GateResult {
    match runner() {
        CovProbe::Ran(stdout) => match parse_coverage_from_output(&stdout) {
            Some(actual) => coverage_threshold_result(actual, min_coverage),
            // Tool ran cleanly but no TOTAL line — treat as advisory.
            None => coverage_advisory_result(min_coverage),
        },
        // Bug-hunt #5 (Refs #154): tool present but exited non-zero ⇒ a real
        // failure signal (broken build / failing tests). Fail the gate instead
        // of vacuously passing it.
        CovProbe::Failed { code } => GateResult {
            gate_type: "coverage".into(),
            passed: false,
            message: match code {
                Some(c) => format!("coverage gate failed: cargo llvm-cov exited with code {c}"),
                None => "coverage gate failed: cargo llvm-cov terminated by signal".into(),
            },
        },
        CovProbe::Unavailable => coverage_advisory_result(min_coverage),
    }
}

/// Build a pass/fail GateResult from a parsed coverage percentage.
fn coverage_threshold_result(actual: f64, min_coverage: u32) -> GateResult {
    if actual >= min_coverage as f64 {
        GateResult {
            gate_type: "coverage".into(),
            passed: true,
            message: format!("coverage {actual:.1}% >= {min_coverage}%"),
        }
    } else {
        GateResult {
            gate_type: "coverage".into(),
            passed: false,
            message: format!("coverage {actual:.1}% < {min_coverage}% required"),
        }
    }
}

/// Advisory pass used only when coverage is genuinely unavailable.
fn coverage_advisory_result(min_coverage: u32) -> GateResult {
    GateResult {
        gate_type: "coverage".into(),
        passed: true,
        message: format!(
            "coverage gate: minimum {min_coverage}% (advisory — llvm-cov not available)"
        ),
    }
}

/// Run `cargo llvm-cov --summary-only`, distinguishing absence from failure.
fn run_llvm_cov() -> CovProbe {
    classify_llvm_cov(
        std::process::Command::new("cargo")
            .args(["llvm-cov", "--summary-only"])
            .output(),
    )
}

/// Map a spawn `Result` into a [`CovProbe`] by decomposing the `Output` and
/// delegating the actual decision to the pure [`classify_exit`] helper.
fn classify_llvm_cov(spawn: std::io::Result<std::process::Output>) -> CovProbe {
    match spawn {
        Err(_) => CovProbe::Unavailable,
        Ok(output) => classify_exit(
            output.status.success(),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).to_string(),
        ),
    }
}

/// Pure spawn-vs-exit decision over an already-spawned process's result.
///
/// Bug-hunt #5 (Refs #154): factored out so it can be unit-tested without
/// invoking any tool or spawning a subprocess (hermetic, no loopback shell).
/// A clean exit (`success`) yields `Ran(stdout)` for coverage parsing; a
/// present tool that exited non-zero yields `Failed` (gate failure). Spawn
/// failure (tool absent) is handled by the caller as `Unavailable`.
fn classify_exit(success: bool, code: Option<i32>, stdout: String) -> CovProbe {
    if success {
        CovProbe::Ran(stdout)
    } else {
        CovProbe::Failed { code }
    }
}

/// Parse line coverage percentage from llvm-cov summary output.
fn parse_coverage_from_output(stdout: &str) -> Option<f64> {
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("TOTAL") || trimmed.contains("TOTAL") {
            for word in trimmed.split_whitespace().rev() {
                if let Some(pct_str) = word.strip_suffix('%') {
                    if let Ok(pct) = pct_str.parse::<f64>() {
                        return Some(pct);
                    }
                }
            }
        }
    }
    None
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

// Tests live in `promotion_tests.rs` (split out to keep this file under the
// 500-line health limit). Included as a `#[path]` child `mod tests` so they
// retain access to the module's private gate helpers (`classify_exit`, etc.).
#[cfg(test)]
#[path = "promotion_tests.rs"]
mod tests;
