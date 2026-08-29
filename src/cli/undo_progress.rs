//! The `undo-progress.yaml` ledger that makes a half-finished undo resumable.
//!
//! GH-376: it was written BEFORE `rollback_to_generation`, which deletes
//! `state_dir/<machine>/` wholesale and so removed every file this module had
//! just written. Measured after a genuinely failing undo: no `undo-progress.yaml`
//! anywhere on disk, and `forjar undo --resume --yes` answered "no partial undo
//! found — nothing to resume" with exit 2. `mark_undo_progress_final` was a
//! permanent no-op. `cmd_undo` now writes the ledger after the rollback.

use crate::core::types;
use std::path::Path;

/// Write undo progress to `undo-progress.yaml` in the machine's state directory.
pub(super) fn write_undo_progress(state_dir: &Path, machine: &str, progress: &types::UndoProgress) {
    let dir = state_dir.join(machine);
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("undo-progress.yaml");
    if let Ok(yaml) = serde_yaml_ng::to_string(progress) {
        let _ = std::fs::write(path, yaml);
    }
}

/// Read undo progress from a machine's state directory.
pub(super) fn read_undo_progress(state_dir: &Path, machine: &str) -> Option<types::UndoProgress> {
    let path = state_dir.join(machine).join("undo-progress.yaml");
    let content = std::fs::read_to_string(path).ok()?;
    serde_yaml_ng::from_str(&content).ok()
}

/// Initialize undo progress for all affected resources.
pub(super) fn init_undo_progress(
    current: u32,
    target: u32,
    changes: &[String],
) -> types::UndoProgress {
    let mut resources = std::collections::HashMap::new();
    for c in changes {
        let rid = c.split_whitespace().nth(1).unwrap_or("unknown");
        resources.insert(
            rid.to_string(),
            types::ResourceProgress {
                status: types::ResourceProgressStatus::Pending,
                at: None,
            },
        );
    }
    types::UndoProgress {
        generation_from: current,
        generation_to: target,
        started_at: crate::tripwire::eventlog::now_iso8601(),
        status: types::UndoStatus::InProgress,
        resources,
    }
}

/// Stamp every machine's `undo-progress.yaml` Completed or Partial.
pub(super) fn mark_undo_progress_final<'a>(
    state_dir: &Path,
    machines: impl Iterator<Item = &'a String>,
    ok: bool,
) {
    let final_status = if ok {
        types::UndoStatus::Completed
    } else {
        types::UndoStatus::Partial
    };
    for machine in machines {
        if let Some(mut p) = read_undo_progress(state_dir, machine) {
            p.status = final_status;
            write_undo_progress(state_dir, machine, &p);
        }
    }
}
