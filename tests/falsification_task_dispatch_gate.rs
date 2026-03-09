//! FJ-2700/2702: Task dispatch and quality gate evaluation.
//!
//! Popperian rejection criteria for:
//! - FJ-2700: prepare_dispatch (param substitution, override precedence)
//! - FJ-2700: record_invocation (history, trimming, total count)
//! - FJ-2700: validate_dispatch (empty name/command, valid)
//! - FJ-2700: dispatch_script (set -euo pipefail, newline)
//! - FJ-2700: success_rate (all pass, all fail, mixed, empty)
//! - FJ-2700: format_dispatch_summary (output format)
//! - FJ-2702: evaluate_gate — exit code, JSON field, threshold, min, regex
//! - FJ-2702: GateAction — block (default), warn, skip_dependents
//!
//! Usage: cargo test --test falsification_task_dispatch_gate

use forjar::core::task::dispatch::{
    dispatch_script, format_dispatch_summary, prepare_dispatch, record_invocation, success_rate,
    validate_dispatch, PreparedDispatch,
};
use forjar::core::task::{evaluate_gate, gpu_env_vars, GateResult};
use forjar::core::types::{DispatchConfig, DispatchInvocation, DispatchState, QualityGate};

// ============================================================================
// FJ-2700: prepare_dispatch — parameter substitution
// ============================================================================

#[test]
fn dispatch_substitutes_single_param() {
    let config = DispatchConfig {
        name: "deploy".into(),
        command: "deploy --env {{ env }}".into(),
        params: vec![("env".into(), "production".into())],
        timeout_secs: Some(300),
    };
    let prepared = prepare_dispatch(&config, &[]);
    assert_eq!(prepared.command, "deploy --env production");
    assert_eq!(prepared.timeout_secs, Some(300));
    assert_eq!(prepared.name, "deploy");
}

#[test]
fn dispatch_substitutes_multiple_params() {
    let config = DispatchConfig {
        name: "build".into(),
        command: "make {{ target }} PROFILE={{ profile }}".into(),
        params: vec![
            ("target".into(), "all".into()),
            ("profile".into(), "release".into()),
        ],
        timeout_secs: None,
    };
    let prepared = prepare_dispatch(&config, &[]);
    assert_eq!(prepared.command, "make all PROFILE=release");
}

#[test]
fn dispatch_config_params_apply_before_overrides() {
    let config = DispatchConfig {
        name: "test".into(),
        command: "test {{ suite }}".into(),
        params: vec![("suite".into(), "unit".into())],
        timeout_secs: None,
    };
    let prepared = prepare_dispatch(&config, &[("suite".into(), "integration".into())]);
    assert_eq!(prepared.command, "test unit");
}

#[test]
fn dispatch_override_only_when_no_config_param() {
    let config = DispatchConfig {
        name: "test".into(),
        command: "test {{ suite }}".into(),
        params: vec![],
        timeout_secs: Some(60),
    };
    let prepared = prepare_dispatch(&config, &[("suite".into(), "integration".into())]);
    assert_eq!(prepared.command, "test integration");
}

#[test]
fn dispatch_unmatched_placeholder_preserved() {
    let config = DispatchConfig {
        name: "run".into(),
        command: "run {{ missing }}".into(),
        params: vec![],
        timeout_secs: None,
    };
    let prepared = prepare_dispatch(&config, &[]);
    assert_eq!(prepared.command, "run {{ missing }}");
}

// ============================================================================
// FJ-2700: record_invocation — history management
// ============================================================================

#[test]
fn record_invocation_increments_total() {
    let mut state = DispatchState::default();
    let inv = DispatchInvocation {
        timestamp: "2026-03-09T00:00:00Z".into(),
        exit_code: 0,
        duration_ms: 100,
        caller: Some("ci".into()),
    };
    record_invocation(&mut state, inv, 10);
    assert_eq!(state.total_invocations, 1);
    assert_eq!(state.invocations.len(), 1);
}

#[test]
fn record_invocation_trims_at_max() {
    let mut state = DispatchState::default();
    for i in 0..20 {
        let inv = DispatchInvocation {
            timestamp: format!("t{i}"),
            exit_code: if i % 3 == 0 { 1 } else { 0 },
            duration_ms: i * 100,
            caller: None,
        };
        record_invocation(&mut state, inv, 5);
    }
    assert_eq!(state.total_invocations, 20);
    assert_eq!(state.invocations.len(), 5);
    assert_eq!(state.invocations[0].timestamp, "t19");
}

// ============================================================================
// FJ-2700: validate_dispatch
// ============================================================================

#[test]
fn validate_dispatch_rejects_empty_name() {
    let config = DispatchConfig {
        name: String::new(),
        command: "echo hi".into(),
        params: vec![],
        timeout_secs: None,
    };
    let err = validate_dispatch(&config).unwrap_err();
    assert!(err.contains("name"));
}

#[test]
fn validate_dispatch_rejects_empty_command() {
    let config = DispatchConfig {
        name: "test".into(),
        command: String::new(),
        params: vec![],
        timeout_secs: None,
    };
    let err = validate_dispatch(&config).unwrap_err();
    assert!(err.contains("command"));
}

#[test]
fn validate_dispatch_accepts_valid() {
    let config = DispatchConfig {
        name: "build".into(),
        command: "cargo build --release".into(),
        params: vec![],
        timeout_secs: Some(600),
    };
    assert!(validate_dispatch(&config).is_ok());
}

// ============================================================================
// FJ-2700: dispatch_script & success_rate & format_summary
// ============================================================================

#[test]
fn dispatch_script_has_pipefail() {
    let prepared = PreparedDispatch {
        command: "echo hello".into(),
        timeout_secs: None,
        name: "test".into(),
    };
    let script = dispatch_script(&prepared);
    assert!(script.starts_with("set -euo pipefail"));
    assert!(script.contains("echo hello"));
    assert!(script.ends_with('\n'));
}

#[test]
fn success_rate_all_pass() {
    let state = DispatchState {
        invocations: vec![
            DispatchInvocation {
                timestamp: "t1".into(),
                exit_code: 0,
                duration_ms: 100,
                caller: None,
            },
            DispatchInvocation {
                timestamp: "t2".into(),
                exit_code: 0,
                duration_ms: 200,
                caller: None,
            },
        ],
        total_invocations: 2,
    };
    assert!((success_rate(&state) - 100.0).abs() < 0.01);
}

#[test]
fn success_rate_all_fail() {
    let state = DispatchState {
        invocations: vec![DispatchInvocation {
            timestamp: "t1".into(),
            exit_code: 1,
            duration_ms: 100,
            caller: None,
        }],
        total_invocations: 1,
    };
    assert!((success_rate(&state) - 0.0).abs() < 0.01);
}

#[test]
fn success_rate_empty_is_zero() {
    assert!((success_rate(&DispatchState::default()) - 0.0).abs() < 0.01);
}

#[test]
fn format_dispatch_summary_structure() {
    let state = DispatchState {
        invocations: vec![DispatchInvocation {
            timestamp: "2026-03-09".into(),
            exit_code: 0,
            duration_ms: 1500,
            caller: Some("admin".into()),
        }],
        total_invocations: 42,
    };
    let summary = format_dispatch_summary("deploy", &state);
    assert!(summary.contains("deploy"));
    assert!(summary.contains("total=42"));
    assert!(summary.contains("by=admin"));
}

// ============================================================================
// FJ-2702: evaluate_gate — exit code
// ============================================================================

#[test]
fn gate_pass_on_exit_zero() {
    assert_eq!(
        evaluate_gate(&QualityGate::default(), 0, ""),
        GateResult::Pass
    );
}

#[test]
fn gate_fail_on_nonzero_exit() {
    match evaluate_gate(&QualityGate::default(), 1, "") {
        GateResult::Fail(_, msg) => assert!(msg.contains("code 1")),
        GateResult::Pass => panic!("expected failure"),
    }
}

#[test]
fn gate_custom_message_on_failure() {
    let gate = QualityGate {
        message: Some("coverage too low".into()),
        ..Default::default()
    };
    match evaluate_gate(&gate, 1, "") {
        GateResult::Fail(_, msg) => assert_eq!(msg, "coverage too low"),
        GateResult::Pass => panic!("expected failure"),
    }
}

// ============================================================================
// FJ-2702: evaluate_gate — JSON field + threshold + min
// ============================================================================

#[test]
fn gate_json_threshold_pass() {
    let gate = QualityGate {
        parse: Some("json".into()),
        field: Some("grade".into()),
        threshold: vec!["A".into(), "B".into()],
        ..Default::default()
    };
    assert_eq!(
        evaluate_gate(&gate, 0, r#"{"grade":"A"}"#),
        GateResult::Pass
    );
}

#[test]
fn gate_json_threshold_fail() {
    let gate = QualityGate {
        parse: Some("json".into()),
        field: Some("grade".into()),
        threshold: vec!["A".into(), "B".into()],
        ..Default::default()
    };
    match evaluate_gate(&gate, 0, r#"{"grade":"F"}"#) {
        GateResult::Fail(_, msg) => assert!(msg.contains("F")),
        GateResult::Pass => panic!("expected threshold failure"),
    }
}

#[test]
fn gate_json_missing_field() {
    let gate = QualityGate {
        parse: Some("json".into()),
        field: Some("missing".into()),
        ..Default::default()
    };
    match evaluate_gate(&gate, 0, r#"{"grade":"A"}"#) {
        GateResult::Fail(_, msg) => assert!(msg.contains("not found")),
        GateResult::Pass => panic!("expected missing field failure"),
    }
}

#[test]
fn gate_json_invalid_json() {
    let gate = QualityGate {
        parse: Some("json".into()),
        field: Some("grade".into()),
        ..Default::default()
    };
    match evaluate_gate(&gate, 0, "not json") {
        GateResult::Fail(_, msg) => assert!(msg.contains("JSON")),
        GateResult::Pass => panic!("expected parse failure"),
    }
}

#[test]
fn gate_json_min_pass() {
    let gate = QualityGate {
        parse: Some("json".into()),
        field: Some("coverage".into()),
        min: Some(80.0),
        ..Default::default()
    };
    assert_eq!(
        evaluate_gate(&gate, 0, r#"{"coverage":95.5}"#),
        GateResult::Pass
    );
}

#[test]
fn gate_json_min_fail() {
    let gate = QualityGate {
        parse: Some("json".into()),
        field: Some("coverage".into()),
        min: Some(80.0),
        ..Default::default()
    };
    match evaluate_gate(&gate, 0, r#"{"coverage":72.3}"#) {
        GateResult::Fail(_, msg) => assert!(msg.contains("72.3")),
        GateResult::Pass => panic!("expected min failure"),
    }
}

// ============================================================================
// FJ-2702: evaluate_gate — regex stdout
// ============================================================================

#[test]
fn gate_regex_match_passes() {
    let gate = QualityGate {
        regex: Some(r"ALL \d+ TESTS PASSED".into()),
        ..Default::default()
    };
    assert_eq!(
        evaluate_gate(&gate, 0, "ALL 42 TESTS PASSED"),
        GateResult::Pass
    );
}

#[test]
fn gate_regex_no_match_fails() {
    let gate = QualityGate {
        regex: Some(r"PASSED".into()),
        ..Default::default()
    };
    match evaluate_gate(&gate, 0, "3 tests FAILED") {
        GateResult::Fail(_, msg) => assert!(msg.contains("PASSED")),
        GateResult::Pass => panic!("expected regex failure"),
    }
}

#[test]
fn gate_regex_invalid_pattern() {
    let gate = QualityGate {
        regex: Some(r"[invalid".into()),
        ..Default::default()
    };
    match evaluate_gate(&gate, 0, "anything") {
        GateResult::Fail(_, msg) => assert!(msg.contains("invalid regex")),
        GateResult::Pass => panic!("expected invalid regex failure"),
    }
}

// ============================================================================
// FJ-2702: GateAction — on_fail modes
// ============================================================================

#[test]
fn gate_action_default_is_block() {
    match evaluate_gate(&QualityGate::default(), 1, "") {
        GateResult::Fail(action, _) => assert_eq!(format!("{action:?}"), "Block"),
        GateResult::Pass => panic!("expected failure"),
    }
}

#[test]
fn gate_action_warn() {
    let gate = QualityGate {
        on_fail: Some("warn".into()),
        ..Default::default()
    };
    match evaluate_gate(&gate, 1, "") {
        GateResult::Fail(action, _) => assert_eq!(format!("{action:?}"), "Warn"),
        GateResult::Pass => panic!("expected failure"),
    }
}

#[test]
fn gate_action_skip_dependents() {
    let gate = QualityGate {
        on_fail: Some("skip_dependents".into()),
        ..Default::default()
    };
    match evaluate_gate(&gate, 1, "") {
        GateResult::Fail(action, _) => assert_eq!(format!("{action:?}"), "SkipDependents"),
        GateResult::Pass => panic!("expected failure"),
    }
}

// ============================================================================
// FJ-2703: gpu_env_vars
// ============================================================================

#[test]
fn gpu_env_vars_with_device() {
    let vars = gpu_env_vars(Some(2));
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0], ("CUDA_VISIBLE_DEVICES".into(), "2".into()));
    assert_eq!(vars[1], ("HIP_VISIBLE_DEVICES".into(), "2".into()));
}

#[test]
fn gpu_env_vars_none() {
    assert!(gpu_env_vars(None).is_empty());
}
