//! PMAT-565: `forjar lock --restamp` — every lock under a state dir names
//! the binary that wrote it, in ONE run.
//!
//! `state::save_lock` has stamped the writer on every write since PMAT-565,
//! so an apply corrects the lock of every machine it touches. The fleet's
//! locks are not all touched by one apply — a manifest names its own
//! machines — and "converges on the next incidental write" is how a wrong
//! `generator` survived from 1.1.1 to 1.30.0. This walks the dir and writes
//! each lock through the same writer, sidecar included, and says what moved.

use crate::core::state::{load_lock, save_lock, stamped_for_write, writer_stamp};
use std::path::Path;

/// One lock's before/after, for the report.
struct Restamp {
    machine: String,
    before: String,
    created_by: Option<String>,
}

/// Every `<machine>/state.lock.yaml` under `state_dir`, by machine name.
fn machines_with_locks(state_dir: &Path) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {e}", state_dir.display()))?;
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().join("state.lock.yaml").is_file())
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect();
    names.sort();
    Ok(names)
}

/// Rewrite every lock whose `generator` is not this binary.
pub(crate) fn cmd_lock_restamp(state_dir: &Path, dry_run: bool, json: bool) -> Result<(), String> {
    let writer = writer_stamp();
    let mut done: Vec<Restamp> = Vec::new();
    let mut unchanged = 0usize;
    for machine in machines_with_locks(state_dir)? {
        let Some(lock) = load_lock(state_dir, &machine)? else {
            continue;
        };
        if lock.generator == writer {
            unchanged += 1;
            continue;
        }
        let stamped = stamped_for_write(&lock);
        if !dry_run {
            save_lock(state_dir, &lock)?;
        }
        done.push(Restamp {
            machine,
            before: lock.generator.clone(),
            created_by: stamped.created_by,
        });
    }
    report(&done, unchanged, &writer, dry_run, json)
}

fn report(
    done: &[Restamp],
    unchanged: usize,
    writer: &str,
    dry_run: bool,
    json: bool,
) -> Result<(), String> {
    if json {
        let rows: Vec<serde_json::Value> = done
            .iter()
            .map(|r| {
                serde_json::json!({
                    "machine": r.machine, "before": r.before, "after": writer,
                    "created_by": r.created_by,
                })
            })
            .collect();
        let out = serde_json::json!({
            "writer": writer, "dry_run": dry_run,
            "restamped": done.len(), "unchanged": unchanged, "locks": rows,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| format!("JSON error: {e}"))?
        );
        return Ok(());
    }
    let verb = if dry_run {
        "would restamp"
    } else {
        "restamped"
    };
    for r in done {
        println!(
            "  {verb} {}: {} -> {writer} (created_by: {})",
            r.machine,
            r.before,
            r.created_by.as_deref().unwrap_or("-")
        );
    }
    println!(
        "{} lock(s) {}, {unchanged} already named {writer}",
        done.len(),
        if dry_run {
            "would be restamped"
        } else {
            "restamped"
        }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::state::new_lock;

    #[test]
    fn a_dir_with_no_locks_reports_zero() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_lock_restamp(d.path(), false, false).is_ok());
    }

    #[test]
    fn an_unreadable_state_dir_is_an_error_not_zero() {
        let d = tempfile::tempdir().unwrap();
        let missing = d.path().join("nope");
        assert!(cmd_lock_restamp(&missing, false, false).is_err());
    }

    #[test]
    fn a_lock_already_naming_this_binary_is_left_alone() {
        let d = tempfile::tempdir().unwrap();
        let lock = new_lock("m", "m");
        save_lock(d.path(), &lock).unwrap();
        let before = std::fs::read_to_string(d.path().join("m/state.lock.yaml")).unwrap();
        cmd_lock_restamp(d.path(), false, false).unwrap();
        let after = std::fs::read_to_string(d.path().join("m/state.lock.yaml")).unwrap();
        assert_eq!(before, after, "an idempotent second pass rewrites nothing");
    }
}
