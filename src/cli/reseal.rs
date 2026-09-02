//! FJ-118: `reseal` subcommand — regenerate BLAKE3 integrity sidecars from
//! current lock contents without running a full apply.
//!
//! Use when `forjar apply` reports `integrity check failed for <X>: expected
//! <old>, got <new>` after a partial write (old forjar versions silently
//! discarded sidecar-write errors) or after a `git checkout` / merge that
//! restored one file without the other.

use super::helpers_state::list_state_machines;
use std::path::{Path, PathBuf};

/// Collect the target list for `reseal`.
fn collect_targets(
    state_dir: &Path,
    file: Option<PathBuf>,
    all: bool,
    machine: Option<String>,
) -> Result<Vec<PathBuf>, String> {
    if let Some(f) = file {
        if !f.exists() {
            return Err(format!("file not found: {}", f.display()));
        }
        return Ok(vec![f]);
    }
    if let Some(m) = machine {
        let p = state_dir.join(&m).join("state.lock.yaml");
        if !p.exists() {
            return Err(format!("no lock file for machine {m} at {}", p.display()));
        }
        return Ok(vec![p]);
    }
    if all {
        return collect_all_targets(state_dir);
    }
    Err("reseal requires one of --file, --machine, or --all".into())
}

/// Walk state_dir for every machine's lock file + global lock.
fn collect_all_targets(state_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !state_dir.exists() {
        return Err(format!("state dir not found: {}", state_dir.display()));
    }
    let mut targets: Vec<PathBuf> = list_state_machines(state_dir)?
        .iter()
        .map(|m| state_dir.join(m).join("state.lock.yaml"))
        .filter(|p| p.exists())
        .collect();
    let global = state_dir.join("forjar.lock.yaml");
    if global.exists() {
        targets.push(global);
    }
    Ok(targets)
}

/// Reseal a single target. YAML-parses the lock first so a corrupt file
/// cannot be silently blessed with a fresh sidecar. Returns `true` if
/// the sidecar was resealed (or would be under --dry-run), `false` if
/// the sidecar write failed but the lock was valid.
fn reseal_one(target: &Path, dry_run: bool) -> Result<bool, String> {
    use crate::core::state::integrity;

    let bytes =
        std::fs::read(target).map_err(|e| format!("cannot read {}: {}", target.display(), e))?;
    serde_yaml_ng::from_slice::<serde_yaml_ng::Value>(&bytes)
        .map_err(|e| format!("{} is not valid YAML: {}", target.display(), e))?;

    if dry_run {
        println!("[dry-run] would reseal {}", target.display());
        return Ok(true);
    }

    match integrity::write_b3_sidecar(target) {
        Ok(()) => {
            println!("resealed {}", target.display());
            Ok(true)
        }
        Err(e) => {
            eprintln!("skip {}: {}", target.display(), e);
            Ok(false)
        }
    }
}

/// Regenerate BLAKE3 integrity sidecars from current lock contents.
pub(crate) fn cmd_reseal(
    state_dir: &Path,
    file: Option<PathBuf>,
    all: bool,
    machine: Option<String>,
    dry_run: bool,
) -> Result<(), String> {
    let targets = collect_targets(state_dir, file, all, machine)?;
    if targets.is_empty() {
        println!("nothing to reseal");
        return Ok(());
    }

    let mut resealed = 0usize;
    let mut skipped = 0usize;
    for target in &targets {
        if reseal_one(target, dry_run)? {
            resealed += 1;
        } else {
            skipped += 1;
        }
    }

    println!(
        "{}{resealed} resealed, {skipped} skipped",
        if dry_run { "[dry-run] " } else { "" },
    );
    if skipped > 0 {
        return Err(format!("{skipped} sidecar(s) failed to write"));
    }
    Ok(())
}
