//! Refs #358 — everything `apply --plan-file` refuses, and why.
//!
//! Split out of `apply_from_plan.rs` so the command reads as a sequence of
//! named refusals followed by one execution.
//!
//! # The shape of the mistake these checks keep making
//!
//! Twice now a SYNTACTIC predicate has stood in for a semantic one and been
//! evaded by an edit that changed the syntax without changing the meaning:
//!
//! * `plan_seal::check_body_partition` — "the counters must partition the change
//!   list" — evaded by emptying the list, because `0/0/0/0` partitions nothing
//!   perfectly well;
//! * `check_an_empty_body_is_honest` — "`plan.changes.is_empty()`" — evaded on a
//!   PARTIALLY CONVERGED stack (every real deployment) by deleting the one
//!   pending line and keeping an honest `no_op` line beside it. Measured:
//!   `Plan has no changes to apply.`, exit 0, the create still pending, every
//!   seal leg green.
//!
//! Both of those were special cases of a plan body that fails to name work the
//! planner says is pending. So there is no special case here any more:
//! [`check_plan_still_holds`] compares the body against a fresh plan in BOTH
//! directions and has nothing to say about emptiness at all.

use crate::core::executor::PlanScope;
use crate::core::plan_selectors::PlanSelectors;
use crate::core::types;
use std::collections::HashSet;

/// Prefix on every refusal that comes from re-planning rather than from the
/// seal. `plan_seal` owns `PLAN_HASH_MISMATCH` / `PLAN_MALFORMED`; a plan whose
/// integrity is perfect and whose content is a lie is a different failure and
/// says so.
pub(super) const PLAN_STALE: &str = "PLAN_STALE";

/// Refs #358: "this plan has no changes" is only an instruction worth obeying
/// when something vouches for the body that says it.
///
/// A `forjar-plan-v1` document's counters are unauthenticated JSON sitting
/// under a valid `config_hash`, so obeying a zero there is how a requested
/// apply prints a benign sentence and exits 0 having converged nothing — an
/// operator or CI job reading the exit code sees a successful apply over a
/// machine nothing was done to. A sealed body may legitimately say zero — but
/// only [`check_plan_still_holds`] decides whether it is TELLING THE TRUTH.
pub(super) fn check_empty_plan_is_trustworthy(sealed: bool) -> Result<(), String> {
    if sealed {
        return Ok(());
    }
    Err(format!(
        "this '{}' plan file reports no changes, but its body is unsealed — forjar will \
         not report a successful apply on the word of an unauthenticated counter. \
         Re-run `forjar plan --out` to write a sealed '{}' plan.",
        super::plan_file::FORMAT_V1,
        super::plan_file::FORMAT_V2,
    ))
}

/// Index a plan's changes by the `(machine, resource)` pair each one names.
fn by_pair(plan: &types::ExecutionPlan) -> Vec<((&str, &str), &types::PlanAction)> {
    plan.changes
        .iter()
        .map(|c| ((c.machine.as_str(), c.resource_id.as_str()), &c.action))
        .collect()
}

/// `resource on machine (ACTION)`, the phrasing every message here uses.
fn name_change(change: &types::PlannedChange) -> String {
    format!(
        "{} on {} ({})",
        change.resource_id, change.machine, change.action
    )
}

/// Refs #358 — the seal says a plan is UNEDITED. Re-planning says whether it is
/// TRUE, and only the second question is the one an operator is asking.
///
/// The seal is an unkeyed BLAKE3 hash. Anyone who can run `forjar` can compute
/// one, so no arrangement of hashing distinguishes a plan forjar issued from a
/// plan an adversary issued: copy `config_hash` and `state_hash` out of an
/// honest plan (neither leg has moved), rewrite the body, recompute the diff leg
/// and the composition through the public `plan_seal::digest` API, and every
/// check the seal can perform passes.
///
/// What is checkable with no secret at all is the plan's claim. A plan file
/// asserts a set of actions; the planner asserts one too, from the live config
/// and the live locks, and this command already holds both. Where they
/// disagree, the plan file is the one that is wrong — an adversary cannot make
/// the real planner return `NoOp` while a create is pending.
///
/// # Both directions, because either alone is evadable
///
/// `fresh` must be planned under the DOCUMENT'S OWN selectors (see
/// [`PlanSelectors`]); given that, an unedited plan and a fresh plan agree
/// exactly, because the seal has already pinned the config and the locks and
/// the planner is a pure function of the two.
///
/// * every pair the body NAMES must carry the action the planner gives it —
///   catches relabelling a pending create as `no_op`;
/// * every non-`NoOp` change the planner PRODUCES must be named by the body —
///   catches deleting that line instead of relabelling it, and catches emptying
///   the list entirely. Both were reported as separate defects; they are one
///   check.
pub(super) fn check_plan_still_holds(
    plan: &types::ExecutionPlan,
    fresh: &types::ExecutionPlan,
    selectors: &PlanSelectors,
) -> Result<(), String> {
    check_nothing_the_plan_names_has_moved(plan, fresh)?;
    check_nothing_pending_is_unnamed(plan, fresh, selectors)
}

/// Direction 1 — the body's claims, checked against the planner.
fn check_nothing_the_plan_names_has_moved(
    plan: &types::ExecutionPlan,
    fresh: &types::ExecutionPlan,
) -> Result<(), String> {
    let live = by_pair(fresh);
    for change in &plan.changes {
        let key = (change.machine.as_str(), change.resource_id.as_str());
        match live.iter().find(|(pair, _)| *pair == key).map(|(_, a)| *a) {
            Some(actual) if *actual == change.action => {}
            Some(actual) => {
                return Err(format!(
                    "{PLAN_STALE}: the plan file says {} for '{}' on '{}', but planning the \
                     live config against the live state says {actual}. A plan file is \
                     unauthenticated JSON — its seal proves it was not edited in transit, \
                     not that what it says is true — so forjar re-plans and believes the \
                     planner. Re-run `forjar plan --out` to write a plan that matches the \
                     world.",
                    change.action, change.resource_id, change.machine
                ));
            }
            None => {
                return Err(format!(
                    "{PLAN_STALE}: the plan file names '{}' on '{}', which planning the live \
                     config does not produce at all. Re-run `forjar plan --out`.",
                    change.resource_id, change.machine
                ));
            }
        }
    }
    Ok(())
}

/// Direction 2 — the planner's pending work, checked against the body.
///
/// This is the half the first two fixes were missing. A body that names nothing
/// and a body that names one honest `no_op` beside a deleted `create` are the
/// same lie told at different lengths, and both are caught here by the work
/// they fail to mention rather than by the shape of what they do mention.
fn check_nothing_pending_is_unnamed(
    plan: &types::ExecutionPlan,
    fresh: &types::ExecutionPlan,
    selectors: &PlanSelectors,
) -> Result<(), String> {
    let named: HashSet<(&str, &str)> = plan
        .changes
        .iter()
        .map(|c| (c.machine.as_str(), c.resource_id.as_str()))
        .collect();
    let unnamed: Vec<String> = fresh
        .changes
        .iter()
        .filter(|c| c.action != types::PlanAction::NoOp)
        .filter(|c| !named.contains(&(c.machine.as_str(), c.resource_id.as_str())))
        .map(name_change)
        .collect();
    if unnamed.is_empty() {
        return Ok(());
    }
    let under = match selectors.describe() {
        Some(flags) => format!(" under this plan's own filters ({flags})"),
        None => String::new(),
    };
    Err(format!(
        "{PLAN_STALE}: planning the live config against the live state{under} finds {} \
         change(s) this plan file does not name: {}. A sealed plan proves its body was not \
         edited in transit, not that its body is complete — obeying this one would print a \
         successful apply while leaving that work pending. Re-run `forjar plan --out`.",
        unnamed.len(),
        unnamed.join(", ")
    ))
}

/// Refs #358: what a plan leaves undone, said out loud.
///
/// The empty-scope path prints a benign sentence and exits 0, which is correct
/// for a plan that legitimately asks for nothing — a `plan -r bravo` over a
/// converged `bravo` is exactly that, and refusing it would break every
/// idempotent CI loop. It is also what an adversary wants, and the seal is
/// unkeyed, so a forgery CAN declare itself narrow and reach here honestly.
///
/// What it can no longer do is reach here quietly. A filtered plan that applies
/// nothing now names the work outside its filter, so the sentence an operator
/// reads is the whole truth rather than half of it.
pub(super) fn disclose_work_outside_the_filter(
    selectors: &PlanSelectors,
    whole: &types::ExecutionPlan,
) {
    let Some(flags) = selectors.describe() else {
        return;
    };
    let pending: Vec<String> = whole
        .changes
        .iter()
        .filter(|c| c.action != types::PlanAction::NoOp)
        .map(name_change)
        .collect();
    if pending.is_empty() {
        return;
    }
    eprintln!(
        "note: this plan is filtered ({flags}) and asks for nothing. {} change(s) OUTSIDE \
         its filter are still pending: {}. Nothing was applied to them.",
        pending.len(),
        pending.join(", ")
    );
}

/// Does the operator's own selector set keep this pair?
///
/// Mirrors `executor::resource_ops::resource_filtered_out` exactly, because the
/// executor is what will actually skip them — a second opinion here would mean
/// the pre-check and the run disagree about what an empty intersection is.
pub(super) fn survives(
    config: &types::ForjarConfig,
    selectors: &PlanSelectors,
    machine: &str,
    resource_id: &str,
) -> bool {
    if selectors.machine.as_deref().is_some_and(|m| m != machine) {
        return false;
    }
    if selectors
        .resource
        .as_deref()
        .is_some_and(|r| r != resource_id)
    {
        return false;
    }
    let Some(resource) = config.resources.get(resource_id) else {
        return false;
    };
    if let Some(tag) = selectors.tag.as_deref() {
        if !resource.tags.iter().any(|t| t == tag) {
            return false;
        }
    }
    if let Some(group) = selectors.group.as_deref() {
        if resource.resource_group.as_deref() != Some(group) {
            return false;
        }
    }
    true
}

/// Refs #358: `-m`, `-r`, `-t` and `-g` INTERSECT the reviewed scope.
///
/// `--plan-file` executes the delta that was REVIEWED, so a selector can only
/// narrow it — widening it would be the defect this command was fixed for, a
/// resource converged from a plan that never named it. Narrowing to part of a
/// reviewed plan is a legitimate staged rollout, and the executor already
/// intersects all four filters with the scope, so honouring them is just passing
/// them.
///
/// The EMPTY intersection is the case that earns a message. Converging nothing
/// and exiting 0 is how an operator asks for one machine, reads success, and
/// believes it happened — the same class of silent green as the forged
/// zero-change plan above. Erroring costs them one re-run and names what the
/// plan covers.
pub(super) fn check_selectors_narrow_the_plan(
    scope: &PlanScope,
    config: &types::ForjarConfig,
    selectors: &PlanSelectors,
) -> Result<(), String> {
    let Some(flags) = selectors.describe() else {
        return Ok(());
    };
    if scope
        .pairs()
        .any(|(m, r)| survives(config, selectors, m, r))
    {
        return Ok(());
    }
    let mut reviewed: Vec<String> = scope.pairs().map(|(m, r)| format!("{r} on {m}")).collect();
    reviewed.sort();
    Err(format!(
        "{flags} selects nothing this plan asked for — the reviewed plan covers: {}. \
         `--plan-file` executes the reviewed delta, so a selector can only narrow it, never \
         widen it; obeying this would converge nothing and still exit 0. Drop the selector, \
         name part of the plan, or re-run `forjar plan {flags} --out`.",
        reviewed.join(", ")
    ))
}

/// Refs #358: the three apply flags that would RE-PLAN under `--plan-file`.
///
/// `--force`, `--force-tag` and `--refresh` do not choose how the reviewed delta
/// executes; they change what the delta IS, by emptying or evicting the lock
/// entries the planner reads. On a plan apply each one is either defeated or
/// destructive, and neither outcome is something to do silently:
///
/// * `--force` defeats the scope outright. `PlanScope` demotes out-of-scope
///   changes to `NoOp` so that `triggers` still fire, and
///   `should_skip_single` skips a `NoOp` only `if !cfg.force` — so forcing a
///   scoped apply converges every resource on the plan's machines, reviewed or
///   not. That is #356's defect with a flag in front of it.
/// * `--force-tag` and `--refresh` evict lock entries BEFORE planning, so a
///   resource the operator reviewed as `update` can execute as `create`. The
///   staleness check re-plans without them and would not notice.
///
/// Refusing costs an operator one re-run and a `forjar apply --force` without
/// `--plan-file`. Ignoring them costs them the belief that a reviewed plan
/// executed.
pub(super) fn reject_replanning_flags(
    force: bool,
    refresh: bool,
    force_tag: Option<&str>,
) -> Result<(), String> {
    let supplied: Vec<&str> = [
        ("--force", force),
        ("--refresh", refresh),
        ("--force-tag", force_tag.is_some()),
    ]
    .into_iter()
    .filter_map(|(flag, on)| on.then_some(flag))
    .collect();
    if supplied.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{} cannot be used with --plan-file: {}.\nNothing was done. A saved plan is a \
         REVIEWED delta, and each of these changes what the delta is by clearing the lock \
         entries the planner reads — --force additionally defeats the plan's scope and \
         converges every resource on the plan's machines. Re-plan with the flag \
         (`forjar plan --out`) if you want it reviewed, or run `forjar apply` without \
         --plan-file if you do not.",
        if supplied.len() == 1 { "Flag" } else { "Flags" },
        supplied.join(", ")
    ))
}

#[cfg(test)]
#[path = "tests_apply_from_plan_checks.rs"]
mod tests_apply_from_plan_checks;
