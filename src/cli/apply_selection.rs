//! FJ-2723 / FJ-2724 (PMAT-199): resource selection for `apply` and `make`.
//!
//! Split out of `apply.rs` to keep it under the 500-line limit. Three ways to
//! narrow an apply live here, and they are not interchangeable:
//!
//! * `reject_empty_selection` — a selector naming nothing is a mistake in the
//!   invocation, not a request to do nothing.
//! * `apply_goal_closure` — `make`-style: the goals plus everything they need.
//!   Downward-closed, so it can never strand a prerequisite.
//! * `apply_filters` — `--subset`/`--exclude` pattern filters, which CAN cut a
//!   resource out from under a dependent. That is why `make` does not use them.

use crate::core::{resolver, types};

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

/// FJ-2724 (PMAT-199): prune the config to the goals' prerequisite closure.
///
/// This is what makes `forjar make <goal>` mean what `make <goal>` means. The
/// prune happens after param overrides and before every other filter, so
/// `make a --exclude 'x-*'` composes.
///
/// Pruning is safe here in a way `--subset` is not: a `depends_on` closure is
/// downward-closed, so the pruned config can never execute a resource whose
/// prerequisites were dropped. It also cannot produce a spurious Destroy — the
/// plan iterates the execution order derived from `config.resources`, so lock
/// entries with no config resource are simply never visited.
pub(crate) fn apply_goal_closure(
    config: &mut types::ForjarConfig,
    goals: &[String],
    verbose: bool,
) -> Result<(), String> {
    if goals.is_empty() {
        return Ok(());
    }
    let keep = resolver::goal_closure(config, goals)?;
    let before = config.resources.len();
    config.resources.retain(|id, _| keep.contains(id));
    if verbose {
        eprintln!(
            "Goals {:?}: {} of {} resources in the prerequisite closure",
            goals,
            config.resources.len(),
            before
        );
    }
    Ok(())
}

/// Apply subset and exclude filters to config.
pub(crate) fn apply_filters(
    config: &mut types::ForjarConfig,
    subset: Option<&str>,
    exclude: Option<&str>,
    verbose: bool,
) -> Result<(), String> {
    if let Some(pattern) = subset {
        let count = super::apply_gates::filter_subset(&mut config.resources, pattern)?;
        if verbose {
            eprintln!("Subset filter '{pattern}': {count} resources selected");
        }
    }
    if let Some(pattern) = exclude {
        let removed = super::apply_gates::filter_exclude(&mut config.resources, pattern);
        if verbose {
            eprintln!(
                "Exclude filter '{}': removed {} resources ({} remaining)",
                pattern,
                removed,
                config.resources.len()
            );
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
