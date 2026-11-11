//! Unit tests for the plan-file apply path (Refs #358).
//!
//! The refusals live in `tests_apply_from_plan_checks.rs`.

use super::*;
use crate::core::types::PlanAction;

#[test]
fn prepare_config_reports_a_missing_file_rather_than_panicking() {
    let err = prepare_config(Path::new("/nonexistent/forjar.yaml"), None, None).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn every_action_has_a_distinct_sigil() {
    let all = [
        sigil(&PlanAction::Create),
        sigil(&PlanAction::Update),
        sigil(&PlanAction::Destroy),
        sigil(&PlanAction::NoOp),
    ];
    let unique: std::collections::HashSet<&&str> = all.iter().collect();
    assert_eq!(unique.len(), all.len(), "{all:?}");
}

/// Refs #358: the knob record defaults to "nothing requested", so a caller that
/// forgets a field gets the ordinary apply's default rather than an arbitrary
/// one. `retry: 0` and `parallel: false` are what `ApplyArgs` itself defaults
/// to, so a default `ApplyKnobs` and a bare `forjar apply` agree.
#[test]
fn the_default_knobs_request_nothing() {
    let k = ApplyKnobs::default();
    assert!(!k.force_unlock && !k.progress && !k.parallel && !k.rollback_on_failure);
    assert_eq!(k.retry, 0);
    assert_eq!(k.timeout_secs, None);
    assert_eq!(k.max_parallel, None);
    assert_eq!(k.resource_timeout, None);
}

/// forjar#432 (option 2): the saved-plan preflight says the drift gate did not
/// run. The line is printed by `run_plan_apply_gates`, which both the dry run
/// and the real apply pass through before they diverge.
#[test]
fn the_saved_plan_preflight_says_the_drift_gate_did_not_run() {
    use crate::cli::apply_from_plan_gates::DRIFT_GATE_NOT_RUN;
    assert_eq!(
        DRIFT_GATE_NOT_RUN,
        "drift gate: not run for saved plans (bound to state_hash instead)"
    );
    let src = include_str!("apply_from_plan_gates.rs");
    let body = src
        .split("pub(super) fn run_plan_apply_gates(")
        .nth(1)
        .expect("run_plan_apply_gates exists");
    assert!(
        body.contains("eprintln!(\"{DRIFT_GATE_NOT_RUN}\");"),
        "the preflight must print the notice on stderr, never on a stdout a --json stream owns"
    );
}
