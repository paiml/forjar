//! FJ-2700/045: Task dispatch and SAT dependency solving.
//!
//! Popperian rejection criteria for:
//! - FJ-2700: prepare_dispatch (param substitution, overrides), record_invocation
//!   (insert, trim history), format_dispatch_summary, validate_dispatch,
//!   dispatch_script, success_rate
//! - FJ-045: build_sat_problem (structure, var mapping), solve (satisfiable,
//!   unsatisfiable, diamond, linear chain, backtracking)
//!
//! Usage: cargo test --test falsification_task_dispatch

use forjar::core::planner::sat_deps::{build_sat_problem, solve, SatProblem, SatResult};
use forjar::core::task::dispatch::{
    dispatch_script, format_dispatch_summary, prepare_dispatch, record_invocation, success_rate,
    validate_dispatch, PreparedDispatch,
};
use forjar::core::types::{DispatchConfig, DispatchInvocation, DispatchState};
use std::collections::BTreeMap;

// ============================================================================
// FJ-2700: prepare_dispatch
// ============================================================================

#[test]
fn dispatch_substitutes_params() {
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
fn dispatch_overrides() {
    let config = DispatchConfig {
        name: "test".into(),
        command: "test {{ suite }}".into(),
        params: vec![],
        timeout_secs: None,
    };
    let prepared = prepare_dispatch(&config, &[("suite".into(), "integration".into())]);
    assert_eq!(prepared.command, "test integration");
}

#[test]
fn dispatch_no_placeholders() {
    let config = DispatchConfig {
        name: "build".into(),
        command: "cargo build --release".into(),
        params: vec![],
        timeout_secs: None,
    };
    let prepared = prepare_dispatch(&config, &[]);
    assert_eq!(prepared.command, "cargo build --release");
}

// ============================================================================
// FJ-2700: record_invocation
// ============================================================================

#[test]
fn record_adds_to_front() {
    let mut state = DispatchState::default();
    let inv = DispatchInvocation {
        timestamp: "t1".into(),
        exit_code: 0,
        duration_ms: 100,
        caller: Some("ci".into()),
    };
    record_invocation(&mut state, inv, 10);
    assert_eq!(state.total_invocations, 1);
    assert_eq!(state.invocations.len(), 1);
    assert_eq!(state.invocations[0].timestamp, "t1");
}

#[test]
fn record_trims_history() {
    let mut state = DispatchState::default();
    for i in 0..20 {
        let inv = DispatchInvocation {
            timestamp: format!("t{i}"),
            exit_code: 0,
            duration_ms: 100,
            caller: None,
        };
        record_invocation(&mut state, inv, 5);
    }
    assert_eq!(state.total_invocations, 20);
    assert_eq!(state.invocations.len(), 5);
    assert_eq!(state.invocations[0].timestamp, "t19"); // most recent
}

// ============================================================================
// FJ-2700: format_dispatch_summary
// ============================================================================

#[test]
fn summary_format() {
    let state = DispatchState {
        invocations: vec![
            DispatchInvocation {
                timestamp: "2026-03-09".into(),
                exit_code: 0,
                duration_ms: 1500,
                caller: Some("admin".into()),
            },
            DispatchInvocation {
                timestamp: "2026-03-08".into(),
                exit_code: 1,
                duration_ms: 300,
                caller: None,
            },
        ],
        total_invocations: 10,
    };
    let s = format_dispatch_summary("deploy", &state);
    assert!(s.contains("total=10"));
    assert!(s.contains("deploy"));
    assert!(s.contains("by=admin"));
}

#[test]
fn summary_empty() {
    let state = DispatchState::default();
    let s = format_dispatch_summary("empty", &state);
    assert!(s.contains("total=0"));
}

// ============================================================================
// FJ-2700: validate_dispatch
// ============================================================================

#[test]
fn validate_valid() {
    let config = DispatchConfig {
        name: "build".into(),
        command: "cargo build".into(),
        params: vec![],
        timeout_secs: None,
    };
    assert!(validate_dispatch(&config).is_ok());
}

#[test]
fn validate_empty_name() {
    let config = DispatchConfig {
        name: String::new(),
        command: "echo".into(),
        params: vec![],
        timeout_secs: None,
    };
    assert!(validate_dispatch(&config).is_err());
}

#[test]
fn validate_empty_command() {
    let config = DispatchConfig {
        name: "test".into(),
        command: String::new(),
        params: vec![],
        timeout_secs: None,
    };
    assert!(validate_dispatch(&config).is_err());
}

// ============================================================================
// FJ-2700: dispatch_script / success_rate
// ============================================================================

#[test]
fn script_has_pipefail() {
    let prepared = PreparedDispatch {
        command: "echo hello".into(),
        timeout_secs: None,
        name: "test".into(),
    };
    let script = dispatch_script(&prepared);
    assert!(script.starts_with("set -euo pipefail"));
    assert!(script.contains("echo hello"));
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
fn success_rate_mixed() {
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
                exit_code: 1,
                duration_ms: 200,
                caller: None,
            },
        ],
        total_invocations: 2,
    };
    assert!((success_rate(&state) - 50.0).abs() < 0.01);
}

#[test]
fn success_rate_empty() {
    assert!((success_rate(&DispatchState::default()) - 0.0).abs() < 0.01);
}

// ============================================================================
// FJ-045: SAT dependency solving
// ============================================================================

#[test]
fn sat_linear_deps() {
    let resources = vec!["A".into(), "B".into(), "C".into()];
    let deps = vec![("B".into(), "A".into()), ("C".into(), "B".into())];
    let problem = build_sat_problem(&resources, &deps);
    let result = solve(&problem);
    assert!(matches!(result, SatResult::Satisfiable { .. }));
}

#[test]
fn sat_no_deps() {
    let resources = vec!["X".into(), "Y".into()];
    let problem = build_sat_problem(&resources, &[]);
    match solve(&problem) {
        SatResult::Satisfiable { assignment } => {
            assert!(assignment["X"]);
            assert!(assignment["Y"]);
        }
        _ => panic!("expected satisfiable"),
    }
}

#[test]
fn sat_diamond() {
    let resources = vec!["A".into(), "B".into(), "C".into(), "D".into()];
    let deps = vec![
        ("B".into(), "A".into()),
        ("C".into(), "A".into()),
        ("D".into(), "B".into()),
        ("D".into(), "C".into()),
    ];
    let problem = build_sat_problem(&resources, &deps);
    assert!(matches!(solve(&problem), SatResult::Satisfiable { .. }));
}

#[test]
fn sat_contradiction() {
    let mut var_names = BTreeMap::new();
    var_names.insert(1, "A".into());
    let problem = SatProblem {
        num_vars: 1,
        clauses: vec![vec![1], vec![-1]],
        var_names,
    };
    assert!(matches!(solve(&problem), SatResult::Unsatisfiable { .. }));
}

#[test]
fn sat_problem_structure() {
    let resources = vec!["A".into(), "B".into()];
    let deps = vec![("B".into(), "A".into())];
    let problem = build_sat_problem(&resources, &deps);
    assert_eq!(problem.num_vars, 2);
    assert_eq!(problem.clauses.len(), 3); // 1 implication + 2 unit
}

#[test]
fn sat_single_resource() {
    let resources = vec!["solo".into()];
    let problem = build_sat_problem(&resources, &[]);
    match solve(&problem) {
        SatResult::Satisfiable { assignment } => assert!(assignment["solo"]),
        _ => panic!("expected satisfiable"),
    }
}

#[test]
fn sat_ten_linear() {
    let resources: Vec<String> = (0..10).map(|i| format!("r{i}")).collect();
    let deps: Vec<(String, String)> = (1..10)
        .map(|i| (format!("r{i}"), format!("r{}", i - 1)))
        .collect();
    let problem = build_sat_problem(&resources, &deps);
    match solve(&problem) {
        SatResult::Satisfiable { assignment } => {
            assert_eq!(assignment.len(), 10);
            assert!(assignment.values().all(|&v| v));
        }
        _ => panic!("expected satisfiable"),
    }
}

#[test]
fn sat_result_serde() {
    let result = SatResult::Satisfiable {
        assignment: BTreeMap::from([("A".into(), true)]),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("Satisfiable"));
}

#[test]
fn sat_unknown_dep_skipped() {
    let resources = vec!["A".into()];
    let deps = vec![("A".into(), "MISSING".into())];
    let problem = build_sat_problem(&resources, &deps);
    assert_eq!(problem.clauses.len(), 1); // only unit clause for A
}
