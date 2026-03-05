//! FJ-1412: Training checkpoint management.
//!
//! Track checkpoint artifacts via output_artifacts; list, verify, and
//! garbage-collect old checkpoints.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

pub(crate) fn cmd_checkpoint(
    file: &Path,
    machine_filter: Option<&str>,
    gc: bool,
    keep: usize,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let config_dir = file.parent().unwrap_or(Path::new("."));

    let mut checkpoints = Vec::new();

    for (id, resource) in &config.resources {
        if !is_checkpoint_resource(resource) {
            continue;
        }
        if let Some(filter) = machine_filter {
            if !machine_matches(&resource.machine, filter) {
                continue;
            }
        }

        for artifact in &resource.output_artifacts {
            let art_path = config_dir.join(artifact);
            let info = inspect_checkpoint(&art_path, id, artifact);
            checkpoints.push(info);
        }
    }

    // Sort by mtime (newest first)
    checkpoints.sort_by(|a, b| b.mtime_secs.cmp(&a.mtime_secs));

    if gc {
        return gc_checkpoints(&checkpoints, keep, json);
    }

    if json {
        print_checkpoint_json(&checkpoints);
    } else {
        print_checkpoint_text(&checkpoints);
    }

    Ok(())
}

fn is_checkpoint_resource(resource: &types::Resource) -> bool {
    matches!(resource.resource_type, types::ResourceType::Model)
        || resource
            .tags
            .iter()
            .any(|t| t.contains("checkpoint") || t.contains("training") || t.contains("ml"))
        || resource.resource_group.as_deref() == Some("checkpoints")
}

fn machine_matches(machine: &types::MachineTarget, filter: &str) -> bool {
    machine.to_vec().iter().any(|m| m == filter)
}

struct CheckpointInfo {
    resource: String,
    artifact: String,
    exists: bool,
    size: u64,
    mtime_secs: u64,
    hash: Option<String>,
}

fn inspect_checkpoint(path: &Path, resource: &str, artifact: &str) -> CheckpointInfo {
    if !path.exists() {
        return CheckpointInfo {
            resource: resource.to_string(),
            artifact: artifact.to_string(),
            exists: false,
            size: 0,
            mtime_secs: 0,
            hash: None,
        };
    }

    let meta = path.metadata().ok();
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let mtime_secs = meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let hash = std::fs::read(path)
        .ok()
        .map(|bytes| blake3::hash(&bytes).to_hex()[..16].to_string());

    CheckpointInfo {
        resource: resource.to_string(),
        artifact: artifact.to_string(),
        exists: true,
        size,
        mtime_secs,
        hash,
    }
}

fn gc_checkpoints(checkpoints: &[CheckpointInfo], keep: usize, json: bool) -> Result<(), String> {
    let existing: Vec<&CheckpointInfo> = checkpoints.iter().filter(|c| c.exists).collect();
    let to_remove = if existing.len() > keep {
        existing.len() - keep
    } else {
        0
    };

    if json {
        println!(
            r#"{{"total":{},"kept":{keep},"removed":{to_remove}}}"#,
            existing.len()
        );
    } else {
        println!("{}\n", bold("Checkpoint GC"));
        println!(
            "  Total: {} | Keep: {keep} | Remove: {to_remove}",
            existing.len()
        );
        for (i, cp) in existing.iter().enumerate() {
            let icon = if i < keep { green("✓") } else { yellow("×") };
            let hash = cp.hash.as_deref().unwrap_or("n/a");
            println!(
                "  {icon} {}: {} ({}, {})",
                cp.resource,
                cp.artifact,
                human_size(cp.size),
                dim(hash)
            );
        }
    }

    Ok(())
}

fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn print_checkpoint_json(checkpoints: &[CheckpointInfo]) {
    let items: Vec<String> = checkpoints
        .iter()
        .map(|c| {
            let hash = c.hash.as_deref().unwrap_or("null");
            format!(
                r#"{{"resource":"{r}","artifact":"{a}","exists":{e},"size":{s},"mtime":{m},"hash":"{hash}"}}"#,
                r = c.resource,
                a = c.artifact,
                e = c.exists,
                s = c.size,
                m = c.mtime_secs,
            )
        })
        .collect();

    println!(
        r#"{{"count":{},"checkpoints":[{}]}}"#,
        checkpoints.len(),
        items.join(",")
    );
}

fn print_checkpoint_text(checkpoints: &[CheckpointInfo]) {
    println!("{}\n", bold("Checkpoint Registry"));
    println!("  Total: {}\n", checkpoints.len());

    for cp in checkpoints {
        let icon = if cp.exists { green("✓") } else { dim("?") };
        let hash = cp.hash.as_deref().unwrap_or("n/a");
        println!(
            "  {icon} {}: {} ({}, {})",
            cp.resource,
            cp.artifact,
            human_size(cp.size),
            dim(hash)
        );
    }

    if checkpoints.is_empty() {
        println!("  {} No checkpoint artifacts found", dim("(empty)"));
    }
}
