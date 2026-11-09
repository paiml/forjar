//! FJ-LOCK-001 C3/C5: the lock flags of `forjar apply`.

use super::commands::ApplyArgs;

/// FJ-LOCK-001 C3/C5: apply `--lock-timeout` and `--force-unlock <ID>` before
/// the apply takes its locks. A bare `--force-unlock` rides the apply's own
/// stale-lock clearing; one with an ID removes exactly that lock now, and is
/// refused when the lock's holder is alive on this host.
pub(super) fn prepare_state_lock(
    args: &ApplyArgs,
    state_dir: &std::path::Path,
) -> Result<(), String> {
    use crate::core::state;
    if let Some(t) = args.lock_timeout.as_deref() {
        state::set_lock_timeout(state::parse_lock_timeout(t)?);
    }
    match args.force_unlock.as_deref() {
        Some(id) if !id.is_empty() => {
            let path = state::force_unlock_by_id(state_dir, id)?;
            eprintln!("force-unlocked {} (id {id})", path.display());
        }
        _ => {}
    }
    Ok(())
}
