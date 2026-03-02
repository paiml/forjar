//! Tests for FJ-1328: Recipe conversion strategy.

use super::convert::{analyze_conversion, ChangeType, ConversionSignals};
use super::purity::PurityLevel;

fn sig(name: &str, version: bool, store: bool, sandbox: bool, curl: bool, provider: &str) -> ConversionSignals {
    ConversionSignals {
        name: name.to_string(),
        has_version: version,
        has_store: store,
        has_sandbox: sandbox,
        has_curl_pipe: curl,
        provider: provider.to_string(),
        current_version: if version {
            Some("1.0.0".to_string())
        } else {
            None
        },
    }
}

#[test]
fn test_fj1328_already_pure() {
    let signals = vec![sig("nginx", true, true, true, false, "apt")];
    let report = analyze_conversion(&signals);
    assert_eq!(report.resources[0].current_purity, PurityLevel::Pure);
    assert_eq!(report.auto_change_count, 0);
}

#[test]
fn test_fj1328_needs_version_pin() {
    let signals = vec![sig("nginx", false, false, false, false, "apt")];
    let report = analyze_conversion(&signals);
    assert!(report.resources[0]
        .auto_changes
        .iter()
        .any(|c| c.change_type == ChangeType::AddVersionPin));
}

#[test]
fn test_fj1328_needs_store() {
    let signals = vec![sig("nginx", true, false, false, false, "apt")];
    let report = analyze_conversion(&signals);
    assert!(report.resources[0]
        .auto_changes
        .iter()
        .any(|c| c.change_type == ChangeType::EnableStore));
}

#[test]
fn test_fj1328_needs_lock_pin() {
    let signals = vec![sig("nginx", true, false, false, false, "apt")];
    let report = analyze_conversion(&signals);
    assert!(report.resources[0]
        .auto_changes
        .iter()
        .any(|c| c.change_type == ChangeType::GenerateLockPin));
}

#[test]
fn test_fj1328_curl_pipe_manual() {
    let signals = vec![sig("script", true, true, true, true, "shell")];
    let report = analyze_conversion(&signals);
    assert_eq!(report.resources[0].current_purity, PurityLevel::Impure);
    assert!(!report.resources[0].manual_changes.is_empty());
    assert!(report.resources[0]
        .manual_changes
        .iter()
        .any(|m| m.contains("curl|bash")));
}

#[test]
fn test_fj1328_sandbox_manual() {
    let signals = vec![sig("nginx", true, true, false, false, "apt")];
    let report = analyze_conversion(&signals);
    assert!(report.resources[0]
        .manual_changes
        .iter()
        .any(|m| m.contains("sandbox")));
}

#[test]
fn test_fj1328_projected_purity_pinned() {
    let signals = vec![sig("nginx", false, false, false, false, "apt")];
    let report = analyze_conversion(&signals);
    // After auto: has version + has store → Pinned
    assert_eq!(report.resources[0].target_purity, PurityLevel::Pinned);
}

#[test]
fn test_fj1328_projected_purity_stays_impure() {
    let signals = vec![sig("script", true, true, true, true, "shell")];
    let report = analyze_conversion(&signals);
    assert_eq!(report.resources[0].target_purity, PurityLevel::Impure);
}

#[test]
fn test_fj1328_multiple_resources() {
    let signals = vec![
        sig("nginx", true, true, true, false, "apt"),
        sig("script", false, false, false, true, "shell"),
        sig("redis", true, false, false, false, "apt"),
    ];
    let report = analyze_conversion(&signals);
    assert_eq!(report.resources.len(), 3);
    assert_eq!(report.current_purity, PurityLevel::Impure);
}

#[test]
fn test_fj1328_empty_signals() {
    let report = analyze_conversion(&[]);
    assert!(report.resources.is_empty());
    assert_eq!(report.auto_change_count, 0);
    assert_eq!(report.current_purity, PurityLevel::Pure);
}

#[test]
fn test_fj1328_non_cacheable_provider() {
    let signals = vec![sig("infra", true, false, false, false, "terraform")];
    let report = analyze_conversion(&signals);
    // terraform is not in cacheable providers, so no EnableStore change
    assert!(!report.resources[0]
        .auto_changes
        .iter()
        .any(|c| c.change_type == ChangeType::EnableStore));
}

#[test]
fn test_fj1328_auto_count() {
    let signals = vec![
        sig("a", false, false, false, false, "apt"),
        sig("b", false, false, false, false, "cargo"),
    ];
    let report = analyze_conversion(&signals);
    // Each gets: AddVersionPin + EnableStore + GenerateLockPin = 3 changes each
    assert_eq!(report.auto_change_count, 6);
}

#[test]
fn test_fj1328_manual_count() {
    let signals = vec![
        sig("a", true, true, false, false, "apt"),   // sandbox manual
        sig("b", true, true, true, true, "shell"),    // curl|bash manual
    ];
    let report = analyze_conversion(&signals);
    assert_eq!(report.manual_change_count, 2);
}

#[test]
fn test_fj1328_projected_overall_purity() {
    let signals = vec![
        sig("a", false, false, false, false, "apt"),
        sig("b", false, false, false, false, "cargo"),
    ];
    let report = analyze_conversion(&signals);
    // Both become Pinned after auto
    assert_eq!(report.projected_purity, PurityLevel::Pinned);
}

#[test]
fn test_fj1328_serde_roundtrip() {
    let signals = vec![sig("nginx", false, false, false, false, "apt")];
    let report = analyze_conversion(&signals);
    let json = serde_json::to_string(&report).unwrap();
    let _parsed: super::convert::ConversionReport = serde_json::from_str(&json).unwrap();
}
