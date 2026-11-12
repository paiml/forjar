//! FJ-LOCK-001 C4/C5 (#924, #925): which locks an apply takes, and how long it
//! waits for them.
//!
//! The state `forjar apply` protects is already per machine
//! (`<sd>/<machine>/state.lock.yaml`, `events.jsonl`, `last-apply.yaml`), so
//! the lock is too: `<sd>/<machine>/.forjar.lock`. An apply takes the locks of
//! the machines it will write, in sorted order, so two multi-machine applies
//! cannot deadlock and a hung apply on one machine never blocks another.
//!
//! Files genuinely shared across machines (`forjar.lock.yaml`, generations,
//! outputs) are written under a short [`hold_global_lock`] instead.

use super::lock_info::read_lock_info;
use super::process_lock::{force_unlock_path, hold_paths, ProcessLockGuard};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// `--lock-timeout` in milliseconds; 0 (the default) fails fast.
static LOCK_TIMEOUT_MS: AtomicU64 = AtomicU64::new(0);

/// How long a global-lock critical section may be waited on. Those sections
/// are file writes measured in milliseconds, so a minute is a hung holder.
const GLOBAL_LOCK_WAIT: Duration = Duration::from_secs(60);

/// Set the wait applied to every machine lock taken from now on.
pub fn set_lock_timeout(timeout: Duration) {
    let ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
    LOCK_TIMEOUT_MS.store(ms, Ordering::Relaxed);
}

/// The wait set by [`set_lock_timeout`].
pub fn lock_timeout() -> Duration {
    Duration::from_millis(LOCK_TIMEOUT_MS.load(Ordering::Relaxed))
}

/// Parse a `--lock-timeout` value: `0`, `5s`, `250ms`, `2m`, `1h`; a bare
/// number is seconds.
pub fn parse_lock_timeout(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    let split = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    let n: u64 = num
        .parse()
        .map_err(|_| format!("invalid --lock-timeout '{s}' (examples: 0, 5s, 250ms, 2m)"))?;
    let ms = match unit {
        "" | "s" => n.checked_mul(1000),
        "ms" => Some(n),
        "m" => n.checked_mul(60_000),
        "h" => n.checked_mul(3_600_000),
        _ => None,
    };
    ms.map(Duration::from_millis)
        .ok_or_else(|| format!("invalid --lock-timeout '{s}' (examples: 0, 5s, 250ms, 2m)"))
}

/// `<sd>/<machine>/.forjar.lock`. A machine name is a directory name; one that
/// could leave the state dir is refused.
pub fn machine_lock_path(state_dir: &Path, machine: &str) -> Result<PathBuf, String> {
    if machine.is_empty() || machine == "." || machine == ".." || machine.contains(['/', '\\']) {
        return Err(format!(
            "machine name '{machine}' is not a valid lock scope"
        ));
    }
    Ok(state_dir.join(machine).join(".forjar.lock"))
}

/// `<sd>/.forjar.global.lock`, the short lock over files shared by machines.
pub fn global_guard_path(state_dir: &Path) -> PathBuf {
    state_dir.join(".forjar.global.lock")
}

/// Take the lock of every machine in `machines`, sorted and de-duplicated,
/// waiting up to the configured `--lock-timeout` for each holder. With
/// `force_unlock`, a dead holder is cleared first and a live one is refused.
pub fn hold_machine_locks(
    state_dir: &Path,
    machines: &[String],
    force_unlock: bool,
) -> Result<ProcessLockGuard, String> {
    let mut names: Vec<&String> = machines.iter().collect();
    names.sort();
    names.dedup();
    let mut paths = Vec::with_capacity(names.len());
    for m in names {
        let path = machine_lock_path(state_dir, m)?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        }
        paths.push(path);
    }
    hold_paths(&paths, force_unlock, lock_timeout())
}

/// Take the short global lock for a write to a file shared by machines.
/// Re-entrant on the same thread, so nested writers join the hold.
pub fn hold_global_lock(state_dir: &Path) -> Result<ProcessLockGuard, String> {
    std::fs::create_dir_all(state_dir).map_err(|e| format!("cannot create state dir: {e}"))?;
    hold_paths(&[global_guard_path(state_dir)], false, GLOBAL_LOCK_WAIT)
}

/// Every lock file under a state dir: the root lock, the global lock and one
/// per machine directory.
fn all_lock_paths(state_dir: &Path) -> Vec<PathBuf> {
    let mut paths = vec![state_dir.join(".forjar.lock"), global_guard_path(state_dir)];
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for e in entries.flatten() {
            if e.path().is_dir() {
                paths.push(e.path().join(".forjar.lock"));
            }
        }
    }
    paths.sort();
    paths
}

/// `forjar apply --force-unlock <ID>`: remove the lock carrying `id`, unless
/// its holder is alive on this host. Names no existing lock: an error, so a
/// typo is never mistaken for a cleared lock.
pub fn force_unlock_by_id(state_dir: &Path, id: &str) -> Result<PathBuf, String> {
    let found = all_lock_paths(state_dir)
        .into_iter()
        .find(|p| read_lock_info(p).is_some_and(|i| i.id == id));
    let Some(path) = found else {
        return Err(format!(
            "--force-unlock {id}: no lock under {} carries that id",
            state_dir.display()
        ));
    };
    force_unlock_path(&path, Some(id))?;
    Ok(path)
}
