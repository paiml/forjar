//! forjar#404: the pre-apply drift gate's fan-out across machines.
//!
//! Split out of `apply_drift.rs` so that file stays inside the 500-line
//! health limit and the concurrency policy — how many machines at once, and
//! what happens when one of them fails — lives in one place.

use super::apply_drift::{record_machine_drift, DriftRepair, GateRun};
use crate::core::types;

/// Run the gate over every locked machine, one thread per machine in waves.
pub(super) fn gate(
    run: GateRun<'_>,
    locks: &[(String, types::StateLock)],
) -> Result<Vec<DriftRepair>, String> {
    if locks.len() <= 1 {
        gate_sequential(run, locks)
    } else {
        gate_parallel(run, locks)
    }
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
    let mut observed = Vec::new();
    let mut failures = Vec::new();
    // BOUNDED. `std::thread::scope` over the whole slice spawned one thread and
    // one ssh per locked machine at once — fine at 5, not at 5,000. Waves of
    // GATE_FANOUT keep the concurrency at the size of a connection pool, the
    // way `forks` does for Ansible.
    for wave in locks.chunks(GATE_FANOUT) {
        let results: Vec<Result<Vec<DriftRepair>, String>> = std::thread::scope(|s| {
            let handles: Vec<_> = wave
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
        // EVERY failure, not the first. `r?` on the first error dropped the
        // findings AND the errors of every machine after it in the wave, so an
        // operator saw one unreachable host and nothing about the other nine.
        for (r, (name, _)) in results.into_iter().zip(wave) {
            match r {
                Ok(found) => observed.extend(found),
                Err(e) => failures.push(format!("[{name}] {e}")),
            }
        }
    }
    if failures.is_empty() {
        Ok(observed)
    } else {
        Err(format!(
            "drift gate failed on {} machine(s): {}",
            failures.len(),
            failures.join("; ")
        ))
    }
}

/// Machines checked at once by the pre-apply gate.
const GATE_FANOUT: usize = 32;
