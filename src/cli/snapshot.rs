//! Snapshot management.

use std::path::{Path, PathBuf};


pub(crate) fn cmd_snapshot_save(name: &str, state_dir: &Path) -> Result<(), String> {
    if !state_dir.exists() {
        return Err(format!(
            "state directory does not exist: {}",
            state_dir.display()
        ));
    }
    let snap_dir = snapshots_dir(state_dir).join(name);
    if snap_dir.exists() {
        return Err(format!("snapshot '{}' already exists", name));
    }
    std::fs::create_dir_all(&snap_dir).map_err(|e| format!("cannot create snapshot dir: {}", e))?;

    // Copy all files/dirs in state_dir except "snapshots" itself
    copy_dir_recursive(state_dir, &snap_dir, "snapshots")?;

    // Write metadata
    let meta = format!(
        "created_at: \"{}\"\nname: \"{}\"\n",
        crate::tripwire::eventlog::now_iso8601(),
        name
    );
    std::fs::write(snap_dir.join(".snapshot.yaml"), meta)
        .map_err(|e| format!("cannot write snapshot metadata: {}", e))?;

    println!("Snapshot saved: {}", name);
    Ok(())
}


pub(crate) fn cmd_snapshot_list(state_dir: &Path, json: bool) -> Result<(), String> {
    let snap_base = snapshots_dir(state_dir);
    if !snap_base.exists() {
        if json {
            println!("[]");
        } else {
            println!("No snapshots.");
        }
        return Ok(());
    }

    let mut snapshots: Vec<(String, String)> = Vec::new();
    let entries =
        std::fs::read_dir(&snap_base).map_err(|e| format!("cannot read snapshots dir: {}", e))?;
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let name = entry.file_name().to_string_lossy().to_string();
            let meta_path = entry.path().join(".snapshot.yaml");
            let created = if meta_path.exists() {
                std::fs::read_to_string(&meta_path)
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
            } else {
                "unknown".to_string()
            };
            snapshots.push((name, created));
        }
    }
    snapshots.sort_by(|a, b| a.0.cmp(&b.0));

    if json {
        let items: Vec<serde_json::Value> = snapshots
            .iter()
            .map(|(n, c)| serde_json::json!({"name": n, "created_at": c}))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&items).map_err(|e| format!("JSON error: {}", e))?
        );
    } else if snapshots.is_empty() {
        println!("No snapshots.");
    } else {
        for (name, created) in &snapshots {
            println!("  {} ({})", name, created);
        }
    }
    Ok(())
}


pub(crate) fn cmd_snapshot_restore(name: &str, state_dir: &Path, yes: bool) -> Result<(), String> {
    let snap_dir = snapshots_dir(state_dir).join(name);
    if !snap_dir.exists() {
        return Err(format!("snapshot '{}' does not exist", name));
    }
    if !yes {
        return Err("Restore will overwrite current state. Use --yes to confirm.".to_string());
    }

    // Remove current state (except snapshots dir)
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {}", e))?;
    for entry in entries.flatten() {
        let name_os = entry.file_name();
        if name_os.to_string_lossy() == "snapshots" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)
                .map_err(|e| format!("cannot remove {}: {}", path.display(), e))?;
        } else {
            std::fs::remove_file(&path)
                .map_err(|e| format!("cannot remove {}: {}", path.display(), e))?;
        }
    }

    // Copy snapshot back (excluding .snapshot.yaml metadata)
    copy_dir_recursive(&snap_dir, state_dir, ".snapshot.yaml")?;

    println!("Restored snapshot: {}", name);
    Ok(())
}


pub(crate) fn cmd_snapshot_delete(name: &str, state_dir: &Path) -> Result<(), String> {
    let snap_dir = snapshots_dir(state_dir).join(name);
    if !snap_dir.exists() {
        return Err(format!("snapshot '{}' does not exist", name));
    }
    std::fs::remove_dir_all(&snap_dir).map_err(|e| format!("cannot delete snapshot: {}", e))?;
    println!("Deleted snapshot: {}", name);
    Ok(())
}


// FJ-260: forjar snapshot — named state checkpoints

pub(crate) fn snapshots_dir(state_dir: &Path) -> PathBuf {
    state_dir.join("snapshots")
}


/// Recursively copy a directory, skipping entries whose name matches `skip`.
pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path, skip: &str) -> Result<(), String> {
    let entries =
        std::fs::read_dir(src).map_err(|e| format!("cannot read {}: {}", src.display(), e))?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy() == skip {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        if src_path.is_dir() {
            std::fs::create_dir_all(&dst_path)
                .map_err(|e| format!("cannot create {}: {}", dst_path.display(), e))?;
            copy_dir_recursive(&src_path, &dst_path, "")?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "cannot copy {} → {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

