//! Lock lifecycle.

use super::helpers::*;
use std::path::Path;

/// Minified YAML: blank lines and whole-line comments dropped, every other line
/// kept byte for byte.
fn minify_yaml(content: &str) -> String {
    let mut minified = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        minified.push_str(line);
        minified.push('\n');
    }
    minified
}

/// Writes `<machine>.lock.yaml.min` next to the state tree when minifying the
/// machine's lock actually saves bytes, and returns how many. `Ok(0)` when
/// there is no lock, it is empty, or minifying gains nothing — none of which is
/// an error.
fn compress_machine_lock(state_dir: &Path, machine: &str) -> Result<u64, String> {
    let lock_path = state_dir.join(machine).join("state.lock.yaml");
    if !lock_path.exists() {
        return Ok(0);
    }
    let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
    let original_size = content.len() as u64;
    if original_size == 0 {
        return Ok(0);
    }

    let minified = minify_yaml(&content);
    let new_size = minified.len() as u64;
    if new_size >= original_size {
        return Ok(0);
    }

    let compressed_path = state_dir.join(format!("{machine}.lock.yaml.min"));
    std::fs::write(&compressed_path, &minified)
        .map_err(|e| format!("Failed to write compressed lock: {e}"))?;
    Ok(original_size - new_size)
}

/// FJ-565: Compress old lock files with zstd-like compression (base64 encoding for portability).
pub(crate) fn cmd_lock_compress(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut compressed = 0u64;
    let mut total_saved = 0u64;

    for m in &machines {
        let saved = compress_machine_lock(state_dir, m)?;
        if saved > 0 {
            total_saved += saved;
            compressed += 1;
        }
    }

    if json {
        println!(r#"{{"compressed":{compressed},"bytes_saved":{total_saved}}}"#);
    } else if compressed == 0 {
        println!("No lock files needed compression");
    } else {
        println!("Compressed {compressed} lock files, saved {total_saved} bytes");
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
        let dest_machine_dir = dest_dir.join(m);
        std::fs::create_dir_all(&dest_machine_dir)
            .map_err(|e| format!("Failed to create snapshot dir: {e}"))?;
        let dest = dest_machine_dir.join("state.lock.yaml");
        std::fs::copy(&lock_path, &dest)
            .map_err(|e| format!("Failed to snapshot {}: {}", lock_path.display(), e))?;
        copied += 1;
    }

    if json {
        println!(r#"{{"snapshot":"{snapshot_name}","files":{copied}}}"#);
    } else if copied == 0 {
        println!("No lock files to snapshot");
    } else {
        println!("Created snapshot '{snapshot_name}' with {copied} lock files");
    }
    Ok(())
}

/// Atomically rewrite a lock file (temp + rename) and refresh its BLAKE3 `.b3`
/// integrity sidecar so the next `forjar apply` integrity check passes.
fn write_lock_and_sidecar(lock_path: &Path, content: &str) -> Result<(), String> {
    let tmp_path = lock_path.with_extension("yaml.tmp");
    std::fs::write(&tmp_path, content).map_err(|e| format!("Failed to write lock: {e}"))?;
    std::fs::rename(&tmp_path, lock_path)
        .map_err(|e| format!("Failed to rename lock into place: {e}"))?;
    crate::core::state::integrity::write_b3_sidecar(lock_path)
        .map_err(|e| format!("Failed to refresh integrity sidecar: {e}"))
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
            // FJ-154 (#20): a raw `std::fs::write` here left the BLAKE3 `.b3`
            // sidecar holding the pre-defrag hash, so the next `forjar apply`
            // hard-failed its integrity check and effectively bricked the stack
            // until a manual reseal. Write atomically and refresh the sidecar
            // (mirrors state::save_lock / reseal) so defrag → apply round-trips.
            write_lock_and_sidecar(&lock_path, &new_content)?;
            defragged += 1;
        }
    }

    if json {
        println!(r#"{{"defragged":{defragged}}}"#);
    } else if defragged == 0 {
        println!("No lock files to defragment");
    } else {
        println!("Defragmented {defragged} lock files (resources reordered alphabetically)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::state::integrity;

    /// A minimal valid `StateLock` YAML with resources deliberately out of
    /// alphabetical order (`zebra` before `alpha`) so defrag actually rewrites.
    fn sample_lock_yaml() -> String {
        "\
schema: v1
machine: testbox
hostname: testbox
generated_at: '2026-06-13T00:00:00Z'
generator: forjar-test
blake3_version: '1'
resources:
  zebra:
    type: package
    status: converged
    hash: aaaa
    details: {}
  alpha:
    type: file
    status: converged
    hash: bbbb
    details: {}
"
        .to_string()
    }

    #[test]
    fn defrag_refreshes_b3_sidecar_so_integrity_passes() {
        let state_dir = tempfile::tempdir().unwrap();
        let machine_dir = state_dir.path().join("testbox");
        std::fs::create_dir_all(&machine_dir).unwrap();
        let lock_path = machine_dir.join("state.lock.yaml");
        std::fs::write(&lock_path, sample_lock_yaml()).unwrap();

        // Write a STALE sidecar (hash of unrelated content). Before the fix,
        // defrag's raw write left this stale, so the next apply hard-failed.
        let sidecar = {
            let mut p = lock_path.as_os_str().to_owned();
            p.push(".b3");
            std::path::PathBuf::from(p)
        };
        std::fs::write(&sidecar, blake3::hash(b"stale").to_hex().as_str()).unwrap();

        cmd_lock_defrag(state_dir.path(), true).unwrap();

        // The sidecar now matches the rewritten lock contents.
        let rewritten = std::fs::read(&lock_path).unwrap();
        let expected = blake3::hash(&rewritten).to_hex().to_string();
        let on_disk = std::fs::read_to_string(&sidecar).unwrap();
        assert_eq!(on_disk.trim(), expected);

        // And the full integrity check (what apply runs) reports no errors.
        let results = integrity::verify_state_integrity(state_dir.path());
        assert!(
            !integrity::has_errors(&results),
            "defrag → apply integrity check should pass, got {results:?}"
        );

        // Resources were actually reordered (alpha before zebra) and no temp
        // file was left behind.
        let content = String::from_utf8(rewritten).unwrap();
        let alpha = content.find("alpha").unwrap();
        let zebra = content.find("zebra").unwrap();
        assert!(alpha < zebra, "resources should be sorted alphabetically");
        assert!(!lock_path.with_extension("yaml.tmp").exists());
    }
}
