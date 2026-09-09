//! forjar#497: the planner's census of what it did not measure.
//!
//! `determine_present_action` lets an observed probe override a matching
//! config hash, and falls through to the hash comparison when there is no
//! probe. So a MISSING probe and a probe that found nothing stale produce the
//! same `NoOp`, and the probe is only taken for machines this host answers
//! for — every SSH target, and since forjar#495 every pepita namespace, has
//! planned `NoOp` over changed sources for as long as the probe has existed.
//!
//! The action is left alone on purpose. Planning `Update` for every resource
//! this host cannot probe would rebuild every remote task on every apply and
//! break f(f(x)) = f(x) at the plan level. What changes is that the plan says
//! which resources it could not measure, in the shape `drift` already uses
//! for a resource it could not evaluate: named, with a reason, never counted
//! as clean by omission. This is the repository's own doctrine — an
//! UNMEASURED check must never print the same thing as a passed one.
//!
//! The census is a pure post-pass over the finished change set. It reads the
//! config, the changes and the probe map the planner was handed, and nothing
//! else: no filesystem, no transport.

use std::collections::HashMap;

use crate::core::task::{declares_inputs, probe_covers, IoDigest};
use crate::core::types::{ForjarConfig, PlanAction, PlannedChange, Resource, UnprobedResource};

/// Name every converged resource whose declared build I/O this plan did not
/// measure, per (resource, machine).
///
/// `NoOp` only. A resource that plans `Create` or `Update` is going to run,
/// so a missing probe hides nothing behind it; listing it would be the
/// unconditional banner the forjar#342 contract forbids, and noise is how a
/// disclosure stops being read. A resource that declares no `task_inputs`,
/// `ambient_inputs` or `output_artifacts` had nothing to probe and is never
/// named.
///
/// Called AFTER `propagation`, so a `NoOp` that a rebuilt prerequisite just
/// promoted to `Update` is no longer counted as silence.
pub(super) fn census(
    config: &ForjarConfig,
    changes: &[PlannedChange],
    probes: &HashMap<String, IoDigest>,
) -> Vec<UnprobedResource> {
    changes
        .iter()
        .filter(|c| c.action == PlanAction::NoOp)
        .filter_map(|c| {
            let resource = config.resources.get(&c.resource_id)?;
            if !declares_build_io(resource) {
                return None;
            }
            let reason = unprobed_reason(config, &c.resource_id, &c.machine, probes)?;
            Some(UnprobedResource {
                resource_id: c.resource_id.clone(),
                machine: c.machine.clone(),
                reason,
            })
        })
        .collect()
}

/// The resource declares something the probe would have measured.
fn declares_build_io(resource: &Resource) -> bool {
    declares_inputs(resource) || !resource.output_artifacts.is_empty()
}

/// Why no probe stands behind this (resource, machine), or `None` when one
/// does.
///
/// The machine is asked FIRST, through the same predicate the probe itself
/// uses (`probe_covers`, one definition). The probe map is keyed by resource
/// id alone, so a resource declared on both a local and a remote machine
/// carries the local probe's answer under its id — that answer says nothing
/// about the remote machine's tree, and the remote row is named here even
/// though the map has an entry. The second arm covers a probe the caller
/// could have taken and did not: an older caller passing an empty map, or a
/// digest that came back empty and was dropped.
fn unprobed_reason(
    config: &ForjarConfig,
    resource_id: &str,
    machine: &str,
    probes: &HashMap<String, IoDigest>,
) -> Option<String> {
    if !probe_covers(config, machine) {
        return Some(format!(
            "this host does not answer for machine {machine}, so the declared \
             inputs and artifacts of {resource_id} were not measured"
        ));
    }
    if !probes.contains_key(resource_id) {
        return Some(format!("no probe was taken for {resource_id} on {machine}"));
    }
    None
}
