//! FJ-2702: Quality gate evaluation for pipeline tasks.
//!
//! Evaluates gate conditions against task execution output.
//! Supports: exit code gates, JSON field parsing, regex stdout, numeric thresholds.

use crate::core::types::QualityGate;

/// Result of a quality gate evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateResult {
    /// Gate passed — continue pipeline.
    Pass,
    /// Gate failed — action determined by `on_fail` field.
    Fail(GateAction, String),
}

/// Action to take when a gate fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateAction {
    /// Block pipeline execution (default).
    Block,
    /// Emit warning but continue.
    Warn,
    /// Skip dependent stages.
    SkipDependents,
}

/// Evaluate a quality gate against execution output.
///
/// Returns `GateResult::Pass` if the gate condition is met, or
/// `GateResult::Fail` with the appropriate action and error message.
///
/// # Examples
///
/// ```
/// use forjar::core::task::{evaluate_gate, GateResult};
/// use forjar::core::types::QualityGate;
///
/// let gate = QualityGate::default();
/// assert_eq!(evaluate_gate(&gate, 0, ""), GateResult::Pass);
/// ```
pub fn evaluate_gate(gate: &QualityGate, exit_code: i32, stdout: &str) -> GateResult {
    let action = parse_action(gate.on_fail.as_deref());

    // Exit code gate: command must exit 0 to pass
    if exit_code != 0 {
        let msg = gate
            .message
            .clone()
            .unwrap_or_else(|| format!("gate failed: command exited with code {exit_code}"));
        return GateResult::Fail(action, msg);
    }

    // JSON field gate
    if let Some(ref parse_mode) = gate.parse {
        if parse_mode == "json" {
            return evaluate_json_gate(gate, stdout, action);
        }
    }

    // Regex stdout gate
    if let Some(ref pattern) = gate.regex {
        return evaluate_regex_gate(pattern, stdout, action, gate.message.as_deref());
    }

    GateResult::Pass
}

fn evaluate_json_gate(gate: &QualityGate, stdout: &str, action: GateAction) -> GateResult {
    let field_name = match gate.field.as_deref() {
        Some(f) => f,
        None => return GateResult::Pass,
    };

    let parsed: serde_json::Value = match serde_json::from_str(stdout) {
        Ok(v) => v,
        Err(e) => {
            return GateResult::Fail(
                action,
                gate.message
                    .clone()
                    .unwrap_or_else(|| format!("gate: invalid JSON output: {e}")),
            );
        }
    };

    let value = match parsed.get(field_name) {
        Some(v) => v,
        None => {
            return GateResult::Fail(
                action,
                gate.message
                    .clone()
                    .unwrap_or_else(|| format!("gate: JSON field '{field_name}' not found")),
            );
        }
    };

    // Threshold check (string values in allowed list)
    if !gate.threshold.is_empty() {
        let val_str = match value.as_str() {
            Some(s) => s.to_string(),
            None => value.to_string(),
        };
        if !gate.threshold.contains(&val_str) {
            return GateResult::Fail(
                action,
                gate.message.clone().unwrap_or_else(|| {
                    format!(
                        "gate: field '{field_name}' = '{val_str}', expected one of {:?}",
                        gate.threshold
                    )
                }),
            );
        }
    }

    // Numeric minimum check
    if let Some(min) = gate.min {
        let num = value.as_f64().unwrap_or_else(|| {
            value
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(f64::NAN)
        });
        if num < min {
            return GateResult::Fail(
                action,
                gate.message.clone().unwrap_or_else(|| {
                    format!("gate: field '{field_name}' = {num}, minimum is {min}")
                }),
            );
        }
    }

    GateResult::Pass
}

fn evaluate_regex_gate(
    pattern: &str,
    stdout: &str,
    action: GateAction,
    message: Option<&str>,
) -> GateResult {
    match regex::Regex::new(pattern) {
        Ok(re) => {
            if re.is_match(stdout) {
                GateResult::Pass
            } else {
                GateResult::Fail(
                    action,
                    message.map(String::from).unwrap_or_else(|| {
                        format!("gate: stdout did not match pattern '{pattern}'")
                    }),
                )
            }
        }
        Err(e) => GateResult::Fail(action, format!("gate: invalid regex '{pattern}': {e}")),
    }
}

fn parse_action(on_fail: Option<&str>) -> GateAction {
    match on_fail {
        Some("warn") => GateAction::Warn,
        Some("skip_dependents") => GateAction::SkipDependents,
        _ => GateAction::Block,
    }
}

/// FJ-2703: Build environment variables for GPU device targeting.
///
/// Returns `CUDA_VISIBLE_DEVICES` and `HIP_VISIBLE_DEVICES` for the given device.
///
/// # Examples
///
/// ```
/// use forjar::core::task::gpu_env_vars;
///
/// let vars = gpu_env_vars(Some(0));
/// assert_eq!(vars[0].0, "CUDA_VISIBLE_DEVICES");
/// assert_eq!(vars[0].1, "0");
/// ```
pub fn gpu_env_vars(gpu_device: Option<u32>) -> Vec<(String, String)> {
    match gpu_device {
        Some(dev) => vec![
            ("CUDA_VISIBLE_DEVICES".into(), dev.to_string()),
            ("HIP_VISIBLE_DEVICES".into(), dev.to_string()),
        ],
        None => vec![],
    }
}
