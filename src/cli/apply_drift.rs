//! Pre-apply drift observation.
//!
//! Extracted from `apply.rs` (forjar#334): that file sat 137 lines over the
//! repo's 500-line ceiling, so the ratchet forbade it growing by even the one
//! gate this issue needed. The behaviour here is unchanged by the move.

use super::helpers_state::*;
use crate::core::types;
use std::borrow::Cow;
use std::path::Path;

/// Which resources this apply is actually going to act on.
///
/// forjar#404 (CRUX audit E02): the gate took only `machine_filter`, so
/// `apply -r one-resource` still probed EVERY locked resource on the machine
/// — one full SSH handshake each — and then wrote `status: drifted` over
/// resources the very same run was about to skip. The lock came back claiming
/// drift nothing had repaired, and the next `forjar drift` read it as fact.
#[derive(Clone, Copy, Default)]
pub(super) struct GateScope<'a> {
    /// `-m` / `--machine`.
    pub machine: Option<&'a str>,
    /// `-r` / `--resource`: an exact resource id.
    pub resource: Option<&'a str>,
    /// `-t` / `--tag`.
    pub tag: Option<&'a str>,
}

impl GateScope<'_> {
    /// Does the run act on this resource id?
    ///
    /// Mirrors the executor's own predicates: `resource_filter` is an exact id
    /// match (`resource_ops::should_skip_single`) and `tag_filter` requires the
    /// declaration to carry the tag (`resource_ops::resource_filtered_out`). A
    /// locked id with no declaration left cannot carry a tag, so a tag filter
    /// excludes it — exactly as the executor would.
    fn covers(&self, id: &str, resources: &indexmap::IndexMap<String, types::Resource>) -> bool {
        if self.resource.is_some_and(|r| r != id) {
            return false;
        }
        match self.tag {
            None => true,
            Some(tag) => resources
                .get(id)
                .is_some_and(|r| r.tags.iter().any(|t| t == tag)),
        }
    }

    /// The lock as the gate should read it: only the entries in scope.
    ///
    /// Borrows when nothing is filtered, so the common unfiltered apply pays
    /// no clone.
    fn narrow<'l>(
        &self,
        lock: &'l types::StateLock,
        resources: &indexmap::IndexMap<String, types::Resource>,
    ) -> Cow<'l, types::StateLock> {
        if self.resource.is_none() && self.tag.is_none() {
            return Cow::Borrowed(lock);
        }
        let mut scoped = lock.clone();
        scoped.resources.retain(|id, _| self.covers(id, resources));
        Cow::Owned(scoped)
    }
}

/// Everything the per-machine half of the gate needs, so the fan-out closure
/// captures one `Copy` value instead of six.
#[derive(Clone, Copy)]
struct GateRun<'a> {
    config: &'a types::ForjarConfig,
    state_dir: &'a Path,
    scope: GateScope<'a>,
    dry_run: bool,
    verbose: bool,
}

/// One drift finding observed BEFORE the apply ran.
///
/// forjar#336: the gate used to consume each finding for two side effects — an
/// stderr line and a `ResourceStatus::Drifted` write — and return unit, so by
/// the time the summary was printed the only surviving facts about the run were
/// three integers that cannot express WHY a resource converged. A deploy and an
/// intrusion produced byte-identical summaries.
#[derive(Debug, Clone)]
pub(super) struct DriftRepair {
    /// Machine the drift was observed on.
    pub machine: String,
    /// Resource that had drifted.
    pub resource_id: String,
    /// What the detector saw.
    pub detail: String,
}

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
    scope: GateScope<'_>,
    force: bool,
    dry_run: bool,
    verbose: bool,
) -> Result<Vec<DriftRepair>, String> {
    // `--force` bypasses the gate entirely, so drift repairs are unobservable
    // under it BY CONSTRUCTION — the empty vec is the honest answer, not a
    // missing measurement. Running the detector here anyway would add a full
    // transport round-trip per resource to the one path that exists to skip
    // observation.
    if !config.policy.tripwire || force {
        return Ok(Vec::new());
    }
    // `load_machine_locks` returns a HashMap, and a HashMap's iteration order
    // is not an order: the `drift:` lines and the returned findings came out in
    // a different sequence on every run. Sorting costs nothing here and makes
    // the fan-out below deterministic.
    let mut locks: Vec<(String, types::StateLock)> =
        load_machine_locks(config, state_dir, scope.machine)?
            .into_iter()
            .collect();
    locks.sort_by(|a, b| a.0.cmp(&b.0));

    let run = GateRun {
        config,
        state_dir,
        scope,
        dry_run,
        verbose,
    };
    let observed = if locks.len() <= 1 {
        gate_sequential(run, &locks)?
    } else {
        gate_parallel(run, &locks)?
    };

    // PRINTED HERE, NOT IN THE WORKER. Under the fan-out, per-finding
    // `eprintln!`s from several machines interleave mid-line; emitting them
    // from the join point keeps the exact same text in a stable machine order.
    for d in &observed {
        eprintln!("  drift: [{}] {} — {}", d.machine, d.resource_id, d.detail);
    }
    if !observed.is_empty() && verbose {
        eprintln!(
            "{} resource(s) drifted — they will be reconciled by this apply",
            observed.len()
        );
    }
    Ok(observed)
}

/// One machine (or none): no thread is worth starting.
fn gate_sequential(
    run: GateRun<'_>,
    locks: &[(String, types::StateLock)],
) -> Result<Vec<DriftRepair>, String> {
    let mut observed = Vec::new();
    for (name, lock) in locks {
        observed.extend(record_machine_drift(run, name, lock)?);
    }
    Ok(observed)
}

/// forjar#404: check every machine at once.
///
/// `forjar drift` has fanned this identical work out with `std::thread::scope`
/// since FJ-1396 (`cli/drift.rs`); the apply gate ran the same detector in a
/// bare `for` loop, so machine 2 waited on machine 1's handshakes. Modelled at
/// 100 machines × 50 resources that is 25–50 minutes of pure SSH setup before
/// any convergence work starts.
///
/// Results are collected in the handles' order, so the caller's output does not
/// depend on which machine answered first.
fn gate_parallel(
    run: GateRun<'_>,
    locks: &[(String, types::StateLock)],
) -> Result<Vec<DriftRepair>, String> {
    let results: Vec<Result<Vec<DriftRepair>, String>> = std::thread::scope(|s| {
        let handles: Vec<_> = locks
            .iter()
            .map(|(name, lock)| s.spawn(move || record_machine_drift(run, name, lock)))
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err("drift gate worker panicked".to_string()))
            })
            .collect()
    });
    let mut observed = Vec::new();
    for r in results {
        observed.extend(r?);
    }
    Ok(observed)
}

/// The per-machine half: detect, print, record, and hand the findings back.
///
/// Extracted so `check_pre_apply_drift` stays inside the repo's complexity cap
/// once it accumulates rather than discards.
fn record_machine_drift(
    run: GateRun<'_>,
    machine_name: &str,
    lock: &types::StateLock,
) -> Result<Vec<DriftRepair>, String> {
    let GateRun {
        config,
        state_dir,
        scope,
        dry_run,
        verbose,
    } = run;
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
    // forjar#404: DETECT THROUGH THE SCOPE, WRITE THROUGH THE WHOLE LOCK.
    //
    // Every detector in `tripwire::drift` walks `lock.resources`, so narrowing
    // the lock is what bounds the remote queries — that is the whole cost of
    // the gate. It must NOT be the lock that gets persisted below: saving the
    // narrowed clone would delete every out-of-scope resource from
    // `state.lock.yaml`, turning a `-r` apply into state amputation.
    let narrowed = scope.narrow(lock, &config.resources);
    let findings = match config.machines.get(machine_name) {
        Some(m) => crate::tripwire::drift::detect_drift_full(&narrowed, m, &resolved),
        None => crate::tripwire::drift::detect_drift(&narrowed),
    };
    if findings.is_empty() {
        return Ok(Vec::new());
    }
    // RECORD IT, so the planner acts on it. `Drifted` is a status the
    // planner already honours and nothing ever set. Persisting it also
    // makes the lock honest between runs: forjar observed drift, and
    // the lock now says so until an apply reconciles it.
    let mut updated = lock.clone();
    let mut observed = Vec::with_capacity(findings.len());
    for f in &findings {
        if let Some(rl) = updated.resources.get_mut(&f.resource_id) {
            rl.status = types::ResourceStatus::Drifted;
        }
        observed.push(DriftRepair {
            machine: machine_name.to_string(),
            resource_id: f.resource_id.clone(),
            detail: f.detail.clone(),
        });
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
    Ok(observed)
}

/// The RESOURCES this run actually repaired.
///
/// forjar#336. Two filters, and both are load-bearing.
///
/// INTERSECT WITH WHAT CONVERGED. Reporting raw findings would over-claim: the
/// gate leaves a resource excluded by `-r` / `--only-machine` / a tag filter, or
/// one that failed, as `drifted` / `failed` in the post-apply lock, and
/// `build_resource_reports` derives `status` from that lock — so "the lock says
/// converged afterwards" is the sound oracle for "the drift was repaired".
/// Claiming a repair the run never performed is worse than saying nothing: the
/// operator then does not go and fix it.
///
/// COUNT RESOURCES, NOT FINDINGS. `detect_drift_full` emits one finding per
/// OBSERVABLE, so a single tampered file yields both `content changed` and
/// `file state changed`. Counting findings made the summary say "2 repaired
/// drift" for one file and tripped the `drift_repaired <= total_converged`
/// assertion — the invariant caught it, which is what it is for.
pub(super) fn repaired(
    observed: &[DriftRepair],
    results: &[types::ApplyResult],
) -> Vec<DriftRepair> {
    let mut seen = std::collections::HashSet::new();
    observed
        .iter()
        .filter(|d| {
            results.iter().any(|r| {
                r.machine == d.machine
                    && r.resource_reports
                        .iter()
                        .any(|rr| rr.resource_id == d.resource_id && rr.status == "converged")
            })
        })
        .filter(|d| seen.insert((d.machine.clone(), d.resource_id.clone())))
        .cloned()
        .collect()
}
