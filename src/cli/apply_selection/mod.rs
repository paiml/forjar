//! FJ-2723 / FJ-2724 (PMAT-199): resource selection for `apply` and `make`.
//!
//! Split out of `apply.rs` to keep it under the 500-line limit.
//!
//! PMAT-160 (#466 #467 #468): `resolve_selection` in [`closure`] is THE path.
//! It replaced three independent prunes — `apply_goal_closure` (make-style
//! goals), `apply_filters` (`--subset`/`--exclude`) and `apply_scope`'s four
//! selectors — which each ran at a different point of `cmd_apply_scoped` and
//! could not run at all for `--check`. Every apply mode now resolves once, in
//! one order, against the config the operator wrote.
//!
//! Two helpers survive them, because they answer questions the resolver does
//! not: `reject_empty_selection` (a selector naming nothing is a mistake in the
//! invocation, and `check_existence` calls it) and `strip_unrequested_phony`
//! (goal-only phony resources, which `plan` and the MCP layer share).

use crate::core::types;

mod closure;
mod narrow;

/// The resolver's own report, asserted by `tests_apply_selection_closure`.
/// Nothing in the shipped path reads it — the config it returns IS the answer.
#[cfg(test)]
pub(crate) use closure::Selection;
pub(crate) use closure::{resolve_selection, Selectors};

/// FJ-2723 (PMAT-199): a selector that matches nothing is an error, not a no-op.
///
/// `forjar apply -r <typo>` used to print `0 converged, 0 unchanged` and exit
/// 0. Every signal said success while nothing had been applied — the same
/// silent-green shape as a check that always passes, and worse in CI, where the
/// exit code is the only thing anyone reads. A selector naming something that
/// does not exist is a mistake in the invocation, and saying so costs one line.
pub(crate) fn reject_empty_selection(
    config: &types::ForjarConfig,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    group_filter: Option<&str>,
) -> Result<(), String> {
    if let Some(id) = resource_filter {
        if !config.resources.contains_key(id) {
            let mut known: Vec<&str> = config.resources.keys().map(String::as_str).collect();
            known.sort_unstable();
            return Err(format!(
                "--resource '{id}' matches no resource in this config. Known: {}",
                known.join(", ")
            ));
        }
    }
    if let Some(tag) = tag_filter {
        if !config
            .resources
            .values()
            .any(|r| r.tags.iter().any(|t| t == tag))
        {
            return Err(format!("--tag '{tag}' matches no resource in this config"));
        }
    }
    if let Some(group) = group_filter {
        if !config
            .resources
            .values()
            .any(|r| r.resource_group.as_deref() == Some(group))
        {
            return Err(format!(
                "--group '{group}' matches no resource in this config"
            ));
        }
    }
    Ok(())
}

/// FJ-2725 (PMAT-199): remove phony resources that were not explicitly requested.
///
/// A phony resource names an ACTION (`clean`, `test`, `all`), not a file. It has
/// no artifact, so there is nothing to observe and nothing to converge to.
/// forjar's answer is goal-only: a phony resource participates in an apply only
/// when it is named as a goal, and then it runs unconditionally.
///
/// # Why not "runs on every apply", the naive reading of make
///
/// `planner::propagation` promotes every NoOp dependent of a changed resource,
/// so an always-changed phony would rebuild its entire transitive closure on
/// every single apply. It would also mean `forjar plan` could never again print
/// "0 to change" for any config containing one, breaking the
/// `idempotent-apply-v1` plan-fixed-point contract and poisoning every drift
/// lane that reads a non-empty plan as drift. make does not have this problem
/// because it decides per target from the filesystem rather than by propagating
/// dirtiness along edges.
///
/// # Why not "phony prerequisites auto-run when reached", make's real rule
///
/// That is not convergent here. Let `build` depend on phony `clean`, which
/// deletes build's artifacts: apply #2 sees the outputs missing, plans `build`,
/// pulls in `clean`, deletes them again — `f(f(x)) != f(x)`, permanently. And
/// it would not even work: probes are computed once before planning, so a
/// prerequisite's side effects during an apply are invisible to that apply's
/// plan.
///
/// Goal-only makes the plan action a constant function of the config —
/// independent of lock, probe and history — so idempotency holds by
/// construction rather than by argument. Dropping the edge to an unrequested
/// phony resource IS the "ordering only, never auto-run" rule.
pub(crate) fn strip_unrequested_phony(config: &mut types::ForjarConfig, goals: &[String]) {
    let dropped: Vec<String> = config
        .resources
        .iter()
        .filter(|(id, r)| r.phony && !goals.iter().any(|g| g == *id))
        .map(|(id, _)| id.clone())
        .collect();

    if dropped.is_empty() {
        return;
    }
    for id in &dropped {
        config.resources.shift_remove(id);
    }
    // Scrub edges to the removed resources so the DAG stays well-formed.
    for resource in config.resources.values_mut() {
        resource.depends_on.retain(|d| !dropped.contains(d));
        resource.triggers.retain(|t| !dropped.contains(t));
        resource.restart_on.retain(|t| !dropped.contains(t));
    }
}
