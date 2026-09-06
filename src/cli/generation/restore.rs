//! The two halves of a generation that touch the state dir itself: copying it
//! into a snapshot, and copying a snapshot back over it.
//!
//! `restore_generation_to_state` is the destructive one — it EMPTIES the state
//! dir (bar `generations/`, `snapshots/` and `.snapshots/`) before copying the
//! generation back — which is why its only caller,
//! `rollback_to_generation`, asks `state_identity::refuse_multi_stack_restore`
//! first (PMAT-161).

use std::path::Path;

/// PMAT-161 (#469): refuse a restore in a state dir that more than one stack
/// has applied to.
///
/// It lives here, beside the destructive function it protects, because every
/// caller reaches that function: `undo`, `rollback --generation` (which has no
/// config in the picture at all) and `apply --rollback-on-failure`. A guard in
/// `undo` would leave the other two open.
///
/// Per-name stamps (#469) settled who a state dir belongs to; they did not make
/// the restore stack-scoped, and those are different questions. There is no
/// `--force` and `--yes` does not lift it — an override here is the data loss
/// with a flag on it. Stack-scoped restore is PMAT-162.
///
/// `generation` is `None` for `undo --resume`, which asks before it has read
/// the ledger that names its target.
///
/// Unchanged: a dir with no global lock, an unreadable one, a legacy 1.0 dir
/// (one stamp after migration) and any single-stack dir.
pub(crate) fn refuse_multi_stack_restore(
    state_dir: &Path,
    generation: Option<u32>,
) -> Result<(), String> {
    let Ok(Some(lock)) = crate::core::state::load_global_lock(state_dir) else {
        return Ok(());
    };
    let dir = std::fs::canonicalize(state_dir).unwrap_or_else(|_| state_dir.to_path_buf());
    let target = generation.map_or_else(
        || "the target generation".to_string(),
        |n| format!("generation {n}"),
    );
    crate::core::state::multi_stack_restore_refusal(&lock, &dir.display().to_string(), &target)
        .map_or(Ok(()), Err)
}

/// Copy state files into a generation directory, skipping generations/ and snapshots/.
pub(super) fn copy_state_to_generation(state_dir: &Path, target: &Path) -> Result<(), String> {
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "generations" || name == "snapshots" || name == ".snapshots" {
            continue;
        }
        let src = entry.path();
        let dst = target.join(&name);
        if src.is_dir() {
            std::fs::create_dir_all(&dst)
                .map_err(|e| format!("cannot create {}: {e}", dst.display()))?;
            crate::cli::snapshot::copy_dir_recursive(&src, &dst, "")?;
        } else {
            std::fs::copy(&src, &dst)
                .map_err(|e| format!("cannot copy {} → {}: {e}", src.display(), dst.display()))?;
        }
    }
    Ok(())
}

/// Restore state from a generation directory back to state_dir.
pub(super) fn restore_generation_to_state(gen_path: &Path, state_dir: &Path) -> Result<(), String> {
    // Remove current state (except generations/ and snapshots/)
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "generations" || name == "snapshots" || name == ".snapshots" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)
                .map_err(|e| format!("cannot remove {}: {e}", path.display()))?;
        } else {
            std::fs::remove_file(&path)
                .map_err(|e| format!("cannot remove {}: {e}", path.display()))?;
        }
    }

    // Copy generation contents back (skip metadata)
    let entries =
        std::fs::read_dir(gen_path).map_err(|e| format!("cannot read generation: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        // Both are ABOUT the generation, not part of the state it holds. The
        // recorded config in particular must not leak into the state dir: the
        // next generation writes its own from the config actually applied.
        if name == ".generation.yaml" || name == crate::cli::undo_replay::APPLIED_CONFIG {
            continue;
        }
        let src = entry.path();
        let dst = state_dir.join(&name);
        if src.is_dir() {
            std::fs::create_dir_all(&dst)
                .map_err(|e| format!("cannot create {}: {e}", dst.display()))?;
            crate::cli::snapshot::copy_dir_recursive(&src, &dst, "")?;
        } else {
            std::fs::copy(&src, &dst)
                .map_err(|e| format!("cannot copy {} → {}: {e}", src.display(), dst.display()))?;
        }
    }
    Ok(())
}

/// Atomically switch the `current` symlink to point to `target_dir`.
pub(super) fn atomic_symlink_switch(gen_dir: &Path, target_dir: &Path) -> Result<(), String> {
    let current_link = gen_dir.join("current");
    let tmp_link = gen_dir.join("current.tmp");

    let _ = std::fs::remove_file(&tmp_link);

    #[cfg(unix)]
    std::os::unix::fs::symlink(target_dir, &tmp_link)
        .map_err(|e| format!("cannot create temp symlink: {e}"))?;

    #[cfg(not(unix))]
    std::fs::write(&tmp_link, target_dir.to_string_lossy().as_bytes())
        .map_err(|e| format!("cannot create temp link: {e}"))?;

    std::fs::rename(&tmp_link, &current_link).map_err(|e| {
        format!(
            "cannot rename {} → {}: {e}",
            tmp_link.display(),
            current_link.display(),
        )
    })?;

    Ok(())
}
