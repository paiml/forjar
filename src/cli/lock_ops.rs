//! Lock operations.

use super::helpers::*;
use crate::core::{state, types};
use std::path::Path;

// ── FJ-395: lock compact ──

/// Check a single machine directory for compactable events, optionally applying.
fn compact_machine_events(path: &Path, yes: bool) -> Result<Option<(String, usize)>, String> {
    let m_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let log_path = path.join("events.jsonl");
    if !log_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&log_path).unwrap_or_default();
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= 1 {
        return Ok(None);
    }
    let removed = lines.len() - 1;
    if yes {
        let last = lines.last().unwrap_or(&"");
        std::fs::write(&log_path, format!("{last}\n")).map_err(|e| e.to_string())?;
    }
    Ok(Some((m_name, removed)))
}

pub(crate) fn cmd_lock_compact(state_dir: &Path, yes: bool, json: bool) -> Result<(), String> {
    if !state_dir.exists() {
        return Err(format!(
            "state directory not found: {}",
            state_dir.display()
        ));
    }

    let mut total_removed = 0usize;
    let mut machines_compacted = Vec::new();

    let entries = std::fs::read_dir(state_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some((m_name, removed)) = compact_machine_events(&path, yes)? {
                total_removed += removed;
                machines_compacted.push((m_name, removed));
            }
        }
    }

    if json {
        let entries: Vec<String> = machines_compacted
            .iter()
            .map(|(m, n)| format!("{{\"machine\":\"{m}\",\"removed\":{n}}}"))
            .collect();
        println!(
            "{{\"compacted\":{},\"total_removed\":{},\"dry_run\":{},\"machines\":[{}]}}",
            yes,
            total_removed,
            !yes,
            entries.join(",")
        );
    } else if machines_compacted.is_empty() {
        println!("Nothing to compact — event logs are already minimal.");
    } else if yes {
        println!("{} Compacted {} event(s):", green("✓"), total_removed);
        for (m, n) in &machines_compacted {
            println!("  {m} — {n} old event(s) removed");
        }
    } else {
        println!("Dry run — would compact {total_removed} event(s):");
        for (m, n) in &machines_compacted {
            println!("  {m} — {n} old event(s)");
        }
        println!("\nRun with {} to compact.", bold("--yes"));
    }
    Ok(())
}

/// Check one machine's per-resource hash *fields*, returning (verified_count, issues).
///
/// This is a self-report: the hashes live inside the lock body, so a tamperer who
/// rewrites the body controls them. It is a cheap sanity check on top of — never a
/// substitute for — the BLAKE3 sidecar comparison in [`sidecar_failures`].
fn verify_machine_lock(state_dir: &Path, m_name: &str) -> (usize, Vec<String>) {
    match state::load_lock(state_dir, m_name) {
        Ok(Some(lock)) => {
            let issues: Vec<String> = lock
                .resources
                .iter()
                .filter(|(_, rl)| rl.hash.is_empty())
                .map(|(name, _)| format!("{m_name}/{name} — empty hash"))
                .collect();
            let verified = if issues.is_empty() {
                lock.resources.len()
            } else {
                0
            };
            (verified, issues)
        }
        Ok(None) => (0, Vec::new()),
        Err(e) => (0, vec![format!("{m_name} — corrupt: {e}")]),
    }
}

// ── FJ-405: lock verify ──

pub(crate) fn cmd_lock_verify(state_dir: &Path, json: bool) -> Result<(), String> {
    if !state_dir.exists() {
        return Err(format!(
            "state directory not found: {}",
            state_dir.display()
        ));
    }

    // CB-2010: recompute BLAKE3 over every lock body and compare it against the
    // `.b3` sidecar. `lock-verify` used to inspect only `rl.hash.is_empty()`, so a
    // tampered body, a zeroed sidecar, a deleted sidecar and a deleted lock file all
    // printed "✓ Lock integrity verified". That is a claim about a measurement that
    // was never taken.
    let mut issues: Vec<String> = sidecar_failures(state_dir)
        .into_iter()
        .map(|(machine, reason)| format!("{machine} — {reason}"))
        .collect();

    let mut verified = 0usize;
    for m_name in discover_machines(state_dir) {
        let (v, machine_issues) = verify_machine_lock(state_dir, &m_name);
        verified += v;
        issues.extend(machine_issues);
    }

    let corrupt = issues.len();
    if json {
        println!(
            "{{\"verified\":{},\"corrupt\":{},\"ok\":{}}}",
            verified,
            corrupt,
            corrupt == 0
        );
    } else if corrupt == 0 {
        println!(
            "{} Lock integrity verified — {} resource(s) checked",
            green("✓"),
            verified
        );
    } else {
        println!(
            "{} Lock integrity check found {} issue(s):",
            red("✗"),
            corrupt
        );
        for issue in &issues {
            println!("  {} {}", red("•"), issue);
        }
    }
    if corrupt == 0 {
        Ok(())
    } else {
        Err(format!("{corrupt} lock integrity issue(s)"))
    }
}

/// Collect all lock resources from state dir, optionally filtered by machine.
fn collect_lock_resources(
    state_dir: &Path,
    machine: Option<&str>,
) -> Result<Vec<(String, String, String, String)>, String> {
    let mut all_resources = Vec::new();
    if !state_dir.exists() {
        return Ok(all_resources);
    }
    let entries = std::fs::read_dir(state_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let m_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if let Some(filter) = machine {
            if m_name != filter {
                continue;
            }
        }
        if let Ok(Some(lock)) = state::load_lock(state_dir, &m_name) {
            for (name, rl) in &lock.resources {
                all_resources.push((
                    m_name.clone(),
                    name.clone(),
                    format!("{:?}", rl.status),
                    rl.hash.clone(),
                ));
            }
        }
    }
    Ok(all_resources)
}

// ── FJ-415: lock export ──

pub(crate) fn cmd_lock_export(
    state_dir: &Path,
    fmt: &str,
    machine: Option<&str>,
) -> Result<(), String> {
    let all_resources = collect_lock_resources(state_dir, machine)?;

    match fmt {
        "json" => {
            let entries: Vec<String> = all_resources
                .iter()
                .map(|(m, n, s, h)| {
                    format!(
                        "{{\"machine\":\"{m}\",\"resource\":\"{n}\",\"status\":\"{s}\",\"hash\":\"{h}\"}}"
                    )
                })
                .collect();
            println!("[{}]", entries.join(","));
        }
        "csv" => {
            println!("machine,resource,status,hash");
            for (m, n, s, h) in &all_resources {
                println!("{m},{n},{s},{h}");
            }
        }
        "yaml" => {
            println!("resources:");
            for (m, n, s, h) in &all_resources {
                println!("  - machine: {m}");
                println!("    resource: {n}");
                println!("    status: {s}");
                println!("    hash: {h}");
            }
        }
        _ => {
            return Err(format!("unknown format '{fmt}'. Use json, csv, or yaml"));
        }
    }
    Ok(())
}

/// Collect orphaned lock entries not present in config resources.
fn collect_orphaned_resources(
    state_dir: &Path,
    config_resources: &std::collections::HashSet<&str>,
) -> Result<Vec<(String, String)>, String> {
    let mut orphaned = Vec::new();
    if !state_dir.exists() {
        return Ok(orphaned);
    }
    let entries = std::fs::read_dir(state_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let m_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if m_name.starts_with('.') {
            continue;
        }
        if let Ok(Some(lock)) = state::load_lock(state_dir, &m_name) {
            for name in lock.resources.keys() {
                if !config_resources.contains(name.as_str()) {
                    orphaned.push((m_name.clone(), name.clone()));
                }
            }
        }
    }
    Ok(orphaned)
}

// ── FJ-425: lock gc ──

pub(crate) fn cmd_lock_gc(
    file: &Path,
    state_dir: &Path,
    yes: bool,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let config_resources: std::collections::HashSet<&str> =
        config.resources.keys().map(|k| k.as_str()).collect();
    let orphaned = collect_orphaned_resources(state_dir, &config_resources)?;

    if json {
        let entries: Vec<String> = orphaned
            .iter()
            .map(|(m, r)| format!("{{\"machine\":\"{m}\",\"resource\":\"{r}\"}}"))
            .collect();
        println!(
            "{{\"orphaned\":{},\"dry_run\":{},\"entries\":[{}]}}",
            orphaned.len(),
            !yes,
            entries.join(",")
        );
    } else if orphaned.is_empty() {
        println!("{} No orphaned lock entries found", green("✓"));
    } else if yes {
        println!(
            "{} Would remove {} orphaned lock entries (removal not yet implemented — use lock-prune)",
            yellow("⚠"),
            orphaned.len()
        );
        for (m, r) in &orphaned {
            println!("  {} {}/{}", red("×"), m, r);
        }
    } else {
        println!("Dry run — found {} orphaned lock entries:", orphaned.len());
        for (m, r) in &orphaned {
            println!("  {} {}/{}", yellow("●"), m, r);
        }
        println!("\nRun with {} to remove.", bold("--yes"));
    }
    Ok(())
}

// ── FJ-435: lock diff ──

/// Load all lock resources from a state directory, keyed by machine name.
fn load_all_lock_resources(
    dir: &Path,
) -> Result<
    std::collections::HashMap<String, indexmap::IndexMap<String, types::ResourceLock>>,
    String,
> {
    let mut result = std::collections::HashMap::new();
    if !dir.exists() {
        return Ok(result);
    }
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let m_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if m_name.starts_with('.') {
            continue;
        }
        if let Ok(Some(lock)) = state::load_lock(dir, &m_name) {
            result.insert(m_name, lock.resources);
        }
    }
    Ok(result)
}

/// Compute diff entries between two lock resource maps.
fn compute_lock_diffs(
    left: &std::collections::HashMap<String, indexmap::IndexMap<String, types::ResourceLock>>,
    right: &std::collections::HashMap<String, indexmap::IndexMap<String, types::ResourceLock>>,
) -> Vec<String> {
    let mut diffs = Vec::new();
    for (machine, resources) in right {
        for (rname, rl) in resources {
            let left_hash = left
                .get(machine)
                .and_then(|r| r.get(rname))
                .map(|l| l.hash.as_str());
            match left_hash {
                None => diffs.push(format!("+ {machine}/{rname} (added)")),
                Some(h) if h != rl.hash => {
                    diffs.push(format!("~ {machine}/{rname} (changed)"));
                }
                _ => {}
            }
        }
    }
    for (machine, resources) in left {
        for (rname, _) in resources {
            if !right.get(machine).is_some_and(|r| r.contains_key(rname)) {
                diffs.push(format!("- {machine}/{rname} (removed)"));
            }
        }
    }
    diffs
}

pub(crate) fn cmd_lock_diff(from: &Path, to: &Path, json: bool) -> Result<(), String> {
    let left = load_all_lock_resources(from)?;
    let right = load_all_lock_resources(to)?;
    let diffs = compute_lock_diffs(&left, &right);

    if json {
        let items: Vec<String> = diffs.iter().map(|d| format!("\"{d}\"")).collect();
        println!(
            "{{\"diffs\":[{}],\"count\":{}}}",
            items.join(","),
            diffs.len()
        );
    } else if diffs.is_empty() {
        println!("{} No differences between lock files.", green("✓"));
    } else {
        println!("{} difference(s):", diffs.len());
        for d in &diffs {
            println!("  {d}");
        }
    }
    Ok(())
}
