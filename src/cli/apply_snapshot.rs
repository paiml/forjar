//! Pre-apply snapshots and numbered generations (FJ-1381, FJ-1386, GH-376).
//!
//! Extracted from `apply.rs`, which sits over the 500-line ratchet. Both
//! features hang off one config key, `policy.snapshot_generations`, which is
//! at once the on-switch and the retention count — and that overloading is
//! what made `forjar undo` inert on every default config, since undo has
//! nothing to target unless something here ran.
//!
//! # GH-376: a generation is recorded AFTER the apply, not before
//!
//! Both halves used to run before `executor::apply`. That made every recorded
//! generation internally inconsistent and `current` a lie:
//!
//! ```text
//!   gen 0   config_hash = v1   state.lock = (absent)     <- pre-v1
//!   gen 1   config_hash = v2   state.lock = v1
//!   gen 2 * config_hash = v3   state.lock = v2           <- `current`
//!   live                       state.lock = v3           <- no generation holds it
//! ```
//!
//! The metadata described the config being applied while the snapshot beside it
//! held the state from BEFORE that apply, so no generation ever paired a config
//! with the state it produced — snapshotting the config body into that layout
//! would have captured the very config the generation exists to undo. And after
//! N applies the newest generation was N−1 while the host was at N, so
//! `undo --generations 1` reached back TWO applies.
//!
//! Recording after a successful apply fixes both at the source: generation N
//! holds the lock apply N produced and the config that produced it, `current`
//! names the state the host is actually in, and `target = current - generations`
//! is once again the right arithmetic. The empty generation 0 disappears — the
//! first apply now records its own result, so the second apply is still
//! undoable, which is the property the old pre-apply gen 0 existed to preserve.
//!
//! The pre-apply NAMED snapshot (`snapshots/pre-apply-*`) is unaffected and
//! still runs before the apply, where it belongs.

use crate::core::types;
use std::path::Path;

thread_local! {
    /// GH-376: set while `undo` replays a recorded generation.
    static RECORDING_PAUSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Scope guard that stops the apply underneath it from appending a generation.
///
/// `undo` re-converges the host by applying the TARGET generation's recorded
/// config. Letting that apply append a fresh generation would leave `current`
/// past the generation the host is now in, so a second `undo` would target the
/// state it had just restored and report "no changes" — an undo that cannot be
/// repeated. Rolling the `current` pointer back and leaving it there is both
/// truthful and what `rollback --generation` already does.
pub(super) struct PauseGenerationRecording {
    previous: bool,
}

impl PauseGenerationRecording {
    pub(super) fn new() -> Self {
        let previous = RECORDING_PAUSED.with(|p| p.replace(true));
        Self { previous }
    }
}

impl Drop for PauseGenerationRecording {
    fn drop(&mut self) {
        RECORDING_PAUSED.with(|p| p.set(self.previous));
    }
}

/// FJ-1381: Auto-snapshot before apply if `snapshot_generations` is set.
///
/// A named snapshot copies the state dir, so it needs one to exist.
pub(super) fn maybe_auto_snapshot(
    config: &types::ForjarConfig,
    state_dir: &Path,
    dry_run: bool,
    verbose: bool,
) {
    let Some(gens) = config.policy.snapshot_generations else {
        return;
    };
    if gens == 0 || dry_run || !state_dir.exists() {
        return;
    }
    save_pre_apply_snapshot(state_dir, gens, verbose);
}

/// GH-376: record the generation the apply just produced, with the config that
/// produced it.
///
/// Called after EVERY apply, failures included: a generation is a record of
/// what happened, and `--rollback-on-failure` needs the failed run's generation
/// to rewind to. Gating this on success made a stack with one failing resource
/// record none at all, for ever.
pub(super) fn maybe_record_generation(
    config: &types::ForjarConfig,
    state_dir: &Path,
    dry_run: bool,
    verbose: bool,
) {
    let Some(gens) = config.policy.snapshot_generations else {
        return;
    };
    if gens == 0 || dry_run || RECORDING_PAUSED.with(std::cell::Cell::get) {
        return;
    }
    record_generation(state_dir, config, gens, verbose);
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
fn record_generation(state_dir: &Path, config: &types::ForjarConfig, gens: u32, verbose: bool) {
    match super::generation::create_generation(state_dir, Some(config)) {
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
///
/// PMAT-177: NOT in a state dir several stacks share. Retention counts the
/// snapshots in the DIR while the promise it keeps — `policy.snapshot_generations`
/// — is made by ONE config, so in a shared dir alpha's apply deleted the
/// pre-apply snapshots bravo and charlie made: the copies an operator reaches
/// for to recover from exactly this class of mistake. Snapshots carry no owner,
/// so there is nothing here to count per stack yet; attributing them is
/// PMAT-162, and until it lands the apply keeps every snapshot and says why.
///
/// PMAT-182: the condition itself lives in
/// [`state::stamp::retention::skip_note`], which the generation sweep asks
/// too. It was written here, inline, and the second retention path went a day
/// without it.
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
    if let Some(note) = crate::core::state::stamp::retention::skip_note(state_dir, "snapshot") {
        eprintln!("{note}");
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

#[cfg(test)]
mod tests_gc_scope {
    use super::*;
    use crate::core::state::{new_global_lock, save_global_lock};
    use crate::core::types::StackStamp;

    fn stamp(machine: &str) -> StackStamp {
        StackStamp {
            file: Some(format!("../{machine}/forjar.yaml")),
            last_apply: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar test".to_string(),
            machines: vec![machine.to_string()],
            outputs: Vec::new(),
        }
    }

    /// Plant `count` snapshot directories and return their names.
    fn plant(state_dir: &Path, count: usize) -> Vec<String> {
        let dir = super::super::snapshot::snapshots_dir(state_dir);
        std::fs::create_dir_all(&dir).unwrap();
        (0..count)
            .map(|i| {
                let name = format!("planted-{i:02}");
                std::fs::create_dir_all(dir.join(&name)).unwrap();
                name
            })
            .collect()
    }

    fn kept(state_dir: &Path) -> Vec<String> {
        let dir = super::super::snapshot::snapshots_dir(state_dir);
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    /// Write a global lock naming exactly `stacks` stacks.
    fn lock_with(state_dir: &Path, stacks: &[&str]) {
        let mut lock = new_global_lock(stacks[0]);
        for name in stacks {
            lock.stacks.insert((*name).to_string(), stamp(name));
        }
        save_global_lock(state_dir, &lock).unwrap();
    }

    /// PMAT-182: the sweep asks the SHARED helper, so the two retention paths
    /// cannot answer this differently — which is exactly how the generation
    /// sweep was left unguarded when this one was fixed.
    #[test]
    fn the_sweep_asks_the_shared_helper_about_this_dir() {
        let dir = tempfile::tempdir().unwrap();
        let skip = |d: &Path| crate::core::state::stamp::retention::skip_note(d, "snapshot");
        assert_eq!(skip(dir.path()), None, "no global lock");
        lock_with(dir.path(), &["alpha"]);
        assert_eq!(skip(dir.path()), None, "one stack");
        lock_with(dir.path(), &["alpha", "bravo"]);
        assert_eq!(
            skip(dir.path()).as_deref(),
            Some("note: snapshot gc skipped: state dir holds 2 stacks (PMAT-162)")
        );
    }

    /// PMAT-177: the deletion itself is what the shared dir is spared.
    #[test]
    fn gc_keeps_every_snapshot_in_a_shared_dir() {
        let dir = tempfile::tempdir().unwrap();
        let planted = plant(dir.path(), 12);
        lock_with(dir.path(), &["alpha", "bravo"]);
        gc_old_snapshots(dir.path(), 2, false);
        assert_eq!(kept(dir.path()), planted, "a shared dir loses no snapshot");
    }

    /// ANTI-VACUITY: the single-stack dir still prunes to its keep count.
    #[test]
    fn gc_still_prunes_a_single_stack_dir() {
        let dir = tempfile::tempdir().unwrap();
        plant(dir.path(), 12);
        lock_with(dir.path(), &["alpha"]);
        gc_old_snapshots(dir.path(), 2, false);
        assert_eq!(
            kept(dir.path()),
            ["planted-10", "planted-11"],
            "retention is unchanged where the dir has one owner"
        );
    }
}
