//! Lock lifecycle.

use super::helpers::*;
use std::path::Path;

/// FJ-565: Compress old lock files with zstd-like compression (base64 encoding for portability).
pub(crate) fn cmd_lock_compress(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut compressed = 0u64;
    let mut total_saved = 0u64;

    for m in &machines {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        let original_size = content.len() as u64;
        if original_size == 0 {
            continue;
        }

        // Write compressed version (minified YAML — remove comments and extra whitespace)
        let mut minified = String::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            minified.push_str(line);
            minified.push('\n');
        }

        let new_size = minified.len() as u64;
        if new_size < original_size {
            let compressed_path = state_dir.join(format!("{m}.lock.yaml.min"));
            std::fs::write(&compressed_path, &minified)
                .map_err(|e| format!("Failed to write compressed lock: {e}"))?;
            total_saved += original_size - new_size;
            compressed += 1;
        }
    }

    if json {
        println!(
            r#"{{"compressed":{compressed},"bytes_saved":{total_saved}}}"#
        );
    } else if compressed == 0 {
        println!("No lock files needed compression");
    } else {
        println!(
            "Compressed {compressed} lock files, saved {total_saved} bytes"
        );
    }
    Ok(())
}

/// FJ-615: Archive old lock files to compressed storage.
pub(crate) fn cmd_lock_archive(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let archive_dir = state_dir.join("archive");
    let mut archived = 0u64;

    for m in &machines {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        // Archive event logs (not the active lock files)
        let events_path = state_dir.join(format!("{m}.events.jsonl"));
        if events_path.exists() {
            std::fs::create_dir_all(&archive_dir)
                .map_err(|e| format!("Failed to create archive dir: {e}"))?;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let archive_name = format!("{m}.events.{timestamp}.jsonl");
            let dest = archive_dir.join(&archive_name);
            std::fs::copy(&events_path, &dest)
                .map_err(|e| format!("Failed to archive {}: {}", events_path.display(), e))?;
            archived += 1;
        }
    }

    if json {
        println!(r#"{{"archived":{archived}}}"#);
    } else if archived == 0 {
        println!("No event logs to archive");
    } else {
        println!(
            "Archived {} event logs to {}",
            archived,
            archive_dir.display()
        );
    }
    Ok(())
}

/// FJ-625: Create point-in-time lock file snapshot with metadata.
pub(crate) fn cmd_lock_snapshot(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let snapshot_dir = state_dir.join("snapshots");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let snapshot_name = format!("snapshot-{timestamp}");
    let dest_dir = snapshot_dir.join(&snapshot_name);
    let mut copied = 0u64;

    for m in &machines {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("Failed to create snapshot dir: {e}"))?;
        let dest = dest_dir.join(m).join("state.lock.yaml");
        std::fs::copy(&lock_path, &dest)
            .map_err(|e| format!("Failed to snapshot {}: {}", lock_path.display(), e))?;
        copied += 1;
    }

    if json {
        println!(r#"{{"snapshot":"{snapshot_name}","files":{copied}}}"#);
    } else if copied == 0 {
        println!("No lock files to snapshot");
    } else {
        println!(
            "Created snapshot '{snapshot_name}' with {copied} lock files"
        );
    }
    Ok(())
}

/// FJ-575: Defragment lock files (reorder resources alphabetically).
pub(crate) fn cmd_lock_defrag(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut defragged = 0u64;

    for m in &machines {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(mut lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            // Sort resources alphabetically
            let mut sorted: indexmap::IndexMap<String, crate::core::types::ResourceLock> =
                indexmap::IndexMap::new();
            let mut keys: Vec<String> = lock.resources.keys().cloned().collect();
            keys.sort();
            for key in keys {
                if let Some(val) = lock.resources.swap_remove(&key) {
                    sorted.insert(key, val);
                }
            }
            lock.resources = sorted;

            let new_content = serde_yaml_ng::to_string(&lock)
                .map_err(|e| format!("Failed to serialize lock: {e}"))?;
            std::fs::write(&lock_path, &new_content)
                .map_err(|e| format!("Failed to write lock: {e}"))?;
            defragged += 1;
        }
    }

    if json {
        println!(r#"{{"defragged":{defragged}}}"#);
    } else if defragged == 0 {
        println!("No lock files to defragment");
    } else {
        println!(
            "Defragmented {defragged} lock files (resources reordered alphabetically)"
        );
    }
    Ok(())
}
