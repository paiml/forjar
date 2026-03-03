//! FJ-1386: Generational state snapshots — Nix-style numbered generations with instant rollback.
//!
//! Each `forjar apply` creates a new generation under `state/generations/`.
//! Generations are numbered sequentially (0, 1, 2, ...) with a `current`
//! symlink pointing to the active generation. Rollback switches the symlink
//! atomically via temp-symlink + rename(2).

use std::path::{Path, PathBuf};

/// Directory holding numbered generation snapshots.
fn generations_dir(state_dir: &Path) -> PathBuf {
    state_dir.join("generations")
}

/// Create a new generation from the current state directory contents.
/// Returns the new generation number.
pub(crate) fn create_generation(state_dir: &Path) -> Result<u32, String> {
    let gen_dir = generations_dir(state_dir);
    std::fs::create_dir_all(&gen_dir)
        .map_err(|e| format!("cannot create generations dir: {e}"))?;

    let next = next_generation_number(&gen_dir)?;
    let target = gen_dir.join(next.to_string());
    std::fs::create_dir_all(&target)
        .map_err(|e| format!("cannot create generation {next}: {e}"))?;

    // Copy state files (skip generations/ and snapshots/ directories)
    copy_state_to_generation(state_dir, &target)?;

    // Write metadata
    let meta = format!(
        "generation: {next}\ncreated_at: \"{}\"\n",
        crate::tripwire::eventlog::now_iso8601(),
    );
    std::fs::write(target.join(".generation.yaml"), meta)
        .map_err(|e| format!("cannot write generation metadata: {e}"))?;

    // Atomically switch current symlink
    atomic_symlink_switch(&gen_dir, &target)?;

    Ok(next)
}

/// Rollback to a specific generation number.
pub(crate) fn rollback_to_generation(
    state_dir: &Path,
    generation: u32,
    yes: bool,
) -> Result<(), String> {
    if !yes {
        return Err(
            "rollback --generation requires --yes to confirm state overwrite".to_string(),
        );
    }
    let gen_dir = generations_dir(state_dir);
    let target = gen_dir.join(generation.to_string());
    if !target.exists() {
        return Err(format!("generation {generation} does not exist"));
    }

    // Restore state from generation snapshot
    restore_generation_to_state(&target, state_dir)?;

    // Switch current symlink
    atomic_symlink_switch(&gen_dir, &target)?;

    println!("Rolled back to generation {generation}");
    Ok(())
}

/// List all generations with metadata.
pub(crate) fn list_generations(state_dir: &Path, json: bool) -> Result<(), String> {
    let gen_dir = generations_dir(state_dir);
    if !gen_dir.exists() {
        if json {
            println!("[]");
        } else {
            println!("No generations.");
        }
        return Ok(());
    }

    let current = current_generation(&gen_dir);
    let mut gens = collect_generations(&gen_dir)?;
    gens.sort_by_key(|(n, _)| *n);

    if json {
        print_generations_json(&gens, current)?;
    } else if gens.is_empty() {
        println!("No generations.");
    } else {
        print_generations_text(&gens, current);
    }
    Ok(())
}

/// Get the current generation number.
pub(crate) fn current_generation(gen_dir: &Path) -> Option<u32> {
    let current = gen_dir.join("current");
    let target = std::fs::read_link(&current).ok()?;
    let name = target.file_name()?.to_string_lossy().to_string();
    name.parse().ok()
}

/// Garbage-collect old generations, keeping only the newest `keep` entries.
pub(crate) fn gc_generations(state_dir: &Path, keep: u32, verbose: bool) {
    let gen_dir = generations_dir(state_dir);
    if !gen_dir.exists() {
        return;
    }
    let Ok(mut gens) = collect_generation_numbers(&gen_dir) else { return };
    if gens.len() <= keep as usize {
        return;
    }
    gens.sort();
    let to_remove = gens.len() - keep as usize;
    for &gen_num in gens.iter().take(to_remove) {
        let path = gen_dir.join(gen_num.to_string());
        if verbose {
            eprintln!("generation gc: removing generation {gen_num}");
        }
        let _ = std::fs::remove_dir_all(path);
    }
}

// ── Internal helpers ───────────────────────────────────────────────

/// Find the next generation number by scanning existing directories.
fn next_generation_number(gen_dir: &Path) -> Result<u32, String> {
    let nums = collect_generation_numbers(gen_dir)?;
    Ok(nums.into_iter().max().map_or(0, |m| m + 1))
}

/// Collect all generation numbers from the generations directory.
fn collect_generation_numbers(gen_dir: &Path) -> Result<Vec<u32>, String> {
    let entries =
        std::fs::read_dir(gen_dir).map_err(|e| format!("cannot read generations dir: {e}"))?;
    Ok(entries
        .flatten()
        .filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok())
        .collect())
}

/// Collect generations as (number, created_at) pairs.
fn collect_generations(gen_dir: &Path) -> Result<Vec<(u32, String)>, String> {
    let entries =
        std::fs::read_dir(gen_dir).map_err(|e| format!("cannot read generations dir: {e}"))?;
    let mut gens = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Ok(num) = name.parse::<u32>() {
            let meta_path = entry.path().join(".generation.yaml");
            let created = read_created_at(&meta_path);
            gens.push((num, created));
        }
    }
    Ok(gens)
}

/// Read created_at from a .generation.yaml metadata file.
fn read_created_at(meta_path: &Path) -> String {
    std::fs::read_to_string(meta_path)
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("created_at:"))
                .map(|l| {
                    l.trim_start_matches("created_at:")
                        .trim()
                        .trim_matches('"')
                        .to_string()
                })
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// Copy state files into a generation directory, skipping generations/ and snapshots/.
fn copy_state_to_generation(state_dir: &Path, target: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir: {e}"))?;
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
            super::snapshot::copy_dir_recursive(&src, &dst, "")?;
        } else {
            std::fs::copy(&src, &dst).map_err(|e| {
                format!("cannot copy {} → {}: {e}", src.display(), dst.display())
            })?;
        }
    }
    Ok(())
}

/// Restore state from a generation directory back to state_dir.
fn restore_generation_to_state(gen_path: &Path, state_dir: &Path) -> Result<(), String> {
    // Remove current state (except generations/ and snapshots/)
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir: {e}"))?;
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
    let entries = std::fs::read_dir(gen_path)
        .map_err(|e| format!("cannot read generation: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".generation.yaml" {
            continue;
        }
        let src = entry.path();
        let dst = state_dir.join(&name);
        if src.is_dir() {
            std::fs::create_dir_all(&dst)
                .map_err(|e| format!("cannot create {}: {e}", dst.display()))?;
            super::snapshot::copy_dir_recursive(&src, &dst, "")?;
        } else {
            std::fs::copy(&src, &dst).map_err(|e| {
                format!("cannot copy {} → {}: {e}", src.display(), dst.display())
            })?;
        }
    }
    Ok(())
}

/// Atomically switch the `current` symlink to point to `target_dir`.
fn atomic_symlink_switch(gen_dir: &Path, target_dir: &Path) -> Result<(), String> {
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

/// Print generations as JSON.
fn print_generations_json(
    gens: &[(u32, String)],
    current: Option<u32>,
) -> Result<(), String> {
    let items: Vec<serde_json::Value> = gens
        .iter()
        .map(|(n, c)| {
            serde_json::json!({
                "generation": n,
                "created_at": c,
                "current": current == Some(*n),
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&items).map_err(|e| format!("JSON error: {e}"))?
    );
    Ok(())
}

/// Print generations as text table.
fn print_generations_text(gens: &[(u32, String)], current: Option<u32>) {
    for (num, created) in gens {
        let marker = if current == Some(*num) { " *" } else { "" };
        println!("  gen {num}{marker} ({created})");
    }
}
