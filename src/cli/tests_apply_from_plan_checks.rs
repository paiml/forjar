//! Unit tests for what `apply --plan-file` refuses (Refs #358).

use super::*;
use crate::core::types::{PlanAction, PlannedChange, Resource, ResourceType};

#[test]
fn a_sealed_plan_may_legitimately_report_no_changes() {
    assert!(check_empty_plan_is_trustworthy(true).is_ok());
}

#[test]
fn an_unsealed_plan_reporting_no_changes_is_refused() {
    let err = check_empty_plan_is_trustworthy(false).unwrap_err();
    assert!(err.contains(super::super::plan_file::FORMAT_V1), "{err}");
    assert!(err.contains(super::super::plan_file::FORMAT_V2), "{err}");
    assert!(err.contains("unauthenticated counter"), "{err}");
}

fn change(id: &str, machine: &str, action: PlanAction) -> PlannedChange {
    PlannedChange {
        resource_id: id.to_string(),
        machine: machine.to_string(),
        resource_type: ResourceType::File,
        action,
        description: format!("{id}: planned"),
    }
}

fn plan_of(changes: Vec<PlannedChange>) -> types::ExecutionPlan {
    let mut plan = types::ExecutionPlan {
        name: "t".to_string(),
        execution_order: changes.iter().map(|c| c.resource_id.clone()).collect(),
        changes,
        to_create: 0,
        to_update: 0,
        to_destroy: 0,
        unchanged: 0,
    };
    for c in &plan.changes {
        match c.action {
            PlanAction::Create => plan.to_create += 1,
            PlanAction::Update => plan.to_update += 1,
            PlanAction::Destroy => plan.to_destroy += 1,
            PlanAction::NoOp => plan.unchanged += 1,
        }
    }
    plan
}

/// A two-resource config: `alpha` tagged `t_alpha` in group `ga`, `bravo`
/// tagged `t_bravo` in group `gb`.
fn config_of() -> types::ForjarConfig {
    let mut config = types::ForjarConfig::default();
    for (id, tag, group) in [("alpha", "t_alpha", "ga"), ("bravo", "t_bravo", "gb")] {
        config.resources.insert(
            id.to_string(),
            Resource {
                tags: vec![tag.to_string()],
                resource_group: Some(group.to_string()),
                ..Default::default()
            },
        );
    }
    config
}

fn unfiltered() -> PlanSelectors {
    PlanSelectors::default()
}

// ── check_plan_still_holds ──

/// THE ADVERSARY, at the unit level: an empty body over a world with a create
/// pending. Every leg of a re-sealed plan verifies; this is the check that does
/// not.
#[test]
fn an_empty_body_is_refused_when_the_planner_still_has_work() {
    let fresh = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    let err = check_plan_still_holds(&plan_of(vec![]), &fresh, &unfiltered()).unwrap_err();
    assert!(err.starts_with(PLAN_STALE), "{err}");
    assert!(err.contains("alpha on web"), "{err}");
}

/// THE SECOND ADVERSARY (Refs #358, round three): the shape that survives an
/// "is the body empty?" test AND a "is the scope empty?" test that only fires
/// on an empty scope. On a partially converged stack, delete the one pending
/// line and keep the honest `no_op` line beside it. The counters still
/// partition (0/0/0/1) and `PlanScope::from_plan` skips `NoOp`, so every
/// emptiness predicate in the file is silent.
///
/// Measured before the fix, against the built binary: `Plan has no changes to
/// apply.`, exit 0, the create still pending.
#[test]
fn a_deleted_pending_line_is_refused_even_beside_an_honest_no_op() {
    let fresh = plan_of(vec![
        change("alpha", "web", PlanAction::Create),
        change("bravo", "web", PlanAction::NoOp),
    ]);
    let lie = plan_of(vec![change("bravo", "web", PlanAction::NoOp)]);
    let err = check_plan_still_holds(&lie, &fresh, &unfiltered()).unwrap_err();
    assert!(err.starts_with(PLAN_STALE), "{err}");
    assert!(err.contains("alpha on web"), "{err}");
    assert!(err.contains("does not name"), "{err}");
}

/// The same deletion where the plan is NOT empty afterwards — a two-change plan
/// with one line removed. Nothing about this is "empty", so only the
/// planner-to-body direction can see it.
#[test]
fn a_deleted_line_out_of_a_still_busy_plan_is_refused() {
    let fresh = plan_of(vec![
        change("alpha", "web", PlanAction::Create),
        change("bravo", "web", PlanAction::Update),
    ]);
    let lie = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    let err = check_plan_still_holds(&lie, &fresh, &unfiltered()).unwrap_err();
    assert!(err.contains("bravo on web"), "{err}");
}

/// Every omission is reported in ONE run, not the first one found: an operator
/// whose plan is missing four lines should learn that once.
#[test]
fn every_unnamed_pending_change_is_named_in_one_refusal() {
    let fresh = plan_of(vec![
        change("alpha", "web", PlanAction::Create),
        change("bravo", "web", PlanAction::Update),
        change("charlie", "db", PlanAction::Destroy),
    ]);
    let err = check_plan_still_holds(&plan_of(vec![]), &fresh, &unfiltered()).unwrap_err();
    for pending in ["alpha on web", "bravo on web", "charlie on db"] {
        assert!(err.contains(pending), "{pending} missing from: {err}");
    }
    assert!(err.contains("3 change(s)"), "{err}");
}

/// The same attack with the list KEPT and the pending create relabelled, so the
/// counters still partition their list and `check_body_partition` is silent.
#[test]
fn a_create_relabelled_as_a_no_op_is_refused() {
    let fresh = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    let lie = plan_of(vec![change("alpha", "web", PlanAction::NoOp)]);
    let err = check_plan_still_holds(&lie, &fresh, &unfiltered()).unwrap_err();
    assert!(err.starts_with(PLAN_STALE), "{err}");
    assert!(err.contains("NO-OP") && err.contains("CREATE"), "{err}");
}

/// A plan naming a pair the planner does not produce at all.
#[test]
fn a_pair_the_planner_does_not_produce_is_refused() {
    let fresh = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    let lie = plan_of(vec![change("ghost", "web", PlanAction::Create)]);
    let err = check_plan_still_holds(&lie, &fresh, &unfiltered()).unwrap_err();
    assert!(err.starts_with(PLAN_STALE), "{err}");
    assert!(err.contains("ghost"), "{err}");
}

/// GREEN GUARD: a FILTERED plan is narrower than the config by design and must
/// not be refused for it — but the comparison only knows that because the plan
/// file RECORDS its filters and the caller re-plans under them. `fresh` here is
/// what `plan -r alpha` produces, not what the whole config produces.
#[test]
fn a_plan_narrower_than_the_config_is_honoured_under_its_own_selectors() {
    let fresh = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    let filtered = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    let sel = PlanSelectors::new(None, Some("alpha"), None, None);
    assert!(check_plan_still_holds(&filtered, &fresh, &sel).is_ok());
}

/// The refusal SAYS it re-planned under the document's own filters, because a
/// message that named work outside a filter as "missing" would send an operator
/// hunting for an edit that never happened.
#[test]
fn a_filtered_plan_that_is_stale_says_which_filters_were_used() {
    let fresh = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    let sel = PlanSelectors::new(None, Some("alpha"), None, None);
    let err = check_plan_still_holds(&plan_of(vec![]), &fresh, &sel).unwrap_err();
    assert!(err.contains("-r alpha"), "{err}");
    assert!(err.contains("own filters"), "{err}");
}

/// GREEN GUARD: an honest zero — a converged stack still LISTS its resources as
/// `NoOp`, and the planner agrees.
#[test]
fn an_honest_converged_plan_is_honoured() {
    let converged = plan_of(vec![change("alpha", "web", PlanAction::NoOp)]);
    assert!(check_plan_still_holds(&converged, &converged, &unfiltered()).is_ok());
}

/// GREEN GUARD: an empty body over an empty world is honestly empty.
#[test]
fn an_empty_body_over_an_empty_world_is_honoured() {
    assert!(check_plan_still_holds(&plan_of(vec![]), &plan_of(vec![]), &unfiltered()).is_ok());
}

/// A world holding only no-ops is not "pending", so an empty body over it is
/// not a lie about anything that would have been done.
#[test]
fn an_empty_body_over_a_world_of_no_ops_is_honoured() {
    let fresh = plan_of(vec![change("alpha", "web", PlanAction::NoOp)]);
    assert!(check_plan_still_holds(&plan_of(vec![]), &fresh, &unfiltered()).is_ok());
}

// ── check_selectors_narrow_the_plan ──

fn scope_of(changes: Vec<PlannedChange>) -> PlanScope {
    PlanScope::from_plan(&plan_of(changes))
}

fn both_pending() -> PlanScope {
    scope_of(vec![
        change("alpha", "web", PlanAction::Create),
        change("bravo", "db", PlanAction::Create),
    ])
}

#[test]
fn no_selector_is_always_in_scope() {
    let scope = scope_of(vec![change("alpha", "web", PlanAction::Create)]);
    assert!(check_selectors_narrow_the_plan(&scope, &config_of(), &unfiltered()).is_ok());
}

#[test]
fn a_machine_the_plan_names_narrows_it() {
    let sel = PlanSelectors::new(Some("web"), None, None, None);
    assert!(check_selectors_narrow_the_plan(&both_pending(), &config_of(), &sel).is_ok());
}

#[test]
fn a_machine_outside_the_plan_is_an_error_naming_what_is_covered() {
    let sel = PlanSelectors::new(Some("ghost"), None, None, None);
    let err = check_selectors_narrow_the_plan(&both_pending(), &config_of(), &sel).unwrap_err();
    assert!(err.contains("ghost"), "{err}");
    assert!(err.contains("alpha on web"), "the plan's pairs: {err}");
    assert!(err.contains("narrow"), "{err}");
}

/// A machine whose only change is a `NoOp` is not IN the scope — the plan asked
/// for nothing there — so `-m` on it is refused rather than silently green.
#[test]
fn a_machine_the_plan_only_no_ops_is_not_in_scope() {
    let scope = scope_of(vec![
        change("alpha", "web", PlanAction::Create),
        change("bravo", "db", PlanAction::NoOp),
    ]);
    let sel = PlanSelectors::new(Some("db"), None, None, None);
    assert!(check_selectors_narrow_the_plan(&scope, &config_of(), &sel).is_err());
}

/// Refs #358: `-r` was DROPPED. `apply --plan-file --yes -r alpha` converged
/// `bravo` as well, at exit 0.
#[test]
fn a_resource_the_plan_names_narrows_it() {
    let sel = PlanSelectors::new(None, Some("alpha"), None, None);
    assert!(check_selectors_narrow_the_plan(&both_pending(), &config_of(), &sel).is_ok());
}

#[test]
fn a_resource_outside_the_plan_is_refused() {
    let scope = scope_of(vec![change("alpha", "web", PlanAction::Create)]);
    let sel = PlanSelectors::new(None, Some("bravo"), None, None);
    let err = check_selectors_narrow_the_plan(&scope, &config_of(), &sel).unwrap_err();
    assert!(err.contains("-r bravo"), "{err}");
}

#[test]
fn a_tag_the_plan_covers_narrows_it_and_one_it_does_not_is_refused() {
    let cfg = config_of();
    let keep = PlanSelectors::new(None, None, Some("t_alpha"), None);
    assert!(check_selectors_narrow_the_plan(&both_pending(), &cfg, &keep).is_ok());
    let miss = PlanSelectors::new(None, None, Some("nosuchtag"), None);
    assert!(check_selectors_narrow_the_plan(&both_pending(), &cfg, &miss).is_err());
}

#[test]
fn a_group_the_plan_covers_narrows_it_and_one_it_does_not_is_refused() {
    let cfg = config_of();
    let keep = PlanSelectors::new(None, None, None, Some("gb"));
    assert!(check_selectors_narrow_the_plan(&both_pending(), &cfg, &keep).is_ok());
    let miss = PlanSelectors::new(None, None, None, Some("nosuchgroup"));
    assert!(check_selectors_narrow_the_plan(&both_pending(), &cfg, &miss).is_err());
}

/// Selectors intersect each other, not just the plan: `-m web -r bravo` selects
/// nothing even though `web` and `bravo` are each in the plan on their own.
#[test]
fn selectors_intersect_with_one_another() {
    let cfg = config_of();
    let sel = PlanSelectors::new(Some("web"), Some("bravo"), None, None);
    assert!(check_selectors_narrow_the_plan(&both_pending(), &cfg, &sel).is_err());
}

/// `survives` must agree with the executor's own predicate about a resource the
/// config does not contain: the executor cannot look one up, so it skips it.
#[test]
fn a_pair_naming_no_configured_resource_survives_nothing() {
    assert!(!survives(&config_of(), &unfiltered(), "web", "ghost"));
}

// ── reject_replanning_flags ──

#[test]
fn a_plan_apply_with_no_replanning_flags_is_not_refused() {
    assert!(reject_replanning_flags(false, false, None).is_ok());
}

/// `--force` would defeat the scope entirely: `restrict` demotes out-of-scope
/// changes to `NoOp`, and `should_skip_single` skips a `NoOp` only when force is
/// off. Forcing a scoped apply converges every resource on the plan's machines.
#[test]
fn force_is_refused_on_a_plan_apply() {
    let err = reject_replanning_flags(true, false, None).unwrap_err();
    assert!(err.contains("--force"), "{err}");
    assert!(err.contains("Nothing was done"), "{err}");
    assert!(err.starts_with("Flag "), "singular form expected: {err}");
}

#[test]
fn refresh_and_force_tag_are_refused_and_reported_together() {
    let err = reject_replanning_flags(true, true, Some("service")).unwrap_err();
    for flag in ["--force", "--refresh", "--force-tag"] {
        assert!(err.contains(flag), "{flag} missing from: {err}");
    }
    assert!(err.starts_with("Flags "), "plural form expected: {err}");
}

// ── disclose_work_outside_the_filter ──

/// An UNFILTERED plan has no "outside", so there is nothing to disclose and the
/// function must not invent a note about the whole config.
#[test]
fn an_unfiltered_empty_plan_discloses_nothing() {
    let whole = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    disclose_work_outside_the_filter(&unfiltered(), &whole);
}

/// The disclosure is a side channel by nature (stderr), so the test that it is
/// reached is the exit-code test in
/// `tests/falsification_plan_body_cannot_omit_pending_work.rs`; here it only has
/// to be total over both shapes without panicking.
#[test]
fn a_filtered_empty_plan_discloses_over_any_world() {
    let sel = PlanSelectors::new(None, Some("bravo"), None, None);
    disclose_work_outside_the_filter(&sel, &plan_of(vec![]));
    disclose_work_outside_the_filter(
        &sel,
        &plan_of(vec![change("alpha", "web", PlanAction::Create)]),
    );
}
