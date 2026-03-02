//! FJ-1302: Profile generation management.
//!
//! Profiles are named symlink trees that point to store entries.
//! Each `create_generation` atomically switches a `current` symlink
//! to a new numbered generation, enabling instant rollback.

use std::path::Path;

/// Create a new generation pointing to the given store content path.
///
/// Returns the new generation number. The `current` symlink is
/// atomically switched via temp symlink + rename.
pub fn create_generation(profiles_dir: &Path, store_content_path: &str) -> Result<u32, String> {
    std::fs::create_dir_all(profiles_dir)
        .map_err(|e| format!("cannot create profiles dir: {}", e))?;

    let next_gen = next_generation_number(profiles_dir)?;
    let gen_dir = profiles_dir.join(next_gen.to_string());
    std::fs::create_dir_all(&gen_dir)
        .map_err(|e| format!("cannot create generation dir: {}", e))?;

    // Write a link target file (the store path this generation points to)
    std::fs::write(gen_dir.join("target"), store_content_path)
        .map_err(|e| format!("cannot write target: {}", e))?;

    // Atomically switch the `current` symlink
    atomic_symlink_switch(profiles_dir, &gen_dir)?;

    Ok(next_gen)
}

/// Rollback to the previous generation.
///
/// Returns the generation number rolled back to.
pub fn rollback(profiles_dir: &Path) -> Result<u32, String> {
    let current = current_generation(profiles_dir)
        .ok_or_else(|| "no current generation to rollback from".to_string())?;
    if current == 0 {
        return Err("cannot rollback past generation 0".to_string());
    }
    let prev = current - 1;
    let prev_dir = profiles_dir.join(prev.to_string());
    if !prev_dir.exists() {
        return Err(format!("generation {} does not exist", prev));
    }
    atomic_symlink_switch(profiles_dir, &prev_dir)?;
    Ok(prev)
}

/// List all generations as (number, target) pairs, sorted ascending.
pub fn list_generations(profiles_dir: &Path) -> Result<Vec<(u32, String)>, String> {
    if !profiles_dir.exists() {
        return Ok(Vec::new());
    }
    let mut gens = Vec::new();
    let entries = std::fs::read_dir(profiles_dir)
        .map_err(|e| format!("cannot read profiles dir: {}", e))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Ok(num) = name.parse::<u32>() {
            let target_path = entry.path().join("target");
            let target = std::fs::read_to_string(&target_path).unwrap_or_default();
            gens.push((num, target));
        }
    }
    gens.sort_by_key(|(n, _)| *n);
    Ok(gens)
}

/// Get the current generation number (from the `current` symlink).
pub fn current_generation(profiles_dir: &Path) -> Option<u32> {
    let current = profiles_dir.join("current");
    let target = std::fs::read_link(&current).ok()?;
    let name = target.file_name()?.to_string_lossy().to_string();
    name.parse().ok()
}

/// Compute the next generation number by scanning existing directories.
fn next_generation_number(profiles_dir: &Path) -> Result<u32, String> {
    let entries = std::fs::read_dir(profiles_dir)
        .map_err(|e| format!("cannot read profiles dir: {}", e))?;
    let max = entries
        .flatten()
        .filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok())
        .max();
    Ok(max.map_or(0, |m| m + 1))
}

/// Atomically switch the `current` symlink to point to `target_dir`.
///
/// Uses temp symlink + rename(2) for crash safety.
fn atomic_symlink_switch(profiles_dir: &Path, target_dir: &Path) -> Result<(), String> {
    let current_link = profiles_dir.join("current");
    let tmp_link = profiles_dir.join("current.tmp");

    // Remove stale temp link if present
    let _ = std::fs::remove_file(&tmp_link);

    #[cfg(unix)]
    std::os::unix::fs::symlink(target_dir, &tmp_link)
        .map_err(|e| format!("cannot create temp symlink: {}", e))?;

    #[cfg(not(unix))]
    std::fs::write(&tmp_link, target_dir.to_string_lossy().as_bytes())
        .map_err(|e| format!("cannot create temp link: {}", e))?;

    std::fs::rename(&tmp_link, &current_link).map_err(|e| {
        format!(
            "cannot rename {} → {}: {}",
            tmp_link.display(),
            current_link.display(),
            e
        )
    })?;

    Ok(())
}
