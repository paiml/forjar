//! FJ-LOCK-001 C3/C5: the lock flags of `forjar apply`.

use super::commands::ApplyLockArgs;

/// FJ-LOCK-001 C3/C5: apply `--lock-timeout` and `--force-unlock <ID>` before
/// the apply takes its locks. A bare `--force-unlock` rides the apply's own
/// stale-lock clearing; one with an ID removes exactly that lock now, and is
/// refused when the lock's holder is alive on this host.
pub(super) fn prepare_state_lock(
    lock: &ApplyLockArgs,
    state_dir: &std::path::Path,
) -> Result<(), String> {
    use crate::core::state;
    if let Some(t) = lock.lock_timeout.as_deref() {
        state::set_lock_timeout(state::parse_lock_timeout(t)?);
    }
    match lock.force_unlock.as_deref() {
        Some(id) if !id.is_empty() => {
            let path = state::force_unlock_by_id(state_dir, id)?;
            eprintln!("force-unlocked {} (id {id})", path.display());
        }
        _ => {}
    }
    Ok(())
}

/// FJ-LOCK-001 C2: hold the state lock of every machine the apply targets.
/// A dry run writes no state and takes no lock.
pub(super) fn hold_apply_locks(
    config: &crate::core::types::ForjarConfig,
    state_dir: &std::path::Path,
    machine: Option<&str>,
    force_unlock: bool,
    dry_run: bool,
) -> Result<Option<crate::core::state::ProcessLockGuard>, String> {
    if dry_run {
        return Ok(None);
    }
    let targets = crate::core::executor::lock_targets(config, machine);
    crate::core::state::hold_machine_locks(state_dir, &targets, force_unlock).map(Some)
}
