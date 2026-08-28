//! The apply summary line and its two extra dimensions.
//!
//! Split out of `apply_output.rs` (forjar#336): adding the drift dimension put
//! that file over the repo's 500-line ceiling, which the pre-commit ratchet
//! enforces by refusing growth. The summary is the surface the whole
//! `apply-summary-distinguishability-v1` contract is about, so it is the piece
//! that earns its own module.

use super::helpers::*;
use crate::core::types;

/// Print apply summary (JSON or text).
///
/// FJ-129: `forced_noop_count` is the number of resources that --force
/// re-applied even though the lock reported them unchanged. When > 0,
/// surface a yellow note line — that's the runtime side of contract
/// `apply-summary-distinguishability-v1`, which makes claim C3
/// (idempotency) observable through --force. The contract assertion at
/// the bottom of this function rejects nonsense states (forced-noop
/// without --force, or forced-noop > converged).
/// GH-336: the clause naming how many convergences were drift REPAIRS.
///
/// Empty at zero, and that is load-bearing: the ordinary summary must stay
/// byte-identical, because other suites assert exact substrings of it
/// (`contains("0 converged")`, `contains("1 unchanged")`).
fn drift_clause(n: usize) -> String {
    if n == 0 {
        String::new()
    } else {
        format!(" ({n} repaired drift)")
    }
}

/// GH-336: name each repaired resource under the summary.
///
/// A convergence driven by observed reality is a different event from one
/// driven by a config change — the difference between a deploy and an
/// intrusion, or a deploy and a unit that keeps resetting itself. The finding
/// was already printed by the pre-apply gate, on stderr, before the run; a
/// scrollback or a CI log tail loses it by the time the summary lands.
fn print_drift_repairs(repairs: &[super::apply_drift::DriftRepair]) {
    for r in repairs {
        println!(
            "{}",
            yellow(&format!(
                "  drift-repaired: [{}] {} — {}",
                r.machine, r.resource_id, r.detail
            ))
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn print_apply_summary(
    config: &types::ForjarConfig,
    results: &[types::ApplyResult],
    total_converged: u32,
    total_unchanged: u32,
    total_failed: u32,
    forced_noop_count: u32,
    drift_repaired: &[super::apply_drift::DriftRepair],
    dur_apply: std::time::Duration,
    json: bool,
) -> Result<(), String> {
    // FJ-129 contract: forced_noop must not exceed converged. If --force
    // wasn't used, the caller passes 0; if --force WAS used, every forced
    // no-op is by definition counted in `total_converged` (the executor
    // ran the resource), so forced_noop ≤ converged is an invariant.
    debug_assert!(
        forced_noop_count <= total_converged,
        "C3-FORCE-DISTINGUISHABLE violated: forced_noop ({}) > converged ({})",
        forced_noop_count,
        total_converged
    );

    // GH-336: a repair is a convergence, so it is already inside
    // `total_converged`. The same shape as the FJ-129 assert above.
    debug_assert!(
        drift_repaired.len() as u32 <= total_converged,
        "drift_repaired ({}) > converged ({})",
        drift_repaired.len(),
        total_converged
    );

    let actual_changes = total_converged.saturating_sub(forced_noop_count);
    let drift_clause = drift_clause(drift_repaired.len());

    if json {
        let output = serde_json::json!({
            "name": config.name,
            "machines": results,
            "summary": {
                "total_converged": total_converged,
                "total_unchanged": total_unchanged,
                "total_failed": total_failed,
                // FJ-129: forced_noop_count is the C3-observable extension.
                // Always present so JSON consumers can branch on > 0;
                // 0 means either --force wasn't used or every forced
                // resource genuinely needed work.
                "forced_noop_count": forced_noop_count,
                "actual_changes": actual_changes,
                // GH-336: the `drift:` lines go to STDERR and the JSON to
                // stdout, so before this a `--json` consumer got ZERO drift
                // signal — a strictly worse form of the defect than the text
                // mode the issue describes. Always present, so a parser can
                // branch on > 0.
                "drift_repaired_count": drift_repaired.len(),
                "drift_repaired": drift_repaired
                    .iter()
                    .map(|d| serde_json::json!({
                        "machine": d.machine,
                        "resource": d.resource_id,
                        "detail": d.detail,
                    }))
                    .collect::<Vec<_>>(),
                "total_duration_seconds": dur_apply.as_secs_f64(),
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output)
                .map_err(|e| format!("JSON serialization error: {e}"))?
        );
    } else {
        for result in results {
            let failed_str = if result.resources_failed > 0 {
                red(&format!("{} failed", result.resources_failed))
            } else {
                format!("{} failed", result.resources_failed)
            };
            println!(
                "{}: {} converged, {} unchanged, {} ({:.1}s)",
                bold(&result.machine),
                green(&result.resources_converged.to_string()),
                result.resources_unchanged,
                failed_str,
                result.total_duration.as_secs_f64()
            );
        }
        println!();
        if total_failed > 0 {
            println!(
                "{}",
                red(&format!(
                    "Apply completed with errors: {total_converged} converged{drift_clause}, {total_unchanged} unchanged, {total_failed} FAILED"
                ))
            );
        } else {
            println!(
                "{}",
                green(&format!(
                    "Apply complete: {total_converged} converged{drift_clause}, {total_unchanged} unchanged."
                ))
            );
        }
        print_drift_repairs(drift_repaired);
        // FJ-129: When --force re-ran resources the lock reported as
        // unchanged, surface that explicitly so claim C3 is observable.
        // `actual_changes == 0` AND `forced_noop_count > 0` is the
        // C3-PASS shape; it's worth calling out unambiguously.
        if forced_noop_count > 0 {
            println!(
                "{}",
                // SAY WHAT WAS MEASURED. Both counts are LOCK-relative, per
                // contract apply-summary-distinguishability-v1
                // (forced_noop_count = shadow_plan(config, real_locks).unchanged).
                // The old wording — "{n} actual change(s)" — read as "nothing on
                // the machine changed", so `--force` restoring a file that had
                // been tampered with on disk announced "0 actual change(s)".
                // That is the LOCK's answer, and it is correct; the phrasing
                // claimed the MACHINE's answer, which this count never measured.
                //
                // The count itself is deliberately NOT changed: subtracting live
                // drift here was tried and reverted as a regression (GH-208,
                // FJ-129 shape 4 went 2 -> 1), and `forjar drift` is the command
                // that answers the live question. See the ledger note on
                // force-and-rollback-report-zero-actual-changes.
                yellow(&format!(
                    "note: --force re-ran {forced_noop_count} resource(s) the lock reported as unchanged \
                     ({actual_changes} differed from the lock, {forced_noop_count} matched it). \
                     This is lock-relative: run `forjar drift` for what the machines actually hold."
                ))
            );
        }
    }
    Ok(())
}
