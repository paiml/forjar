//! PMAT-160 (#466 #467 #468): one selection path for every apply mode.
//!
//! Three modes each derived their own resource set, and each got it wrong in a
//! different direction: `--dry-run -r x` rendered the UNSCOPED plan, `--check`
//! returned before the scope filters ran at all, and `--subset x` pruned the
//! config BEFORE the DAG was validated, so a `depends_on` the file declares
//! correctly came back as "depends on unknown". The three bugs share one cause:
//! selection was a sequence of independent prunes rather than a resolution.
//!
//! So it becomes one function with one order — validate, select, close, prune —
//! and the order IS the fix:
//!
//! * validating the FULL graph first means a declared dependency is never
//!   reported as unknown, and an undeclared one still is;
//! * closing the positive set over `depends_on` means a targeted apply cannot
//!   execute against a prerequisite that was never converged;
//! * contracting (rather than deleting) the edge to an explicitly excluded
//!   resource keeps the ordering the operator did not ask to lose.

use super::narrow::{contract_edges, drop_negatives, Dropped};
use super::reject_empty_selection;
use crate::core::{resolver, types};
use std::collections::HashSet;

/// Every resource-set selector `apply` accepts, resolved once.
///
/// `-m`/`--machine` is deliberately absent: it selects the executor and the
/// lock, not the resource set. `-t` is present but only existence-checked —
/// tags stay a plan-level filter downstream.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Selectors<'a> {
    /// `-r` / `--resource`: exact id (positive).
    pub resource: Option<&'a str>,
    /// `-g` / `--group` (positive).
    pub group: Option<&'a str>,
    /// `--subset` glob (positive).
    pub subset: Option<&'a str>,
    /// `--resource-filter` glob (positive, FJ-666).
    pub resource_filter: Option<&'a str>,
    /// `make` goals (positive).
    pub goals: &'a [String],
    /// `--exclude` glob (negative).
    pub exclude: Option<&'a str>,
    /// `--skip` id (negative).
    pub skip: Option<&'a str>,
    /// `--only-machine` (machine narrowing).
    pub only_machine: Option<&'a str>,
    /// `--exclude-machine` (machine narrowing).
    pub exclude_machine: Option<&'a str>,
    /// `-t`: existence-checked here, never pruned here.
    pub tag: Option<&'a str>,
}

/// What the resolver did, for the verbose lines and for tests.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Selection {
    /// Resources in the parsed config.
    pub total: usize,
    /// Matched by the positive selectors, before closure.
    pub selected: usize,
    /// Pulled in by the `depends_on` closure.
    pub dependencies_added: usize,
    /// Dropped by `--exclude` / `--skip` / machine narrowing.
    pub removed: Vec<String>,
    /// `(dependent, dependency)` edges contracted because the dependency went.
    pub cut_edges: Vec<(String, String)>,
}

/// Resolve every selector against the FULL parsed config, once, then prune.
///
/// The order below is the contract. Swapping any two steps reintroduces one of
/// #466, #467 or #468.
pub(crate) fn resolve_selection(
    config: &mut types::ForjarConfig,
    sel: &Selectors<'_>,
    verbose: bool,
) -> Result<Selection, String> {
    // 1. The unpruned graph is the one the operator wrote. Validate that one.
    resolver::build_execution_order(config)?;
    // 2. Every selector is checked against the full config, so "matches
    //    nothing" always means the invocation, never an earlier filter.
    check_existence(config, sel)?;

    let positive = positive_ids(config, sel)?;
    let closure = resolver::goal_closure(config, &positive)?;
    let mut out = Selection {
        total: config.resources.len(),
        selected: positive.len(),
        dependencies_added: closure.len().saturating_sub(positive.len()),
        removed: Vec::new(),
        cut_edges: Vec::new(),
    };

    let dropped = drop_negatives(config, sel, &closure, verbose)?;
    let removed: HashSet<String> = dropped.iter().map(|(id, _)| id.clone()).collect();
    let keep: Vec<String> = config
        .resources
        .keys()
        .filter(|id| closure.contains(*id) && !removed.contains(*id))
        .cloned()
        .collect();

    out.cut_edges = contract_edges(config, &keep, &removed);
    out.removed = dropped.iter().map(|(id, _)| id.clone()).collect();
    prune(config, &keep);
    if verbose {
        report(config, sel, &out, &dropped);
    }
    Ok(out)
}

// ── existence checks (step 2) ────────────────────────────────────────────────

/// Sorted, comma-joined key list for an error that names what IS available.
fn known(keys: impl Iterator<Item = String>) -> String {
    let mut v: Vec<String> = keys.collect();
    v.sort_unstable();
    v.join(", ")
}

fn check_existence(config: &types::ForjarConfig, sel: &Selectors<'_>) -> Result<(), String> {
    reject_empty_selection(config, sel.resource, sel.tag, sel.group)?;
    check_glob_selectors(config, sel)?;
    check_skip(config, sel.skip)?;
    check_machine(config, sel.only_machine, "--only-machine")?;
    check_machine(config, sel.exclude_machine, "--exclude-machine")?;
    // An unknown goal carries make's own message; a known one costs one walk.
    resolver::goal_closure(config, sel.goals)?;
    Ok(())
}

fn glob_matches_any(config: &types::ForjarConfig, pattern: &str) -> bool {
    config
        .resources
        .keys()
        .any(|id| crate::cli::helpers_state::simple_glob_match(pattern, id))
}

fn check_glob_selectors(config: &types::ForjarConfig, sel: &Selectors<'_>) -> Result<(), String> {
    if let Some(p) = sel.subset {
        if !glob_matches_any(config, p) {
            return Err(format!("no resources match subset pattern '{p}'"));
        }
    }
    if let Some(p) = sel.resource_filter {
        if !glob_matches_any(config, p) {
            return Err(format!(
                "--resource-filter: no resources match subset pattern '{p}'"
            ));
        }
    }
    Ok(())
}

fn check_skip(config: &types::ForjarConfig, skip: Option<&str>) -> Result<(), String> {
    let Some(id) = skip else { return Ok(()) };
    if config.resources.contains_key(id) {
        return Ok(());
    }
    Err(format!(
        "--skip '{id}' matches no resource in this config. Known: {}",
        known(config.resources.keys().cloned())
    ))
}

fn check_machine(
    config: &types::ForjarConfig,
    machine: Option<&str>,
    flag: &str,
) -> Result<(), String> {
    let Some(m) = machine else { return Ok(()) };
    if config.machines.contains_key(m) {
        return Ok(());
    }
    Err(format!(
        "{flag} '{m}' matches no machine in this config. Known: {}",
        known(config.machines.keys().cloned())
    ))
}

// ── the positive set (step 3) ────────────────────────────────────────────────

impl Selectors<'_> {
    /// True when at least one positive selector was supplied.
    fn has_positive(&self) -> bool {
        self.resource.is_some()
            || self.group.is_some()
            || self.subset.is_some()
            || self.resource_filter.is_some()
            || !self.goals.is_empty()
    }

    /// The positive selectors as the operator typed them, for an error that
    /// says which COMBINATION matched nothing.
    fn describe_positive(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        push_flag(&mut parts, "--resource", self.resource);
        push_flag(&mut parts, "--group", self.group);
        push_flag(&mut parts, "--subset", self.subset);
        push_flag(&mut parts, "--resource-filter", self.resource_filter);
        if !self.goals.is_empty() {
            parts.push(format!("goals {:?}", self.goals));
        }
        parts.join(", ")
    }
}

fn push_flag(parts: &mut Vec<String>, flag: &str, value: Option<&str>) {
    if let Some(v) = value {
        parts.push(format!("{flag} '{v}'"));
    }
}

/// Resources matching ALL supplied positive selectors — the whole config when
/// none was supplied.
fn positive_ids(config: &types::ForjarConfig, sel: &Selectors<'_>) -> Result<Vec<String>, String> {
    if !sel.has_positive() {
        return Ok(config.resources.keys().cloned().collect());
    }
    let ids: Vec<String> = config
        .resources
        .iter()
        .filter(|(id, r)| matches_ids(id, r, sel) && matches_globs(id, sel))
        .map(|(id, _)| id.clone())
        .collect();
    if ids.is_empty() {
        // Each selector matched something on its own — the INTERSECTION is what
        // is empty, so name the combination rather than blame one flag.
        return Err(format!(
            "no resources match the selectors: {}",
            sel.describe_positive()
        ));
    }
    Ok(ids)
}

fn matches_ids(id: &str, r: &types::Resource, sel: &Selectors<'_>) -> bool {
    sel.resource.is_none_or(|want| want == id)
        && sel
            .group
            .is_none_or(|g| r.resource_group.as_deref() == Some(g))
        && (sel.goals.is_empty() || sel.goals.iter().any(|g| g == id))
}

fn matches_globs(id: &str, sel: &Selectors<'_>) -> bool {
    let matches = |p: &str| crate::cli::helpers_state::simple_glob_match(p, id);
    sel.subset.is_none_or(matches) && sel.resource_filter.is_none_or(matches)
}

/// Prune to the kept set, preserving the config's original order.
fn prune(config: &mut types::ForjarConfig, keep: &[String]) {
    let set: HashSet<&str> = keep.iter().map(String::as_str).collect();
    config.resources.retain(|id, _| set.contains(id.as_str()));
}

// ── verbose reporting (step 7) ───────────────────────────────────────────────

/// `", +2 dependencies"`, or nothing when the closure added none — so the
/// existing selector lines stay byte-for-byte what they were.
fn added_suffix(out: &Selection) -> String {
    if out.dependencies_added == 0 {
        return String::new();
    }
    format!(", +{} dependencies", out.dependencies_added)
}

fn report(config: &types::ForjarConfig, sel: &Selectors<'_>, out: &Selection, dropped: &Dropped) {
    report_positive(config, sel, out);
    report_negative(config, sel, out, dropped);
    report_cut_edges(out, dropped);
}

fn report_positive(config: &types::ForjarConfig, sel: &Selectors<'_>, out: &Selection) {
    let suffix = added_suffix(out);
    if let Some(p) = sel.subset {
        eprintln!(
            "Subset filter '{p}': {} resources selected{suffix}",
            out.selected
        );
    }
    // `-r` and `-g` printed nothing before, and still print nothing unless the
    // closure changed the answer — that change is the part worth saying.
    if out.dependencies_added > 0 {
        if let Some(id) = sel.resource {
            eprintln!("Resource '{id}': {} selected{suffix}", out.selected);
        }
        if let Some(g) = sel.group {
            eprintln!("Group '{g}': {} selected{suffix}", out.selected);
        }
    }
    if !sel.goals.is_empty() {
        eprintln!(
            "Goals {:?}: {} of {} resources in the prerequisite closure",
            sel.goals,
            config.resources.len(),
            out.total
        );
    }
}

fn report_negative(
    config: &types::ForjarConfig,
    sel: &Selectors<'_>,
    out: &Selection,
    dropped: &Dropped,
) {
    if let Some(p) = sel.exclude {
        let cause = format!("--exclude '{p}'");
        let n = dropped.iter().filter(|(_, c)| *c == cause).count();
        eprintln!(
            "Exclude filter '{}': removed {} resources ({} remaining)",
            p,
            n,
            config.resources.len()
        );
    }
    if scope_selector_used(sel) {
        eprintln!(
            "Scope selectors: {} resources selected",
            out.total - out.removed.len()
        );
    }
}

/// The four `ApplyScope` selectors, which share one summary line.
fn scope_selector_used(sel: &Selectors<'_>) -> bool {
    sel.skip.is_some()
        || sel.only_machine.is_some()
        || sel.exclude_machine.is_some()
        || sel.resource_filter.is_some()
}

/// One line per contracted edge. A dependent that keeps running without a
/// prerequisite the operator removed is a decision worth printing.
fn report_cut_edges(out: &Selection, dropped: &Dropped) {
    for (dependent, dependency) in &out.cut_edges {
        let cause = dropped
            .iter()
            .find(|(id, _)| id == dependency)
            .map_or("selection", |(_, c)| c.as_str());
        eprintln!(
            "{cause}: '{dependent}' depends on it; running '{dependent}' with '{dependency}' assumed satisfied"
        );
    }
}
