//! PMAT-160: the negative half of selection — removals, and the edge
//! contraction that keeps the survivors well-formed.
//!
//! A positive selector says what to touch and closes downward, so it can never
//! strand a prerequisite. A negative one (`--exclude`, `--skip`,
//! `--only-machine`, `--exclude-machine`) can, and that is the operator's
//! decision: the dependent still runs. What must not happen is the dependent
//! keeping an edge to a resource that is no longer in the config — that is
//! exactly the "depends on unknown" that #468 reported — or silently losing the
//! ordering the removed node was standing in for.

use super::closure::Selectors;
use crate::core::types;
use std::collections::{HashMap, HashSet};

/// Each dropped resource paired with the flag that dropped it, e.g.
/// `("b", "--skip 'b'")`. The flag is what the verbose line quotes back.
pub(super) type Dropped = Vec<(String, String)>;

fn push_dropped(dropped: &mut Dropped, id: String, cause: &str) {
    if dropped.iter().any(|(d, _)| *d == id) {
        return;
    }
    dropped.push((id, cause.to_string()));
}

/// Collect the closure members a negative selector removes, in config order.
fn collect_dropped(
    config: &types::ForjarConfig,
    closure: &HashSet<String>,
    dropped: &mut Dropped,
    cause: &str,
    matches: impl Fn(&str) -> bool,
) {
    let hits: Vec<String> = config
        .resources
        .keys()
        .filter(|id| closure.contains(*id) && matches(id))
        .cloned()
        .collect();
    for id in hits {
        push_dropped(dropped, id, cause);
    }
}

/// Apply every negative selector to the closure, in a fixed order.
pub(super) fn drop_negatives(
    config: &mut types::ForjarConfig,
    sel: &Selectors<'_>,
    closure: &HashSet<String>,
    verbose: bool,
) -> Result<Dropped, String> {
    let mut dropped: Dropped = Vec::new();
    if let Some(p) = sel.exclude {
        let cause = format!("--exclude '{p}'");
        collect_dropped(config, closure, &mut dropped, &cause, |id| {
            crate::cli::helpers_state::simple_glob_match(p, id)
        });
    }
    if let Some(s) = sel.skip {
        let cause = format!("--skip '{s}'");
        collect_dropped(config, closure, &mut dropped, &cause, |id| id == s);
    }
    if let Some(m) = sel.only_machine {
        narrow_only_machine(config, m, closure, &mut dropped)?;
    }
    if let Some(m) = sel.exclude_machine {
        narrow_exclude_machine(config, m, closure, &mut dropped, verbose);
    }
    Ok(dropped)
}

/// FJ-736: `--only-machine` narrows the resource, not just the selection.
///
/// Retaining a `machine: [a, b]` resource under `--only-machine a` and applying
/// it unchanged would still touch `b`, so the survivor's target list is
/// rewritten. That is what makes the frame obligation hold literally.
fn narrow_only_machine(
    config: &mut types::ForjarConfig,
    machine: &str,
    closure: &HashSet<String>,
    dropped: &mut Dropped,
) -> Result<(), String> {
    let cause = format!("--only-machine '{machine}'");
    let mut kept_any = false;
    for (id, r) in config.resources.iter_mut() {
        if !closure.contains(id.as_str()) || dropped.iter().any(|(d, _)| d == id) {
            continue;
        }
        if r.machine.iter().any(|m| m == machine) {
            r.machine = types::MachineTarget::Single(machine.to_string());
            kept_any = true;
        } else {
            dropped.push((id.clone(), cause.clone()));
        }
    }
    if kept_any {
        return Ok(());
    }
    Err(format!(
        "--only-machine '{machine}' matches no resource in this config"
    ))
}

/// FJ-726: `--exclude-machine` applies to everything except one machine.
///
/// An empty result is legitimate here in a way it never is for a positive
/// selector: excluding the only machine SHOULD converge nothing. It is
/// announced rather than refused — the shipped bug was the opposite, asking for
/// nothing and getting everything.
fn narrow_exclude_machine(
    config: &mut types::ForjarConfig,
    machine: &str,
    closure: &HashSet<String>,
    dropped: &mut Dropped,
    verbose: bool,
) {
    let cause = format!("--exclude-machine '{machine}'");
    let mut kept_any = false;
    for (id, r) in config.resources.iter_mut() {
        if !closure.contains(id.as_str()) {
            continue;
        }
        let rest: Vec<String> = r
            .machine
            .iter()
            .filter(|m| *m != machine)
            .map(str::to_string)
            .collect();
        r.machine = types::MachineTarget::Multiple(rest);
        if r.machine.is_empty() {
            push_dropped(dropped, id.clone(), &cause);
        } else {
            kept_any = true;
        }
    }
    if !kept_any && verbose {
        eprintln!("--exclude-machine '{machine}': no resources remain to apply");
    }
}

/// Replace every edge to a removed resource with that resource's own kept
/// dependencies, transitively: `a -> b -> c` with `b` removed becomes `a -> c`.
///
/// Deleting the edge instead would lose the ordering; keeping it would leave a
/// dangling `depends_on`. Contraction is the only option that preserves both
/// the operator's removal and the graph the rest of the apply relies on.
pub(super) fn contract_edges(
    config: &mut types::ForjarConfig,
    keep: &[String],
    removed: &HashSet<String>,
) -> Vec<(String, String)> {
    if removed.is_empty() {
        return Vec::new();
    }
    let dep_map: HashMap<String, Vec<String>> = config
        .resources
        .iter()
        .map(|(id, r)| (id.clone(), r.depends_on.clone()))
        .collect();
    let mut cut: Vec<(String, String)> = Vec::new();
    for id in keep {
        let Some(old) = dep_map.get(id) else { continue };
        if !old.iter().any(|d| removed.contains(d)) {
            continue;
        }
        for d in old.iter().filter(|d| removed.contains(*d)) {
            cut.push((id.clone(), d.clone()));
        }
        let contracted = contract_one(old, &dep_map, removed);
        if let Some(r) = config.resources.get_mut(id) {
            r.depends_on = contracted;
        }
    }
    cut
}

/// One dependent's rewritten `depends_on`, order-preserving and deduplicated.
fn contract_one(
    old: &[String],
    dep_map: &HashMap<String, Vec<String>>,
    removed: &HashSet<String>,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for d in old {
        expand_dep(d, dep_map, removed, &mut out, &mut seen);
    }
    out
}

/// Walk `dep` down through removed nodes until only kept ids remain.
///
/// Terminates because `build_execution_order` already proved the graph acyclic;
/// `seen` makes it total even if that ever stops holding.
fn expand_dep(
    dep: &str,
    dep_map: &HashMap<String, Vec<String>>,
    removed: &HashSet<String>,
    out: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    if !removed.contains(dep) {
        if !out.iter().any(|d| d == dep) {
            out.push(dep.to_string());
        }
        return;
    }
    if !seen.insert(dep.to_string()) {
        return;
    }
    let Some(deps) = dep_map.get(dep) else { return };
    for d in deps {
        expand_dep(d, dep_map, removed, out, seen);
    }
}
