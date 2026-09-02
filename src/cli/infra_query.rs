//! FJ-1430: forjar query — composable infrastructure search.
//!
//! Semantic and structured queries over fleet state.
//! Filter by machine glob, resource type, status, staleness, regex on IDs.

use super::helpers::*;
use std::path::Path;

/// A query match result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryMatch {
    pub resource_id: String,
    pub resource_type: String,
    pub machine: Vec<String>,
    pub tags: Vec<String>,
    pub status: String,
}

/// Query result report.
#[derive(Debug, serde::Serialize)]
pub struct QueryReport {
    pub query: String,
    pub matches: Vec<QueryMatch>,
    pub total: usize,
}

/// Query filter options.
pub struct QueryFilter {
    pub pattern: Option<String>,
    pub resource_type: Option<String>,
    pub machine: Option<String>,
    pub tag: Option<String>,
}

/// Run an infrastructure query against config.
pub fn cmd_query(
    file: &Path,
    state_dir: &Path,
    filter: &QueryFilter,
    details: bool,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let matches = execute_query(&config, state_dir, filter);
    let query_desc = describe_filter(filter);

    let report = QueryReport {
        query: query_desc,
        total: matches.len(),
        matches,
    };

    if json {
        let out = serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        print_query_report(&report, details);
    }
    Ok(())
}

fn execute_query(
    config: &crate::core::types::ForjarConfig,
    state_dir: &Path,
    filter: &QueryFilter,
) -> Vec<QueryMatch> {
    let mut matches = Vec::new();
    for (id, res) in &config.resources {
        if !matches_filter(id, res, filter) {
            continue;
        }
        let status = compute_status(id, res, state_dir);
        matches.push(QueryMatch {
            resource_id: id.clone(),
            // The DOCUMENT's spelling, via `Display` — `github_release`, not
            // the `Debug` rendering `GithubRelease`, which is a value serde
            // rejects with `unknown variant` (paiml/forjar#366). An id a report
            // prints should be one the reader can put back into a config or a
            // filter.
            resource_type: res.resource_type.to_string(),
            machine: res.machine.iter().map(|s| s.to_owned()).collect(),
            tags: res.tags.clone(),
            status,
        });
    }
    matches
}

/// The free-text pattern hits either the resource id (as written) or its type
/// name in the document's own spelling.
fn matches_pattern(id: &str, res: &crate::core::types::Resource, pattern: &str) -> bool {
    id.contains(pattern) || res.resource_type.to_string().contains(pattern)
}

/// Type filter: case-insensitive substring of the type name.
///
/// The name is `Display`, so `--type github_release` — the spelling the config
/// itself declares — matches. It used to be the lowercased `Debug` spelling,
/// where the substring `github` matched and the full `github_release` did not
/// (paiml/forjar#366). Moves with the report above: filtering on one spelling
/// while printing another is how `--type <the value we just printed>` returns
/// nothing.
fn matches_resource_type(res: &crate::core::types::Resource, wanted: &str) -> bool {
    res.resource_type
        .to_string()
        .contains(&wanted.to_lowercase())
}

/// Machine filter: substring of any machine the resource targets.
fn matches_machine(res: &crate::core::types::Resource, wanted: &str) -> bool {
    res.machine.iter().any(|mv| mv.contains(wanted))
}

/// Tag filter: substring of any tag on the resource.
fn matches_tag(res: &crate::core::types::Resource, wanted: &str) -> bool {
    res.tags.iter().any(|tag| tag.contains(wanted))
}

/// Every criterion the filter sets has to match; criteria left unset are
/// ignored rather than treated as "matches nothing".
fn matches_filter(id: &str, res: &crate::core::types::Resource, filter: &QueryFilter) -> bool {
    filter
        .pattern
        .as_deref()
        .is_none_or(|pat| matches_pattern(id, res, pat))
        && filter
            .resource_type
            .as_deref()
            .is_none_or(|rt| matches_resource_type(res, rt))
        && filter
            .machine
            .as_deref()
            .is_none_or(|m| matches_machine(res, m))
        && filter.tag.as_deref().is_none_or(|t| matches_tag(res, t))
}

fn compute_status(id: &str, res: &crate::core::types::Resource, state_dir: &Path) -> String {
    for m in res.machine.iter() {
        let lock = state_dir.join(m).join("state.lock.yaml");
        if lock.exists() {
            if let Ok(content) = std::fs::read_to_string(&lock) {
                if content.contains(id) {
                    return "converged".to_string();
                }
            }
        }
    }
    "pending".to_string()
}

fn describe_filter(filter: &QueryFilter) -> String {
    let mut parts = Vec::new();
    if let Some(ref p) = filter.pattern {
        parts.push(format!("pattern={p}"));
    }
    if let Some(ref r) = filter.resource_type {
        parts.push(format!("type={r}"));
    }
    if let Some(ref m) = filter.machine {
        parts.push(format!("machine={m}"));
    }
    if let Some(ref t) = filter.tag {
        parts.push(format!("tag={t}"));
    }
    if parts.is_empty() {
        "*".to_string()
    } else {
        parts.join(", ")
    }
}

fn print_query_report(report: &QueryReport, details: bool) {
    println!("Query: {}", report.query);
    println!("Matches: {}", report.total);
    println!();
    for m in &report.matches {
        if details {
            println!("  {} ({})", m.resource_id, m.resource_type);
            println!("    machines: {}", m.machine.join(", "));
            println!("    tags: {}", m.tags.join(", "));
            println!("    status: {}", m.status);
        } else {
            println!("  {} [{}] {}", m.resource_id, m.resource_type, m.status);
        }
    }
}
