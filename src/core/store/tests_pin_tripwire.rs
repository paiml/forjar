//! Tests for FJ-1314: Tripwire integration for input pinning.

use super::lockfile::{LockFile, Pin};
use super::pin_tripwire::{
    check_before_apply, format_pin_report, needs_pin_update, pin_severity, PinSeverity,
};
use std::collections::BTreeMap;

fn sample_lockfile() -> LockFile {
    let mut pins = BTreeMap::new();
    pins.insert(
        "nginx".to_string(),
        Pin {
            provider: "apt".to_string(),
            version: Some("1.24.0".to_string()),
            hash: "blake3:abc123".to_string(),
            git_rev: None,
            pin_type: None,
        },
    );
    pins.insert(
        "ripgrep".to_string(),
        Pin {
            provider: "cargo".to_string(),
            version: Some("14.1.0".to_string()),
            hash: "blake3:def456".to_string(),
            git_rev: None,
            pin_type: None,
        },
    );
    LockFile {
        schema: "1.0".to_string(),
        pins,
    }
}

#[test]
fn test_fj1314_all_fresh() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:abc123".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let names = vec!["nginx".to_string(), "ripgrep".to_string()];

    let result = check_before_apply(&lf, &current, &names);
    assert!(result.all_fresh);
    assert!(result.stale_pins.is_empty());
    assert!(result.missing_inputs.is_empty());
    assert!(result.summary.contains("safe to apply"));
}

#[test]
fn test_fj1314_stale_pin() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:abc123".to_string());
    current.insert("ripgrep".to_string(), "blake3:CHANGED".to_string());
    let names = vec!["nginx".to_string(), "ripgrep".to_string()];

    let result = check_before_apply(&lf, &current, &names);
    assert!(!result.all_fresh);
    assert_eq!(result.stale_pins.len(), 1);
    assert_eq!(result.stale_pins[0].name, "ripgrep");
    assert!(result.summary.contains("stale"));
}

#[test]
fn test_fj1314_missing_input() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:abc123".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let names = vec![
        "nginx".to_string(),
        "ripgrep".to_string(),
        "python".to_string(),
    ];

    let result = check_before_apply(&lf, &current, &names);
    assert!(!result.all_fresh);
    assert_eq!(result.missing_inputs, vec!["python"]);
    assert!(result.summary.contains("unpinned"));
}

#[test]
fn test_fj1314_stale_and_missing() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:UPDATED".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let names = vec![
        "nginx".to_string(),
        "ripgrep".to_string(),
        "python".to_string(),
    ];

    let result = check_before_apply(&lf, &current, &names);
    assert!(!result.all_fresh);
    assert_eq!(result.stale_pins.len(), 1);
    assert_eq!(result.missing_inputs.len(), 1);
    assert!(result.summary.contains("stale"));
    assert!(result.summary.contains("unpinned"));
}

#[test]
fn test_fj1314_severity_info() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:abc123".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let names = vec!["nginx".to_string(), "ripgrep".to_string()];
    let result = check_before_apply(&lf, &current, &names);
    assert_eq!(pin_severity(&result, false), PinSeverity::Info);
    assert_eq!(pin_severity(&result, true), PinSeverity::Info);
}

#[test]
fn test_fj1314_severity_warning() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:CHANGED".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let names = vec!["nginx".to_string(), "ripgrep".to_string()];
    let result = check_before_apply(&lf, &current, &names);
    assert_eq!(pin_severity(&result, false), PinSeverity::Warning);
}

#[test]
fn test_fj1314_severity_error_strict() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:CHANGED".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let names = vec!["nginx".to_string(), "ripgrep".to_string()];
    let result = check_before_apply(&lf, &current, &names);
    assert_eq!(pin_severity(&result, true), PinSeverity::Error);
}

#[test]
fn test_fj1314_format_report_fresh() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:abc123".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let names = vec!["nginx".to_string(), "ripgrep".to_string()];
    let result = check_before_apply(&lf, &current, &names);
    let report = format_pin_report(&result);
    assert!(report.contains("safe to apply"));
}

#[test]
fn test_fj1314_format_report_stale() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:CHANGED".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let names = vec!["nginx".to_string(), "ripgrep".to_string()];
    let result = check_before_apply(&lf, &current, &names);
    let report = format_pin_report(&result);
    assert!(report.contains("STALE"));
    assert!(report.contains("nginx"));
}

#[test]
fn test_fj1314_format_report_missing() {
    let lf = sample_lockfile();
    let current = BTreeMap::new();
    let names = vec!["unknown".to_string()];
    let result = check_before_apply(&lf, &current, &names);
    let report = format_pin_report(&result);
    assert!(report.contains("MISSING"));
    assert!(report.contains("unknown"));
}

#[test]
fn test_fj1314_needs_pin_update_false() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:abc123".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let names = vec!["nginx".to_string(), "ripgrep".to_string()];
    assert!(!needs_pin_update(&lf, &current, &names));
}

#[test]
fn test_fj1314_needs_pin_update_true() {
    let lf = sample_lockfile();
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:CHANGED".to_string());
    current.insert("ripgrep".to_string(), "blake3:def456".to_string());
    let names = vec!["nginx".to_string(), "ripgrep".to_string()];
    assert!(needs_pin_update(&lf, &current, &names));
}

#[test]
fn test_fj1314_empty_lock_file() {
    let lf = LockFile {
        schema: "1.0".to_string(),
        pins: BTreeMap::new(),
    };
    let current = BTreeMap::new();
    let names = vec!["nginx".to_string()];
    let result = check_before_apply(&lf, &current, &names);
    assert!(!result.all_fresh);
    assert_eq!(result.missing_inputs, vec!["nginx"]);
}

#[test]
fn test_fj1314_empty_inputs() {
    let lf = sample_lockfile();
    let current = BTreeMap::new();
    let names: Vec<String> = vec![];
    let result = check_before_apply(&lf, &current, &names);
    assert!(result.all_fresh);
}
