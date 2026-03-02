//! Tests for FJ-1306/FJ-1329: Validation commands.

use super::purity::{PurityLevel, PuritySignals};
use super::repro_score::ReproInput;
use super::validate::{
    format_purity_report, format_repro_report, validate_purity, validate_repro_score,
};

fn pure_signals() -> PuritySignals {
    PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![],
    }
}

fn pinned_signals() -> PuritySignals {
    PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![],
    }
}

fn impure_signals() -> PuritySignals {
    PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: true,
        dep_levels: vec![],
    }
}

#[test]
fn test_fj1306_validate_all_pure() {
    let sig = pure_signals();
    let result = validate_purity(&[("nginx", &sig)], None);
    assert!(result.pass);
    assert_eq!(result.recipe_purity, PurityLevel::Pure);
    assert_eq!(result.resources[0].level, PurityLevel::Pure);
}

#[test]
fn test_fj1306_validate_mixed() {
    let pure = pure_signals();
    let pinned = pinned_signals();
    let result = validate_purity(&[("nginx", &pure), ("redis", &pinned)], None);
    assert_eq!(result.recipe_purity, PurityLevel::Pinned);
}

#[test]
fn test_fj1306_validate_with_min_pass() {
    let sig = pure_signals();
    let result = validate_purity(&[("nginx", &sig)], Some(PurityLevel::Pinned));
    assert!(result.pass);
}

#[test]
fn test_fj1306_validate_with_min_fail() {
    let sig = pinned_signals();
    let result = validate_purity(&[("nginx", &sig)], Some(PurityLevel::Pure));
    assert!(!result.pass);
}

#[test]
fn test_fj1306_validate_impure_fails_pinned_gate() {
    let sig = impure_signals();
    let result = validate_purity(&[("script", &sig)], Some(PurityLevel::Pinned));
    assert!(!result.pass);
    assert_eq!(result.recipe_purity, PurityLevel::Impure);
}

#[test]
fn test_fj1306_format_report() {
    let sig = pure_signals();
    let result = validate_purity(&[("nginx", &sig)], None);
    let report = format_purity_report(&result);
    assert!(report.contains("Pure"));
    assert!(report.contains("PASS"));
    assert!(report.contains("nginx"));
}

#[test]
fn test_fj1306_format_report_fail() {
    let sig = impure_signals();
    let result = validate_purity(&[("script", &sig)], Some(PurityLevel::Pure));
    let report = format_purity_report(&result);
    assert!(report.contains("FAIL"));
    assert!(report.contains("Required"));
}

#[test]
fn test_fj1306_empty_signals() {
    let result = validate_purity(&[], None);
    assert!(result.pass);
    assert_eq!(result.recipe_purity, PurityLevel::Pure);
}

#[test]
fn test_fj1329_validate_score_pass() {
    let inputs = vec![ReproInput {
        name: "nginx".to_string(),
        purity: PurityLevel::Pure,
        has_store: true,
        has_lock_pin: true,
    }];
    let result = validate_repro_score(&inputs, Some(90.0));
    assert!(result.pass);
    assert_eq!(result.grade, "A");
}

#[test]
fn test_fj1329_validate_score_fail() {
    let inputs = vec![ReproInput {
        name: "script".to_string(),
        purity: PurityLevel::Impure,
        has_store: false,
        has_lock_pin: false,
    }];
    let result = validate_repro_score(&inputs, Some(50.0));
    assert!(!result.pass);
}

#[test]
fn test_fj1329_validate_score_no_min() {
    let inputs = vec![ReproInput {
        name: "x".to_string(),
        purity: PurityLevel::Impure,
        has_store: false,
        has_lock_pin: false,
    }];
    let result = validate_repro_score(&inputs, None);
    assert!(result.pass);
}

#[test]
fn test_fj1329_format_repro_report() {
    let inputs = vec![ReproInput {
        name: "nginx".to_string(),
        purity: PurityLevel::Pure,
        has_store: true,
        has_lock_pin: true,
    }];
    let result = validate_repro_score(&inputs, Some(90.0));
    let report = format_repro_report(&result);
    assert!(report.contains("100.0"));
    assert!(report.contains("PASS"));
    assert!(report.contains("Grade A"));
}

#[test]
fn test_fj1329_format_repro_fail() {
    let inputs = vec![ReproInput {
        name: "x".to_string(),
        purity: PurityLevel::Impure,
        has_store: false,
        has_lock_pin: false,
    }];
    let result = validate_repro_score(&inputs, Some(50.0));
    let report = format_repro_report(&result);
    assert!(report.contains("FAIL"));
    assert!(report.contains("Required"));
}
