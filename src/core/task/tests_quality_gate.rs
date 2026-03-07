//! Tests: FJ-2702 quality gate evaluation + FJ-2703 GPU env targeting.

#![cfg(test)]

use super::quality_gate::*;
use crate::core::types::QualityGate;

fn gate() -> QualityGate {
    QualityGate::default()
}

// ── Exit code gates ──

#[test]
fn exit_code_zero_passes() {
    assert_eq!(evaluate_gate(&gate(), 0, ""), GateResult::Pass);
}

#[test]
fn exit_code_nonzero_fails() {
    let r = evaluate_gate(&gate(), 1, "output");
    assert!(matches!(r, GateResult::Fail(GateAction::Block, _)));
}

#[test]
fn exit_code_custom_message() {
    let g = QualityGate {
        message: Some("lint failed".into()),
        ..gate()
    };
    match evaluate_gate(&g, 1, "") {
        GateResult::Fail(_, msg) => assert_eq!(msg, "lint failed"),
        _ => panic!("expected Fail"),
    }
}

// ── JSON field gates ──

#[test]
fn json_threshold_pass() {
    let g = QualityGate {
        parse: Some("json".into()),
        field: Some("grade".into()),
        threshold: vec!["A".into(), "B".into()],
        ..gate()
    };
    let stdout = r#"{"grade":"A","score":95}"#;
    assert_eq!(evaluate_gate(&g, 0, stdout), GateResult::Pass);
}

#[test]
fn json_threshold_fail() {
    let g = QualityGate {
        parse: Some("json".into()),
        field: Some("grade".into()),
        threshold: vec!["A".into(), "B".into()],
        ..gate()
    };
    let stdout = r#"{"grade":"C","score":70}"#;
    assert!(matches!(
        evaluate_gate(&g, 0, stdout),
        GateResult::Fail(_, _)
    ));
}

#[test]
fn json_min_pass() {
    let g = QualityGate {
        parse: Some("json".into()),
        field: Some("coverage".into()),
        min: Some(95.0),
        ..gate()
    };
    let stdout = r#"{"coverage":96.5}"#;
    assert_eq!(evaluate_gate(&g, 0, stdout), GateResult::Pass);
}

#[test]
fn json_min_fail() {
    let g = QualityGate {
        parse: Some("json".into()),
        field: Some("coverage".into()),
        min: Some(95.0),
        ..gate()
    };
    let stdout = r#"{"coverage":90.0}"#;
    assert!(matches!(
        evaluate_gate(&g, 0, stdout),
        GateResult::Fail(_, _)
    ));
}

#[test]
fn json_missing_field_fails() {
    let g = QualityGate {
        parse: Some("json".into()),
        field: Some("missing".into()),
        threshold: vec!["A".into()],
        ..gate()
    };
    assert!(matches!(
        evaluate_gate(&g, 0, r#"{"other":"val"}"#),
        GateResult::Fail(_, _)
    ));
}

#[test]
fn json_invalid_output_fails() {
    let g = QualityGate {
        parse: Some("json".into()),
        field: Some("x".into()),
        ..gate()
    };
    assert!(matches!(
        evaluate_gate(&g, 0, "not json"),
        GateResult::Fail(_, _)
    ));
}

// ── Regex stdout gates ──

#[test]
fn regex_match_passes() {
    let g = QualityGate {
        regex: Some(r"PASS|OK".into()),
        ..gate()
    };
    assert_eq!(evaluate_gate(&g, 0, "tests: OK"), GateResult::Pass);
}

#[test]
fn regex_no_match_fails() {
    let g = QualityGate {
        regex: Some(r"PASS".into()),
        ..gate()
    };
    assert!(matches!(
        evaluate_gate(&g, 0, "tests: FAIL"),
        GateResult::Fail(_, _)
    ));
}

#[test]
fn regex_invalid_pattern_fails() {
    let g = QualityGate {
        regex: Some(r"[invalid".into()),
        ..gate()
    };
    assert!(matches!(
        evaluate_gate(&g, 0, "anything"),
        GateResult::Fail(_, _)
    ));
}

// ── on_fail actions ──

#[test]
fn on_fail_warn() {
    let g = QualityGate {
        on_fail: Some("warn".into()),
        ..gate()
    };
    match evaluate_gate(&g, 1, "") {
        GateResult::Fail(action, _) => assert_eq!(action, GateAction::Warn),
        _ => panic!("expected Fail"),
    }
}

#[test]
fn on_fail_skip_dependents() {
    let g = QualityGate {
        on_fail: Some("skip_dependents".into()),
        ..gate()
    };
    match evaluate_gate(&g, 1, "") {
        GateResult::Fail(action, _) => assert_eq!(action, GateAction::SkipDependents),
        _ => panic!("expected Fail"),
    }
}

#[test]
fn on_fail_default_is_block() {
    let g = gate();
    match evaluate_gate(&g, 1, "") {
        GateResult::Fail(action, _) => assert_eq!(action, GateAction::Block),
        _ => panic!("expected Fail"),
    }
}

// ── FJ-2703: GPU env vars ──

#[test]
fn gpu_env_vars_none() {
    assert!(gpu_env_vars(None).is_empty());
}

#[test]
fn gpu_env_vars_device_0() {
    let vars = gpu_env_vars(Some(0));
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0], ("CUDA_VISIBLE_DEVICES".into(), "0".into()));
    assert_eq!(vars[1], ("HIP_VISIBLE_DEVICES".into(), "0".into()));
}

#[test]
fn gpu_env_vars_device_3() {
    let vars = gpu_env_vars(Some(3));
    assert_eq!(vars[0].1, "3");
}
