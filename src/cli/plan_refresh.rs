//! forjar#415: `plan --refresh` — consult the host before diffing.
//!
//! A plain `plan` compares the config to the LOCK and says so: it is
//! lock-relative and contacts no machine (forjar#342). `--refresh` runs the
//! same detectors `forjar drift` runs, against every machine in scope, and
//! marks each resource it MEASURED as changed `Drifted` in an in-memory copy
//! of the locks. The planner already plans a `Drifted` entry as `Update`, so a
//! file edited behind forjar's back shows up as a change without an
//! intervening `apply` or `drift`.
//!
//! Nothing is written. The locks on disk are untouched; `drift` and
//! `apply --refresh` stay the commands that change state.
//!
//! The planner is unchanged and still a function of (config, locks) alone —
//! what changes is which locks the CLI hands it.
//!
//! A query the target did not answer is UNMEASURED (forjar#549), not drift: it
//! is counted, re-planned as nothing, and disclosed, because re-planning on "I
//! could not look" would rebuild the world whenever a host is unreachable.

use crate::core::types;
use crate::tripwire::drift;
use std::collections::HashMap;

/// What one `plan --refresh` measured.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RefreshOutcome {
    /// Machines whose drift detectors ran.
    pub machines: usize,
    /// Lock entries a detector measured as changed (each counted once, and
    /// counted even if the lock already said `Drifted`).
    pub drifted: usize,
    /// Lock entries this plan still planned from the lock alone: queries the
    /// target never answered, observed entries in scope that no detector
    /// inspected (a skip is not a pass), and observed entries on a machine the
    /// config does not declare (there is no transport to ask it with).
    pub unconsulted: usize,
}

/// Run the drift detectors for every machine in `locks` and mark what drifted.
///
/// `locks` is already narrowed to the `-m` filter by `load_machine_locks`.
/// `config.resources` is the narrowed set the plan ranges over, so a `--target`
/// or phony strip narrows what is queried too.
pub(crate) fn refresh_locks_for_plan(
    config: &types::ForjarConfig,
    locks: &mut HashMap<String, types::StateLock>,
) -> RefreshOutcome {
    let mut outcome = RefreshOutcome::default();
    // PMAT-197: resolve templates before comparing against the host, exactly
    // as `drift` does, or every `{{params.*}}` resource reads as drifted.
    let resolved = crate::core::resolver::resolve_all(
        &config.resources,
        &config.params,
        &config.machines,
        &config.secrets,
    );
    for (name, lock) in locks.iter_mut() {
        let Some(machine) = config.machines.get(name) else {
            outcome.unconsulted += observed_entries(lock);
            continue;
        };
        let report = drift::detect_drift_full_reported(
            lock,
            machine,
            &resolved,
            drift::DriftOptions::default(),
        );
        outcome.machines += 1;
        // A resource can carry more than one finding; count it once. Count it
        // whatever its prior status: `drifted` is what the host showed, not
        // how many statuses this call flipped.
        let mut measured = std::collections::HashSet::new();
        for finding in &report.findings {
            if finding.is_unmeasured() {
                continue;
            }
            if let Some(entry) = lock.resources.get_mut(&finding.resource_id) {
                entry.status = types::ResourceStatus::Drifted;
                measured.insert(finding.resource_id.clone());
            }
        }
        outcome.drifted += measured.len();
        // Unanswered queries AND observed entries no detector inspected: both
        // are planned from the lock alone, so both are disclosed.
        outcome.unconsulted += report
            .census
            .unconsulted_observations(lock, &config.resources);
    }
    outcome
}

fn observed_entries(lock: &types::StateLock) -> usize {
    lock.resources
        .values()
        .filter(|rl| rl.observed_state().is_some())
        .count()
}

/// The disclosure for a refreshed plan: present iff something went unmeasured.
///
/// The lock-relative sentence would be false here — this plan did contact the
/// machines — so it is replaced, not appended to.
pub(crate) fn refreshed_scope_disclosure(outcome: &RefreshOutcome) -> Option<String> {
    if outcome.unconsulted == 0 {
        return None;
    }
    Some(format!(
        "This plan ran --refresh against {} machine(s), but {} locked resource(s)\n\
         were not measured (the target did not answer, a detector skipped them, or\n\
         their machine is not declared) and are planned from the lock alone — run\n\
         `forjar drift` for what those machines actually hold.",
        outcome.machines, outcome.unconsulted
    ))
}
