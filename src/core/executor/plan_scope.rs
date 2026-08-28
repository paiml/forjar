//! Refs #358 — restrict an apply to the delta a reviewed plan asked for.
//!
//! `apply --plan-file` verified the plan's `config_hash` and then built an
//! `ApplyConfig` in which every selector was `None`, so it converged the WHOLE
//! current config. `plan.changes` and `plan.execution_order` were never read
//! after the counters were summed. The reviewed delta and the executed delta
//! were only incidentally the same, and they diverged exactly when it matters:
//! `config_hash` covers `forjar.yaml` but says nothing about the machines, so
//! any host that drifted between `plan` and `apply` was silently in scope.
//!
//! A scope is a SET of `(machine, resource)` pairs, which is why the existing
//! `resource_filter: Option<&str>` could not carry it — a plan routinely names
//! several resources across several machines.
//!
//! # Out-of-scope changes are demoted, not deleted
//!
//! A change the plan did not ask for becomes `NoOp`, which the executor already
//! skips. Deleting it instead would also silence the `triggers` mechanism: a
//! resource that a scoped resource notifies is part of applying THAT resource
//! (a config file updating and restarting its service), and it fires in an
//! unsealed apply too. Demotion keeps that behaviour identical while removing
//! the thing the defect was about — resources the plan called `NoOp` that have
//! drifted since and would otherwise be converged unreviewed.

use crate::core::types::{ExecutionPlan, PlanAction};
use std::collections::HashSet;

/// The `(machine, resource)` pairs a reviewed plan actually asked for.
#[derive(Debug, Clone, Default)]
pub struct PlanScope {
    pairs: HashSet<(String, String)>,
    machines: HashSet<String>,
}

impl PlanScope {
    /// Derive the scope from a plan's non-`NoOp` changes.
    pub fn from_plan(plan: &ExecutionPlan) -> Self {
        let mut scope = Self::default();
        for change in &plan.changes {
            if change.action == PlanAction::NoOp {
                continue;
            }
            scope.machines.insert(change.machine.clone());
            scope
                .pairs
                .insert((change.machine.clone(), change.resource_id.clone()));
        }
        scope
    }

    /// Number of reviewed `(machine, resource)` pairs.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// True when the plan asked for nothing.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Did the reviewed plan ask for this resource on this machine?
    pub fn covers(&self, machine: &str, resource_id: &str) -> bool {
        self.pairs
            .contains(&(machine.to_string(), resource_id.to_string()))
    }

    /// Did the reviewed plan ask for anything at all on this machine?
    ///
    /// A machine the plan does not name is not connected to: reaching a host to
    /// do nothing is still reaching a host.
    pub fn covers_machine(&self, machine: &str) -> bool {
        self.machines.contains(machine)
    }

    /// Sorted machine names, for messages.
    pub fn machine_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.machines.iter().cloned().collect();
        names.sort();
        names
    }
}

/// Recompute the action counters from whatever survived the restriction.
///
/// The counters must keep partitioning the change list — that invariant is what
/// `plan_seal::check_body_partition` relies on, and `planner` `debug_assert!`s.
fn recount(plan: &mut ExecutionPlan) {
    plan.to_create = 0;
    plan.to_update = 0;
    plan.to_destroy = 0;
    plan.unchanged = 0;
    for change in &plan.changes {
        match change.action {
            PlanAction::Create => plan.to_create += 1,
            PlanAction::Update => plan.to_update += 1,
            PlanAction::Destroy => plan.to_destroy += 1,
            PlanAction::NoOp => plan.unchanged += 1,
        }
    }
}

/// Demote every change the scope does not cover to `NoOp`.
///
/// `None` leaves the plan exactly as the planner produced it, which is what an
/// ordinary `forjar apply` wants.
pub(crate) fn restrict(mut plan: ExecutionPlan, scope: Option<&PlanScope>) -> ExecutionPlan {
    let Some(scope) = scope else {
        return plan;
    };
    for change in &mut plan.changes {
        if !scope.covers(&change.machine, &change.resource_id) {
            change.action = PlanAction::NoOp;
        }
    }
    recount(&mut plan);
    plan
}

#[cfg(test)]
#[path = "tests_plan_scope.rs"]
mod tests_plan_scope;
