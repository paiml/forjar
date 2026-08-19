//! FJ-2711 (PMAT-197): change propagation across `depends_on`.
//!
//! Extracted from `planner::mod` to keep that file under the 500-line health
//! limit, and because this is a self-contained graph rule worth testing alone.

use crate::core::types::*;
use std::collections::HashSet;

/// Promote every `NoOp` dependent of a changed resource to `Update`.
///
/// `make` relinks `build/demo` when `build/util.o` is rebuilt. forjar's
/// `depends_on` is ORDERING ONLY, so without this a rebuilt prerequisite left
/// its dependents converged and the final artifact stale — exactly what the
/// first working staleness probe still got wrong: `obj-util` rebuilt, `link`
/// did not, and `./build/demo` kept printing the old answer.
///
/// The probe cannot see this itself: it runs BEFORE the plan, so a dependent's
/// inputs are observed at their pre-rebuild values.
///
/// `execution_order` is already topologically sorted, so ONE forward sweep
/// reaches a fixpoint — no iteration needed.
///
/// Early cutoff (make's missing feature) is deliberately NOT applied here: at
/// plan time the upstream has not re-run, so its post-build output hash is
/// unknowable. Suppressing a rebuild on a guess is how a build system ships a
/// wrong binary; the cost of being right is one extra link.
///
/// Returns `(promoted_count)` so the caller can fix up its action counters.
pub fn propagate_changes(
    config: &ForjarConfig,
    execution_order: &[String],
    changes: &mut [PlannedChange],
) -> u32 {
    let mut promoted = 0u32;
    let mut dirty: HashSet<String> = changes
        .iter()
        .filter(|c| c.action != PlanAction::NoOp)
        .map(|c| c.resource_id.clone())
        .collect();

    for resource_id in execution_order {
        if dirty.contains(resource_id) {
            continue;
        }
        let Some(resource) = config.resources.get(resource_id) else {
            continue;
        };
        // NOTE: there is no `order_only` field. A previous comment here claimed
        // order-only edges were excluded from propagation; nothing implements
        // that. `import-makefile` maps make's `|` prerequisites onto
        // `depends_on`, so they DO propagate. The cost is a possible extra
        // rebuild (never a wrong result), and it is rare in practice because
        // directory artifacts are identified by existence since v1.11.1, so the
        // mkdir-shaped resource almost never goes dirty. Tracked for v1.13.
        let Some(trigger) = resource
            .depends_on
            .iter()
            .find(|dep: &&String| dirty.contains(dep.as_str()))
        else {
            continue;
        };
        let trigger = trigger.clone();

        for change in changes.iter_mut() {
            if change.resource_id == *resource_id && change.action == PlanAction::NoOp {
                change.action = PlanAction::Update;
                change.description =
                    format!("{resource_id}: rebuild — dependency {trigger} changed");
                promoted += 1;
                dirty.insert(resource_id.clone());
            }
        }
    }
    promoted
}
