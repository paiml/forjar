//! Where a drift run gets its baselines — and what it does when there are none.
//!
//! Split out of `drift.rs` (forjar#385) so the one place that decides whether a
//! state directory is ABSENT or UNREADABLE is a file you can read in full.
//!
//! That distinction is the whole point. `read_dir` failing was treated as one
//! fault and killed the run, so paiml/infra's nightly drift lane — which
//! gitignores `state/`, and therefore checks out without one on every CI run —
//! had never measured anything:
//!
//! ```text
//! FAIL gx10  forjar drift exited 1: error: cannot read state dir .../infra/state
//! ```
//!
//! `NotFound` means "never applied from here", which is routine and survivable:
//! `type: task` resources carry an ASSERTION rather than a baseline, so the run
//! can still ask the host about them (`tripwire::drift::lockless`). Every other
//! `read_dir` error means the directory is there and forjar cannot read it —
//! wrong mode, not a directory, a dead mount — which is a broken host and stays
//! fatal. Collapsing the second into the first would turn a real fault into a
//! quiet partial answer, which is the defect class this fix exists to close.

use crate::core::{state, types};
use std::path::Path;

/// Machine directory names under `state_dir`, in read order, honouring an
/// optional single-machine filter. Unreadable entries and non-directories are
/// skipped; whether an empty result is an error is left to the caller.
///
/// `Ok(None)` is the ABSENT state dir — no lock exists anywhere, so there is
/// nothing here to enumerate and the caller takes the lockless path.
pub(super) fn machine_state_dirs(
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<Option<Vec<String>>, String> {
    let entries = match std::fs::read_dir(state_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "cannot read state dir {}: {}",
                state_dir.display(),
                e
            ))
        }
    };
    let mut names = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if machine_filter.is_some_and(|filter| name != filter) {
            continue;
        }
        if !entry.path().is_dir() {
            continue;
        }
        names.push(name);
    }
    Ok(Some(names))
}

/// `(machine_name, lock)` pairs from the state directory, or `None` when the
/// state directory is absent.
pub(super) fn collect_machine_locks(
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<Option<Vec<(String, types::StateLock)>>, String> {
    let Some(names) = machine_state_dirs(state_dir, machine_filter)? else {
        return Ok(None);
    };
    let mut locks = Vec::new();
    for name in names {
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            locks.push((name, lock));
        }
    }
    // A FILTER THAT MATCHES NOTHING IS AN ERROR, NOT A CLEAN BILL OF HEALTH.
    //
    // `-m <machine>` narrowed the scan by name; if nothing matched, this
    // returned an empty list and the caller reported "No drift detected." over
    // ZERO machines — with `--tripwire` still exiting 0. So a typo in a cron'd
    // `forjar drift --tripwire -m intel` silently stopped checking anything and
    // reported healthy forever. Ledger id
    // drift-tripwire-false-green-on-unknown-machine, confirmed at 1.12.3 and
    // still live at 1.16.0.
    if let Some(filter) = machine_filter {
        // Distinguish "this machine does not exist" from "this machine exists
        // but has no state yet". Only the FIRST is an error: a machine dir with
        // no lock is a machine that has simply never been applied, and failing
        // there would break `drift -m <new-machine>` before its first apply.
        // Keying on lock-presence instead conflated the two and broke
        // test_fj017_drift_machine_filter, which sets up exactly that case.
        let dir_exists = state_dir.join(filter).is_dir();
        if !dir_exists {
            return Err(unknown_machine(state_dir, filter));
        }
    }

    Ok(Some(locks))
}

/// The refusal for a `-m` that names no machine directory.
fn unknown_machine(state_dir: &Path, filter: &str) -> String {
    let known: Vec<String> = std::fs::read_dir(state_dir)
        .map(|es| {
            es.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    format!(
        "unknown machine '{filter}' — it has no directory in {}, so NOTHING was checked. Known: {}",
        state_dir.display(),
        if known.is_empty() {
            "(none)".to_string()
        } else {
            known.join(", ")
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The forjar#385 case: absent is not an error, it is `None`.
    #[test]
    fn an_absent_state_dir_is_not_an_error() {
        let d = tempfile::tempdir().unwrap();
        let missing = d.path().join("no-such-state");
        assert_eq!(machine_state_dirs(&missing, None), Ok(None));
        assert!(matches!(collect_machine_locks(&missing, None), Ok(None)));
    }

    /// THE LINE. A state path that exists and is not a readable directory is a
    /// broken host, and must NOT be reported as "never applied from here".
    #[test]
    fn a_state_path_that_is_a_file_is_still_fatal() {
        let d = tempfile::tempdir().unwrap();
        let file = d.path().join("state");
        std::fs::write(&file, "not a directory").unwrap();
        let err = machine_state_dirs(&file, None).unwrap_err();
        assert!(err.contains("cannot read state dir"), "{err}");
    }

    /// An empty-but-present state dir still enumerates to zero machines, which
    /// is a different answer from "there is no state dir".
    #[test]
    fn a_present_empty_state_dir_enumerates_nothing() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(machine_state_dirs(d.path(), None), Ok(Some(Vec::new())));
    }
}
