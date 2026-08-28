//! Pre-apply drift observation.
//!
//! Extracted from `apply.rs` (forjar#334): that file sat 137 lines over the
//! repo's 500-line ceiling, so the ratchet forbade it growing by even the one
//! gate this issue needed. The behaviour here is unchanged by the move.

use super::helpers_state::*;
use crate::core::types;
use std::path::Path;

/// FJ-1378 / forjar#305: Pre-apply drift reconciliation.
///
/// WHAT THIS USED TO DO, AND WHY IT CHANGED. This BLOCKED the apply when live
/// state had drifted, telling the operator to re-run with `--force`. Two
/// problems with that:
///
///   1. It is not what an IaC apply is for. Terraform, Ansible and Kubernetes
///      all CONVERGE observed drift; refusing to act on the difference between
///      declared and actual is the one job the tool exists to do.
///   2. `--force` is not a repair, it is nuke-and-pave: it empties the lock map
///      so EVERY resource re-applies. There was no way to converge just the
///      resource that drifted.
///
/// So drift is now RECORDED rather than used to refuse. Each drifted resource's
/// lock entry is marked `ResourceStatus::Drifted`, and the planner already
/// turns any non-Converged status into `PlanAction::Update`
/// (planner/mod.rs: "Previously failed or drifted"). The machinery was all
/// there — the `Drifted` variant existed and nothing ever wrote it.
///
/// `forjar drift --tripwire` is unaffected and is still the right thing for a
/// CI gate: it answers "has anything drifted" without changing anything.
pub(super) fn check_pre_apply_drift(
    config: &types::ForjarConfig,
    state_dir: &Path,
    machine_filter: Option<&str>,
    force: bool,
    dry_run: bool,
    verbose: bool,
) -> Result<(), String> {
    if !config.policy.tripwire || force {
        return Ok(());
    }
    let locks = load_machine_locks(config, state_dir, machine_filter)?;
    let mut total_drift = 0usize;
    for (machine_name, lock) in &locks {
        // FJ-1378-fix: Pass the machine object so container transports use
        // docker exec instead of checking the host filesystem.
        // USE THE SAME DETECTOR `forjar drift` USES.
        //
        // This called `detect_drift_with_machine`, which routes to
        // `detect_drift_impl` — the bytes-only `content_hash` path. A `source:`
        // file never gets a `content_hash`, so the gate could not see drift on
        // it even after drift/mod.rs stopped excluding files, and apply kept
        // reporting "unchanged" while `forjar drift` reported DRIFTED. Two
        // shipped surfaces contradicting each other is worse than one that is
        // merely blind.
        //
        // `detect_drift_full` is what `forjar drift` calls (cli/drift.rs), and
        // it needs the resolved resources so template-bearing paths compare
        // against what was actually deployed. (forjar#305.)
        // PMAT-197, REGRESSED BY #307 AND FIXED AGAIN HERE (forjar#310).
        //
        // This passed `&config.resources` — RAW, unresolved. cli/drift.rs has
        // carried the fix and the reason since PMAT-197: "resources MUST be
        // template-resolved before they are compared against live machine
        // state. Passing raw cfg.resources made every {{params.*}}-bearing
        // resource report permanent false drift."
        //
        // The comment three lines above this one already said the code "needs
        // the resolved resources so template-bearing paths compare against what
        // was actually deployed". The comment was right and the code did not do
        // it — so every templated resource was falsely drifted on every apply,
        // rewritten every run, and templated `task` commands re-executed every
        // time. 156 fleet resources are template-bearing.
        //
        // Resolving here also makes apply and `forjar drift` ask the SAME
        // question, which was the entire point of switching to detect_drift_full.
        let resolved = crate::core::resolver::resolve_all(
            &config.resources,
            &config.params,
            &config.machines,
            &config.secrets,
        );
        let findings = match config.machines.get(machine_name.as_str()) {
            Some(m) => crate::tripwire::drift::detect_drift_full(lock, m, &resolved),
            None => crate::tripwire::drift::detect_drift(lock),
        };
        if !findings.is_empty() {
            total_drift += findings.len();
            // RECORD IT, so the planner acts on it. `Drifted` is a status the
            // planner already honours and nothing ever set. Persisting it also
            // makes the lock honest between runs: forjar observed drift, and
            // the lock now says so until an apply reconciles it.
            let mut updated = lock.clone();
            for f in &findings {
                eprintln!(
                    "  drift: [{}] {} — {}",
                    machine_name, f.resource_id, f.detail
                );
                if let Some(rl) = updated.resources.get_mut(&f.resource_id) {
                    rl.status = types::ResourceStatus::Drifted;
                }
            }
            // A DRY RUN MUST NOT WRITE THE LOCK. #307 persisted here
            // unconditionally, so `apply --dry-run` — documented as making no
            // changes — mutated state, and that write was itself what silenced
            // `forjar drift` afterwards (forjar#310). The findings are still
            // PRINTED above, which is the whole job of a dry run.
            if dry_run {
                if verbose {
                    eprintln!(
                        "  (dry run: {} drifted resource(s) on {machine_name} NOT recorded in the lock)",
                        findings.len()
                    );
                }
            } else {
                // A failure to persist must not be silent: the apply would then
                // proceed against a lock that still says Converged and would
                // report "unchanged" over the very drift just printed.
                crate::core::state::save_lock(state_dir, &updated).map_err(|e| {
                    format!("observed {} drifted resource(s) on {machine_name} but could not record it in the lock: {e}", findings.len())
                })?;
            }
        }
    }
    if total_drift > 0 && verbose {
        eprintln!("{total_drift} resource(s) drifted — they will be reconciled by this apply");
    }
    Ok(())
}
