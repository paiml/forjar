//! FJ-1434: Automated preservation checking.
//!
//! Verify pairwise resource preservation: applying resource A
//! doesn't invalidate resource B's postcondition.
//! Based on Hanappi & Hummer OOPSLA 2016.

use super::helpers::*;
use std::path::Path;

/// A preservation pair check result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PreservationPair {
    pub resource_a: String,
    pub resource_b: String,
    pub preserved: bool,
    pub reason: String,
}

/// Preservation check report.
#[derive(Debug, serde::Serialize)]
pub struct PreservationReport {
    pub pairs_checked: usize,
    pub preserved: usize,
    pub conflicts: usize,
    pub results: Vec<PreservationPair>,
}

/// Check pairwise resource preservation.
pub fn cmd_preservation(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let ids: Vec<String> = config.resources.keys().cloned().collect();
    let mut results = Vec::new();

    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let pair = check_pair(&config, &ids[i], &ids[j]);
            results.push(pair);
        }
    }

    let pairs_checked = results.len();
    let preserved = results.iter().filter(|p| p.preserved).count();
    let conflicts = pairs_checked - preserved;

    let report = PreservationReport {
        pairs_checked,
        preserved,
        conflicts,
        results,
    };

    if json {
        let out =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        print_preservation_report(&report);
    }

    if conflicts > 0 {
        Err(format!("{conflicts} preservation conflict(s) detected"))
    } else {
        Ok(())
    }
}

fn check_pair(
    config: &crate::core::types::ForjarConfig,
    id_a: &str,
    id_b: &str,
) -> PreservationPair {
    let res_a = &config.resources[id_a];
    let res_b = &config.resources[id_b];

    // Check for file path conflicts
    if let (Some(path_a), Some(path_b)) = (&res_a.path, &res_b.path) {
        if path_a == path_b {
            return PreservationPair {
                resource_a: id_a.to_string(),
                resource_b: id_b.to_string(),
                preserved: false,
                reason: format!("both write to path: {path_a}"),
            };
        }
    }

    // Check for package conflicts (same package, different versions)
    if has_package_conflict(res_a, res_b) {
        return PreservationPair {
            resource_a: id_a.to_string(),
            resource_b: id_b.to_string(),
            preserved: false,
            reason: "overlapping package lists".to_string(),
        };
    }

    // Check for service name conflicts
    if let (Some(name_a), Some(name_b)) = (&res_a.name, &res_b.name) {
        if name_a == name_b && res_a.resource_type == res_b.resource_type {
            return PreservationPair {
                resource_a: id_a.to_string(),
                resource_b: id_b.to_string(),
                preserved: false,
                reason: format!("same service name: {name_a}"),
            };
        }
    }

    PreservationPair {
        resource_a: id_a.to_string(),
        resource_b: id_b.to_string(),
        preserved: true,
        reason: "no conflicts detected".to_string(),
    }
}

fn has_package_conflict(
    a: &crate::core::types::Resource,
    b: &crate::core::types::Resource,
) -> bool {
    if a.resource_type != crate::core::types::ResourceType::Package
        || b.resource_type != crate::core::types::ResourceType::Package
    {
        return false;
    }
    a.packages.iter().any(|p| b.packages.contains(p))
}

fn print_preservation_report(report: &PreservationReport) {
    println!("Preservation Check Report");
    println!("=========================");
    println!(
        "Pairs: {} | Preserved: {} | Conflicts: {}",
        report.pairs_checked, report.preserved, report.conflicts
    );
    println!();
    for p in &report.results {
        let icon = if p.preserved { "OK " } else { "ERR" };
        println!("[{icon}] {} <-> {}: {}", p.resource_a, p.resource_b, p.reason);
    }
}
