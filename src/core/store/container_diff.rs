//! IMG-001 (forjar#756): turn a `docker diff` into the files of a diff layer.
//!
//! The layer sits on top of the exported base, so it must carry all three
//! kinds of change: Added (A) and Changed (C) files are copied out, Deleted (D)
//! paths become OCI whiteouts (`.wh.<name>`), and a directory that was deleted
//! and rebuilt becomes an opaque directory (`.wh..wh..opq`).

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// One line of `docker diff`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiffKind {
    Added,
    Changed,
    Deleted,
}

/// What to do with a diff: copy paths out, whiteout paths, opaque dirs.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DiffPlan {
    pub copy: Vec<String>,
    pub whiteouts: Vec<String>,
    pub opaque_dirs: Vec<String>,
}

/// Parse `docker diff` output (`A /p`, `C /p`, `D /p`); other lines are ignored.
pub(crate) fn parse_diff(text: &str) -> Vec<(DiffKind, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            let kind = match line.chars().next()? {
                'A' => DiffKind::Added,
                'C' => DiffKind::Changed,
                'D' => DiffKind::Deleted,
                _ => return None,
            };
            let path = line[1..].trim();
            (path.starts_with('/') && path.len() > 1).then(|| (kind, path.to_string()))
        })
        .collect()
}

/// Classify the entries. A deleted path that also has an added/changed path at
/// or beneath it was deleted and rebuilt: it is an opaque dir, not a whiteout.
pub(crate) fn plan_diff(entries: &[(DiffKind, String)]) -> DiffPlan {
    let live: Vec<&str> = entries
        .iter()
        .filter(|(k, _)| *k != DiffKind::Deleted)
        .map(|(_, p)| p.as_str())
        .collect();
    let mut plan = DiffPlan::default();
    let mut seen = BTreeSet::new();
    for (kind, path) in entries {
        if !seen.insert((*kind == DiffKind::Deleted, path.clone())) {
            continue;
        }
        if *kind != DiffKind::Deleted {
            plan.copy.push(path.clone());
            continue;
        }
        let prefix = format!("{path}/");
        if live.iter().any(|p| *p == path || p.starts_with(&prefix)) {
            plan.opaque_dirs.push(path.clone());
        } else {
            plan.whiteouts.push(path.clone());
        }
    }
    plan
}

/// Write the whiteout / opaque marker files of `plan` under `extract_dir`.
pub(crate) fn write_markers(plan: &DiffPlan, extract_dir: &Path) -> Result<(), String> {
    for path in &plan.whiteouts {
        let rel = Path::new(path.trim_start_matches('/'));
        let name = rel.file_name().ok_or_else(|| format!("bad path {path}"))?;
        let parent = extract_dir.join(rel.parent().unwrap_or(Path::new("")));
        std::fs::create_dir_all(&parent).map_err(|e| format!("whiteout dir: {e}"))?;
        let marker = format!(".wh.{}", name.to_string_lossy());
        std::fs::write(parent.join(marker), b"").map_err(|e| format!("whiteout {path}: {e}"))?;
    }
    for dir in &plan.opaque_dirs {
        let d = extract_dir.join(dir.trim_start_matches('/'));
        std::fs::create_dir_all(&d).map_err(|e| format!("opaque dir: {e}"))?;
        std::fs::write(d.join(".wh..wh..opq"), b"").map_err(|e| format!("opaque {dir}: {e}"))?;
    }
    Ok(())
}

/// Is `path` a directory inside the container? (Directories are never copied
/// whole: a changed dir would drag its unchanged base contents into the layer.)
fn is_dir_in_container(runtime: &str, container: &str, path: &str) -> bool {
    Command::new(runtime)
        .args(["exec", container, "test", "-d", path])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Extract the container's A/C files and D whiteouts into `extract_dir`.
/// Returns the number of changes carried (files copied + whiteouts + opaque dirs).
pub(crate) fn extract_changes(
    runtime: &str,
    container_name: &str,
    extract_dir: &Path,
) -> Result<usize, String> {
    let diff_output = Command::new(runtime)
        .args(["diff", container_name])
        .output()
        .map_err(|e| format!("docker diff: {e}"))?;
    if !diff_output.status.success() {
        let stderr = String::from_utf8_lossy(&diff_output.stderr);
        return Err(format!("docker diff failed: {}", stderr.trim()));
    }
    let entries = parse_diff(&String::from_utf8_lossy(&diff_output.stdout));
    let plan = plan_diff(&entries);

    let mut count = 0;
    for path in &plan.copy {
        if is_dir_in_container(runtime, container_name, path) {
            continue;
        }
        let local_path = extract_dir.join(path.trim_start_matches('/'));
        if let Some(parent) = local_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let src = format!("{container_name}:{path}");
        // Best-effort: skip files that can't be copied (sockets, etc.)
        if let Ok(o) = Command::new(runtime)
            .args(["cp", &src, &local_path.to_string_lossy()])
            .output()
        {
            if o.status.success() {
                count += 1;
            }
        }
    }
    write_markers(&plan, extract_dir)?;
    Ok(count + plan.whiteouts.len() + plan.opaque_dirs.len())
}
