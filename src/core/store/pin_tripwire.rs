//! FJ-1314: Tripwire integration for input pinning.
//!
//! Extends tripwire upstream detection for lock file awareness. During
//! `forjar apply`, the lock file is compared against resolved inputs —
//! if an input has changed, forjar warns before applying.

use super::lockfile::{check_staleness, LockFile, StalenessEntry};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Result of a tripwire pin check during apply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinCheckResult {
    /// Whether all pins are fresh
    pub all_fresh: bool,

    /// Stale pins detected
    pub stale_pins: Vec<StalenessEntry>,

    /// Missing inputs (not in lock file)
    pub missing_inputs: Vec<String>,

    /// Summary message for the user
    pub summary: String,
}

/// Check lock file against current resolved hashes before apply.
///
/// Returns a warning report if any inputs have changed since pinning.
pub fn check_before_apply(
    lock_file: &LockFile,
    current_hashes: &BTreeMap<String, String>,
    all_input_names: &[String],
) -> PinCheckResult {
    let stale = check_staleness(lock_file, current_hashes);

    let missing: Vec<String> = all_input_names
        .iter()
        .filter(|name| !lock_file.pins.contains_key(*name))
        .cloned()
        .collect();

    let all_fresh = stale.is_empty() && missing.is_empty();

    let summary = if all_fresh {
        "All input pins are fresh — safe to apply.".to_string()
    } else {
        let mut parts = Vec::new();
        if !stale.is_empty() {
            parts.push(format!("{} stale pin(s)", stale.len()));
        }
        if !missing.is_empty() {
            parts.push(format!("{} unpinned input(s)", missing.len()));
        }
        format!(
            "WARNING: {} detected. Run `forjar pin --update` to refresh.",
            parts.join(" and ")
        )
    };

    PinCheckResult {
        all_fresh,
        stale_pins: stale,
        missing_inputs: missing,
        summary,
    }
}

/// Severity level for pin warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinSeverity {
    /// Informational — pins are fresh
    Info,
    /// Warning — some pins are stale (apply will proceed with warning)
    Warning,
    /// Error — CI gate mode, stale pins block apply
    Error,
}

/// Determine severity based on configuration.
pub fn pin_severity(result: &PinCheckResult, strict_mode: bool) -> PinSeverity {
    if result.all_fresh {
        PinSeverity::Info
    } else if strict_mode {
        PinSeverity::Error
    } else {
        PinSeverity::Warning
    }
}

/// Format a pin check result for display.
pub fn format_pin_report(result: &PinCheckResult) -> String {
    let mut lines = vec![result.summary.clone()];

    for stale in &result.stale_pins {
        lines.push(format!(
            "  STALE: {} — locked={} current={}",
            stale.name, stale.locked_hash, stale.current_hash
        ));
    }

    for name in &result.missing_inputs {
        lines.push(format!("  MISSING: {name} — not in lock file"));
    }

    lines.join("\n")
}

/// Check if a lock file needs regeneration.
pub fn needs_pin_update(
    lock_file: &LockFile,
    current_hashes: &BTreeMap<String, String>,
    all_input_names: &[String],
) -> bool {
    let result = check_before_apply(lock_file, current_hashes, all_input_names);
    !result.all_fresh
}
