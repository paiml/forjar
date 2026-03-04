//! FJ-1384: Stack extraction — split a config into sub-configs by tag/group/glob.

use super::helpers::*;
use super::helpers_state::simple_glob_match;
use crate::core::types;
use std::collections::HashSet;
use std::path::Path;

/// Extract resources matching tag, group, or glob pattern into a new config.
pub(crate) fn cmd_extract(
    file: &Path,
    tag_filter: Option<&str>,
    group_filter: Option<&str>,
    glob_filter: Option<&str>,
    output: Option<&Path>,
    json: bool,
) -> Result<(), String> {
    if tag_filter.is_none() && group_filter.is_none() && glob_filter.is_none() {
        return Err("at least one of --tags, --group, or --glob is required".to_string());
    }

    let config = parse_and_validate(file)?;
    let mut extracted = config.clone();

    // Filter resources by tag/group/glob
    extracted.resources.retain(|id, resource| {
        let tag_match = tag_filter
            .map(|tag| resource.tags.iter().any(|t| t == tag))
            .unwrap_or(true);
        let group_match = group_filter
            .map(|g| resource.resource_group.as_deref() == Some(g))
            .unwrap_or(true);
        let glob_match = glob_filter
            .map(|g| simple_glob_match(g, id))
            .unwrap_or(true);
        tag_match && group_match && glob_match
    });

    if extracted.resources.is_empty() {
        return Err("no resources match the given filters".to_string());
    }

    // Collect referenced machines from filtered resources
    let referenced_machines = collect_referenced_machines(&extracted);
    extracted.machines.retain(|k, _| referenced_machines.contains(k));

    // Filter moved entries to only those referencing extracted resources
    extracted.moved.retain(|m| {
        extracted.resources.contains_key(&m.to)
            || extracted.resources.contains_key(&m.from)
    });

    // Update name to reflect extraction
    let filter_desc = build_filter_desc(tag_filter, group_filter, glob_filter);
    extracted.name = format!("{} (extract: {})", extracted.name, filter_desc);

    // Serialize and output
    let count = extracted.resources.len();
    let machine_count = extracted.machines.len();

    if json {
        let out = serde_json::to_string_pretty(&extracted)
            .map_err(|e| format!("JSON error: {e}"))?;
        write_or_print(output, &out)?;
    } else {
        let out = serde_yaml_ng::to_string(&extracted)
            .map_err(|e| format!("YAML error: {e}"))?;
        write_or_print(output, &out)?;
    }

    eprintln!(
        "Extracted: {} resources, {} machines (filter: {})",
        count, machine_count, filter_desc
    );
    Ok(())
}

/// Collect all machine names referenced by resources.
fn collect_referenced_machines(config: &types::ForjarConfig) -> HashSet<String> {
    let mut machines = HashSet::new();
    for resource in config.resources.values() {
        for m in resource.machine.to_vec() {
            machines.insert(m);
        }
    }
    machines
}

/// Build a human-readable description of the active filters.
fn build_filter_desc(
    tag: Option<&str>,
    group: Option<&str>,
    glob: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    if let Some(t) = tag {
        parts.push(format!("tags={t}"));
    }
    if let Some(g) = group {
        parts.push(format!("group={g}"));
    }
    if let Some(g) = glob {
        parts.push(format!("glob={g}"));
    }
    parts.join(", ")
}

/// Write content to a file or stdout.
fn write_or_print(output: Option<&Path>, content: &str) -> Result<(), String> {
    match output {
        Some(path) => {
            std::fs::write(path, content)
                .map_err(|e| format!("write {}: {e}", path.display()))?;
            eprintln!("Written to {}", path.display());
        }
        None => print!("{content}"),
    }
    Ok(())
}
