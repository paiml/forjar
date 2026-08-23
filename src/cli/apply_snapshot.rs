//! Pre-apply snapshots and numbered generations (FJ-1381, FJ-1386).
//!
//! Extracted from `apply.rs`, which sits over the 500-line ratchet. Both
//! features hang off one config key, `policy.snapshot_generations`, which is
//! at once the on-switch and the retention count — and that overloading is
//! what made `forjar undo` inert on every default config, since undo has
//! nothing to target unless something here ran.

use crate::core::types;
use std::path::Path;

/// FJ-1381: Auto-snapshot before apply if `snapshot_generations` is set.
pub(super) fn maybe_auto_snapshot(
    config: &types::ForjarConfig,
    state_dir: &Path,
    config_path: Option<&Path>,
    dry_run: bool,
    verbose: bool,
) {
    let Some(gens) = config.policy.snapshot_generations else {
        return;
    };
    if gens == 0 || dry_run {
        return;
    }
    // A named snapshot copies the state dir, so it needs one to exist. A
    // generation does not, and used to share the guard: on the very first
    // apply the state dir has not been written yet, so nothing was recorded
    // and `undo` still refused after the operator had done what it asked.
    // The pre-apply state simply IS empty there, and recording that empty
    // generation 0 is exactly what makes the SECOND apply undoable.
    if state_dir.exists() {
        save_pre_apply_snapshot(state_dir, gens, verbose);
    }
    record_generation(state_dir, config_path, gens, verbose);
}

/// FJ-1381: save the pre-apply named snapshot, then GC older ones.
fn save_pre_apply_snapshot(state_dir: &Path, gens: u32, verbose: bool) {
    let snap_name = format!("pre-apply-{}", crate::tripwire::eventlog::now_iso8601());
    if let Err(e) = super::snapshot::cmd_snapshot_save(&snap_name, state_dir) {
        eprintln!("warning: pre-apply snapshot failed: {e}");
    } else if verbose {
        eprintln!("snapshot: saved {snap_name}");
    }
    gc_old_snapshots(state_dir, gens, verbose);
}

/// FJ-1386: record a numbered generation for instant rollback, then GC old ones.
fn record_generation(state_dir: &Path, config_path: Option<&Path>, gens: u32, verbose: bool) {
    match super::generation::create_generation(state_dir, config_path) {
        Ok(gen) => {
            if verbose {
                eprintln!("generation: created gen {gen}");
            }
            super::generation::gc_generations(state_dir, gens, verbose);
        }
        Err(e) => eprintln!("warning: generation creation failed: {e}"),
    }
}

/// FJ-1381: Garbage-collect old snapshots, keeping only the newest `keep`.
fn gc_old_snapshots(state_dir: &Path, keep: u32, verbose: bool) {
    let snap_dir = super::snapshot::snapshots_dir(state_dir);
    if !snap_dir.exists() {
        return;
    }
    let mut entries: Vec<_> = match std::fs::read_dir(&snap_dir) {
        Ok(e) => e.flatten().filter(|e| e.path().is_dir()).collect(),
        Err(_) => return,
    };
    let to_remove = super::apply_gates::snapshots_to_remove(entries.len(), keep);
    if to_remove == 0 {
        return;
    }
    entries.sort_by_key(|e| e.file_name());
    for entry in entries.iter().take(to_remove) {
        if verbose {
            eprintln!(
                "snapshot gc: removing {}",
                entry.file_name().to_string_lossy()
            );
        }
        let _ = std::fs::remove_dir_all(entry.path());
    }
}
