//! FJ-1386: Generational state snapshots — Nix-style numbered generations with instant rollback.
//!
//! Each `forjar apply` creates a new generation under `state/generations/`.
//! Generations are numbered sequentially (0, 1, 2, ...) with a `current`
//! symlink pointing to the active generation. Rollback switches the symlink
//! atomically via temp-symlink + rename(2).

use crate::core::types::GenerationMeta;
use std::path::{Path, PathBuf};

/// Directory holding numbered generation snapshots.
pub(super) fn generations_dir(state_dir: &Path) -> PathBuf {
    state_dir.join("generations")
}

/// Create a new generation from the current state directory contents.
/// Returns the new generation number.
///
/// If `config_path` is provided, computes a BLAKE3 hash of the config file
/// and stores it in the generation metadata for config tracking.
pub(crate) fn create_generation(
    state_dir: &Path,
    config_path: Option<&Path>,
) -> Result<u32, String> {
    let gen_dir = generations_dir(state_dir);
    std::fs::create_dir_all(&gen_dir).map_err(|e| format!("cannot create generations dir: {e}"))?;

    let next = next_generation_number(&gen_dir)?;
    let target = gen_dir.join(next.to_string());
    std::fs::create_dir_all(&target)
        .map_err(|e| format!("cannot create generation {next}: {e}"))?;

    // Copy state files (skip generations/ and snapshots/ directories)
    copy_state_to_generation(state_dir, &target)?;

    // Write metadata using GenerationMeta (FJ-2002)
    let mut meta = GenerationMeta::new(next, crate::tripwire::eventlog::now_iso8601());
    if let Some(git_ref) = crate::core::types::get_git_ref() {
        meta = meta.with_git_ref(git_ref);
    }
    if let Some(cfg_path) = config_path {
        if let Ok(bytes) = std::fs::read(cfg_path) {
            let hash = blake3::hash(&bytes).to_hex().to_string();
            meta = meta.with_config_hash(format!("blake3:{hash}"));
        }
    }
    meta.forjar_version = Some(env!("CARGO_PKG_VERSION").to_string());
    let meta_yaml = meta.to_yaml()?;
    std::fs::write(target.join(".generation.yaml"), meta_yaml)
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
        return Err("rollback --generation requires --yes to confirm state overwrite".to_string());
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
        print_generations_json(&gens, current, &gen_dir)?;
    } else if gens.is_empty() {
        println!("No generations.");
    } else {
        print_generations_text(&gens, current, &gen_dir);
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
    let Ok(mut gens) = collect_generation_numbers(&gen_dir) else {
        return;
    };
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
pub(super) fn next_generation_number(gen_dir: &Path) -> Result<u32, String> {
    let nums = collect_generation_numbers(gen_dir)?;
    Ok(nums.into_iter().max().map_or(0, |m| m + 1))
}

/// Collect all generation numbers from the generations directory.
pub(super) fn collect_generation_numbers(gen_dir: &Path) -> Result<Vec<u32>, String> {
    let entries =
        std::fs::read_dir(gen_dir).map_err(|e| format!("cannot read generations dir: {e}"))?;
    Ok(entries
        .flatten()
        .filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok())
        .collect())
}

/// Enriched generation info for display.
pub(super) struct GenInfo {
    pub(super) num: u32,
    pub(super) created_at: String,
    pub(super) action: String,
    pub(super) changes: u32,
    pub(super) resource_count: usize,
}

/// Collect generations with enriched metadata (FJ-2002).
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

/// Read enriched metadata from a generation directory.
pub(super) fn read_gen_info(gen_dir: &Path, num: u32) -> GenInfo {
    let meta_path = gen_dir.join(num.to_string()).join(".generation.yaml");
    let content = std::fs::read_to_string(&meta_path).unwrap_or_default();
    match GenerationMeta::from_yaml(&content) {
        Ok(meta) => {
            let resource_count = count_lock_resources(&gen_dir.join(num.to_string()));
            let changes = meta.total_changes();
            GenInfo {
                num,
                created_at: meta.created_at,
                action: meta.action,
                changes,
                resource_count,
            }
        }
        Err(_) => GenInfo {
            num,
            created_at: read_created_at(&meta_path),
            action: "apply".into(),
            changes: 0,
            resource_count: 0,
        },
    }
}

/// Count resources in lock files within a generation snapshot.
pub(super) fn count_lock_resources(gen_path: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(gen_path) {
        for entry in entries.flatten() {
            let lock_path = entry.path().join("state.lock.yaml");
            if let Ok(content) = std::fs::read_to_string(&lock_path) {
                if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content)
                {
                    count += lock.resources.len();
                }
            }
        }
    }
    count
}

/// Read created_at from a .generation.yaml metadata file.
pub(super) fn read_created_at(meta_path: &Path) -> String {
    std::fs::read_to_string(meta_path)
        .ok()
        .and_then(|c| {
            c.lines().find(|l| l.starts_with("created_at:")).map(|l| {
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
            super::snapshot::copy_dir_recursive(&src, &dst, "")?;
        } else {
            std::fs::copy(&src, &dst)
                .map_err(|e| format!("cannot copy {} → {}: {e}", src.display(), dst.display()))?;
        }
    }
    Ok(())
}

/// Restore state from a generation directory back to state_dir.
fn restore_generation_to_state(gen_path: &Path, state_dir: &Path) -> Result<(), String> {
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
            std::fs::copy(&src, &dst)
                .map_err(|e| format!("cannot copy {} → {}: {e}", src.display(), dst.display()))?;
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
    gen_dir: &Path,
) -> Result<(), String> {
    let items: Vec<serde_json::Value> = gens
        .iter()
        .map(|(n, _)| {
            let info = read_gen_info(gen_dir, *n);
            serde_json::json!({
                "generation": n,
                "created_at": info.created_at,
                "current": current == Some(*n),
                "action": info.action,
                "changes": info.changes,
                "resources": info.resource_count,
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&items).map_err(|e| format!("JSON error: {e}"))?
    );
    Ok(())
}

/// FJ-2003: Diff two generations by number.
pub(crate) fn cmd_generation_diff(
    state_dir: &Path,
    from: u32,
    to: u32,
    json: bool,
) -> Result<(), String> {
    use crate::core::types::{diff_resource_sets, GenerationDiff};
    let gen_dir = generations_dir(state_dir);
    let from_dir = gen_dir.join(from.to_string());
    let to_dir = gen_dir.join(to.to_string());
    if !from_dir.exists() {
        return Err(format!("generation {from} not found"));
    }
    if !to_dir.exists() {
        return Err(format!("generation {to} not found"));
    }

    let from_locks = load_gen_locks(&from_dir);
    let to_locks = load_gen_locks(&to_dir);
    let all_machines: std::collections::BTreeSet<&str> = from_locks
        .keys()
        .map(|s| s.as_str())
        .chain(to_locks.keys().map(|s| s.as_str()))
        .collect();

    let mut diffs = Vec::new();
    for machine in &all_machines {
        let from_res = lock_to_tuples(from_locks.get(*machine));
        let to_res = lock_to_tuples(to_locks.get(*machine));
        let from_refs: Vec<(&str, &str, &str)> = from_res
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
            .collect();
        let to_refs: Vec<(&str, &str, &str)> = to_res
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
            .collect();
        let resources = diff_resource_sets(&from_refs, &to_refs);
        diffs.push(GenerationDiff {
            gen_from: from,
            gen_to: to,
            machine: machine.to_string(),
            resources,
        });
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&diffs).map_err(|e| format!("JSON error: {e}"))?
        );
    } else {
        for diff in &diffs {
            print!("{}", diff.format_summary());
        }
        if diffs.iter().all(|d| !d.has_changes()) {
            println!("No changes between generation {from} and {to}.");
        }
    }
    Ok(())
}

/// Load locks from a generation directory (all machines).
pub(super) fn load_gen_locks(
    gen_dir: &Path,
) -> std::collections::HashMap<String, crate::core::types::StateLock> {
    let mut locks = std::collections::HashMap::new();
    let Ok(entries) = std::fs::read_dir(gen_dir) else {
        return locks;
    };
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let lock_path = entry.path().join("state.lock.yaml");
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
                locks.insert(name, lock);
            }
        }
    }
    locks
}

/// Convert a StateLock to (resource_id, resource_type, hash) tuples.
pub(super) fn lock_to_tuples(
    lock: Option<&crate::core::types::StateLock>,
) -> Vec<(String, String, String)> {
    let Some(lock) = lock else { return Vec::new() };
    lock.resources
        .iter()
        .map(|(id, rl)| (id.clone(), rl.resource_type.to_string(), rl.hash.clone()))
        .collect()
}

/// Print generations as enriched text table (FJ-2002).
fn print_generations_text(gens: &[(u32, String)], current: Option<u32>, gen_dir: &Path) {
    for (num, _) in gens {
        let info = read_gen_info(gen_dir, *num);
        let marker = if current == Some(*num) { " *" } else { "" };
        let delta = if info.changes > 0 {
            format!(
                " ({} changes, {} resources)",
                info.changes, info.resource_count
            )
        } else if info.resource_count > 0 {
            format!(" ({} resources)", info.resource_count)
        } else {
            String::new()
        };
        println!(
            "  gen {}{marker} [{}] ({}){delta}",
            info.num, info.action, info.created_at
        );
    }
}
