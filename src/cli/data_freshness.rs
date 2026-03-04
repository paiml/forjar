//! FJ-1410: Data freshness monitoring.
//!
//! Detects stale data artifacts via BLAKE3 + mtime; alerts when outputs
//! exceed freshness SLA.

use super::helpers::*;
use std::path::Path;

pub(crate) fn cmd_data_freshness(
    file: &Path,
    state_dir: &Path,
    max_age_hours: Option<u64>,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let max_age_secs = max_age_hours.unwrap_or(24) * 3600;

    let mut entries = Vec::new();
    let now = std::time::SystemTime::now();

    // 1. Check output_artifacts freshness
    for (id, resource) in &config.resources {
        for artifact in &resource.output_artifacts {
            let artifact_path = file.parent().unwrap_or(Path::new(".")).join(artifact);
            let entry = check_artifact_freshness(&artifact_path, id, artifact, now, max_age_secs);
            entries.push(entry);
        }
    }

    // 2. Check store directory freshness
    let store_dir = file.parent().unwrap_or(Path::new(".")).join("store");
    if store_dir.exists() {
        check_dir_freshness(&store_dir, "store", now, max_age_secs, &mut entries);
    }

    // 3. Check state lock freshness
    let global_lock = state_dir.join("forjar.lock.yaml");
    if global_lock.exists() {
        let entry = check_artifact_freshness(&global_lock, "state", "forjar.lock.yaml", now, max_age_secs);
        entries.push(entry);
    }

    let stale_count = entries.iter().filter(|e| e.stale).count();
    let fresh_count = entries.iter().filter(|e| !e.stale && e.exists).count();
    let missing_count = entries.iter().filter(|e| !e.exists).count();

    if json {
        print_freshness_json(&entries, stale_count, fresh_count, missing_count, max_age_secs);
    } else {
        print_freshness_text(&entries, stale_count, fresh_count, missing_count, max_age_secs);
    }

    if stale_count > 0 {
        Err(format!("{stale_count} artifact(s) exceed freshness SLA"))
    } else {
        Ok(())
    }
}

struct FreshnessEntry {
    resource: String,
    artifact: String,
    exists: bool,
    stale: bool,
    age_secs: Option<u64>,
    hash: Option<String>,
}

fn check_artifact_freshness(
    path: &Path,
    resource: &str,
    artifact: &str,
    now: std::time::SystemTime,
    max_age_secs: u64,
) -> FreshnessEntry {
    if !path.exists() {
        return FreshnessEntry {
            resource: resource.to_string(),
            artifact: artifact.to_string(),
            exists: false,
            stale: false,
            age_secs: None,
            hash: None,
        };
    }

    let age_secs = path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|mtime| now.duration_since(mtime).ok())
        .map(|d| d.as_secs());

    let hash = std::fs::read(path)
        .ok()
        .map(|bytes| blake3::hash(&bytes).to_hex()[..16].to_string());

    let stale = age_secs.map(|a| a > max_age_secs).unwrap_or(false);

    FreshnessEntry {
        resource: resource.to_string(),
        artifact: artifact.to_string(),
        exists: true,
        stale,
        age_secs,
        hash,
    }
}

fn check_dir_freshness(
    dir: &Path,
    label: &str,
    now: std::time::SystemTime,
    max_age_secs: u64,
    entries: &mut Vec<FreshnessEntry>,
) {
    if let Ok(dir_entries) = std::fs::read_dir(dir) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let artifact = format!("{label}/{name}");
                entries.push(check_artifact_freshness(&path, label, &artifact, now, max_age_secs));
            }
        }
    }
}

fn format_age(secs: u64) -> String {
    if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

fn print_freshness_json(
    entries: &[FreshnessEntry],
    stale: usize,
    fresh: usize,
    missing: usize,
    max_age_secs: u64,
) {
    let items: Vec<String> = entries
        .iter()
        .map(|e| {
            let hash = e.hash.as_deref().unwrap_or("null");
            let age = e.age_secs.map(|a| a.to_string()).unwrap_or_else(|| "null".to_string());
            let status = if !e.exists {
                "missing"
            } else if e.stale {
                "stale"
            } else {
                "fresh"
            };
            format!(
                r#"{{"resource":"{r}","artifact":"{a}","status":"{status}","age_secs":{age},"hash":"{hash}"}}"#,
                r = e.resource,
                a = e.artifact,
            )
        })
        .collect();

    println!(
        r#"{{"sla_max_age_secs":{max_age_secs},"stale":{stale},"fresh":{fresh},"missing":{missing},"artifacts":[{}]}}"#,
        items.join(",")
    );
}

fn print_freshness_text(
    entries: &[FreshnessEntry],
    stale: usize,
    fresh: usize,
    missing: usize,
    max_age_secs: u64,
) {
    println!("{}\n", bold("Data Freshness Report"));
    println!("  SLA max age: {}", format_age(max_age_secs));
    println!("  Fresh: {fresh} | Stale: {stale} | Missing: {missing}\n");

    for e in entries {
        let icon = if !e.exists {
            dim("?")
        } else if e.stale {
            red("!")
        } else {
            green("✓")
        };
        let age_str = e.age_secs.map(format_age).unwrap_or_else(|| "n/a".to_string());
        let hash_str = e.hash.as_deref().unwrap_or("n/a");
        println!(
            "  {icon} {}: {} (age: {}, {})",
            e.resource,
            e.artifact,
            age_str,
            dim(hash_str)
        );
    }

    if stale > 0 {
        println!("\n  {} {stale} artifact(s) exceed freshness SLA", red("✗"));
    } else {
        println!("\n  {} All artifacts within freshness SLA", green("✓"));
    }
}
