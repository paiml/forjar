//! Presentation for `forjar lock` — verify / write / dry-run result blocks.
//!
//! Split out of `lock_core.rs` to keep it under the 500-line limit. These are
//! formatting only; the lock semantics stay in `lock_core`.
use crate::core::{state, types};

use std::path::Path;

/// Output verify results (JSON or text).
pub(super) fn output_verify_results(
    mismatches: &[String],
    total_machines: usize,
    total_resources: usize,
    json: bool,
) -> Result<(), String> {
    if json {
        let result = serde_json::json!({
            "verified": mismatches.is_empty(),
            "machines": total_machines,
            "resources": total_resources,
            "mismatches": mismatches,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| format!("JSON error: {e}"))?
        );
    } else if mismatches.is_empty() {
        println!(
            "Lock verified: {total_machines} machines, {total_resources} resources — all hashes match"
        );
    } else {
        println!("Lock verification FAILED:");
        for m in mismatches {
            println!("  - {m}");
        }
    }
    if !mismatches.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

/// Output lock generation results (JSON or text).
pub(super) fn output_lock_results(
    state_dir: &Path,
    config_name: &str,
    machine_resources: &indexmap::IndexMap<String, Vec<(String, &types::Resource)>>,
    total_machines: usize,
    total_resources: usize,
    json: bool,
) -> Result<(), String> {
    use crate::tripwire::eventlog::now_iso8601;
    let machine_results: Vec<(String, usize, usize, usize)> = machine_resources
        .iter()
        .map(|(name, resources)| (name.clone(), resources.len(), 0, 0))
        .collect();
    state::update_global_lock(state_dir, config_name, &machine_results)?;

    if json {
        let result = serde_json::json!({
            "locked": true,
            "machines": total_machines,
            "resources": total_resources,
            "state_dir": state_dir.display().to_string(),
            "generated_at": now_iso8601(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| format!("JSON error: {e}"))?
        );
    } else {
        println!(
            "Locked: {} machines, {} resources → {}",
            total_machines,
            total_resources,
            state_dir.display()
        );
    }
    Ok(())
}

/// Output dry-run results (no state written).
pub(super) fn output_dry_run_results(
    total_machines: usize,
    total_resources: usize,
    json: bool,
) -> Result<(), String> {
    if json {
        let result = serde_json::json!({
            "dry_run": true,
            "machines": total_machines,
            "resources": total_resources,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| format!("JSON error: {e}"))?
        );
    } else {
        println!(
            "Dry run: would lock {} machines, {} resources (no changes written)",
            total_machines, total_resources
        );
    }
    Ok(())
}

// FJ-256: forjar lock — generate lock file without applying
