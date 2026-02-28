//! Phase 96 — Transport Diagnostics & Recipe Governance: validate commands.

#![allow(dead_code)]

use crate::core::types;
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// FJ-1030: Recipe input completeness
// ============================================================================

/// Check for template variables like `{{inputs.X}}` in resource content fields
/// that don't have a corresponding key in the resource's `inputs` map.
/// Since forjar configs don't have a formal global inputs section, this checks
/// each resource's inline content for `{{inputs.*}}` references and validates
/// them against the resource's own `inputs` field.
pub(crate) fn cmd_validate_check_recipe_input_completeness(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_recipe_input_completeness_gaps(&config);

    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(name, var)| {
                format!(
                    "{{\"resource\":\"{}\",\"missing_input\":\"{}\"}}",
                    name, var
                )
            })
            .collect();
        println!("{{\"recipe_input_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All template input references are satisfied.");
    } else {
        for (name, var) in &warnings {
            println!(
                "warning: {} references {{{{inputs.{}}}}} but no such input is defined",
                name, var
            );
        }
    }
    Ok(())
}

/// Extract `{{inputs.X}}` references from a string and return the variable names.
pub(crate) fn extract_input_references(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let pattern = "{{inputs.";
    let mut search_from = 0;
    while let Some(start) = text[search_from..].find(pattern) {
        let abs_start = search_from + start + pattern.len();
        if let Some(end) = text[abs_start..].find("}}") {
            let var_name = &text[abs_start..abs_start + end];
            if !var_name.is_empty() && var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                refs.push(var_name.to_string());
            }
            search_from = abs_start + end + 2;
        } else {
            break;
        }
    }
    refs
}

/// Collect all fields that may contain template references for a resource.
fn collect_templatable_fields(resource: &types::Resource) -> Vec<String> {
    let mut fields = Vec::new();
    if let Some(ref c) = resource.content {
        fields.push(c.clone());
    }
    if let Some(ref p) = resource.path {
        fields.push(p.clone());
    }
    if let Some(ref s) = resource.source {
        fields.push(s.clone());
    }
    if let Some(ref t) = resource.target {
        fields.push(t.clone());
    }
    if let Some(ref cmd) = resource.command {
        fields.push(cmd.clone());
    }
    if let Some(ref w) = resource.when {
        fields.push(w.clone());
    }
    fields
}

/// Returns `(resource_name, missing_variable)` for each unresolved `{{inputs.X}}`
/// reference found in resource template fields.
pub(crate) fn find_recipe_input_completeness_gaps(
    config: &types::ForjarConfig,
) -> Vec<(String, String)> {
    let mut warnings = Vec::new();
    for (name, resource) in &config.resources {
        let defined_keys: std::collections::HashSet<&String> =
            resource.inputs.keys().collect();
        let fields = collect_templatable_fields(resource);
        for field in &fields {
            for var in extract_input_references(field) {
                if !defined_keys.contains(&var) {
                    warnings.push((name.clone(), var));
                }
            }
        }
    }
    warnings.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    warnings.dedup();
    warnings
}

// ============================================================================
// FJ-1033: Resource content hash consistency
// ============================================================================

/// Check for resources on different machines that have identical content.
/// This may indicate copy-paste issues or missed parameterization — the same
/// content deployed to multiple machines might need machine-specific tweaks.
pub(crate) fn cmd_validate_check_resource_cross_machine_content_duplicates(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_content_hash_duplicates(&config);

    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(names, machines, hash)| {
                let name_arr: Vec<String> =
                    names.iter().map(|n| format!("\"{}\"", n)).collect();
                let mach_arr: Vec<String> =
                    machines.iter().map(|m| format!("\"{}\"", m)).collect();
                format!(
                    "{{\"resources\":[{}],\"machines\":[{}],\"content_hash\":\"{}\"}}",
                    name_arr.join(","),
                    mach_arr.join(","),
                    hash
                )
            })
            .collect();
        println!(
            "{{\"content_hash_warnings\":[{}]}}",
            items.join(",")
        );
    } else if warnings.is_empty() {
        println!("No cross-machine content duplication detected.");
    } else {
        for (names, machines, _) in &warnings {
            println!(
                "warning: resources [{}] on machines [{}] have identical content",
                names.join(", "),
                machines.join(", ")
            );
        }
    }
    Ok(())
}

/// Simple FNV-1a-style hash for content deduplication (not cryptographic).
pub(crate) fn hash_content(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:016x}", h)
}

/// Returns groups of `(resource_names, machine_names, content_hash)` where
/// different resources targeting different machines share identical content.
pub(crate) fn find_content_hash_duplicates(
    config: &types::ForjarConfig,
) -> Vec<(Vec<String>, Vec<String>, String)> {
    // Group by content hash: hash -> Vec<(resource_name, machine_target)>
    let mut by_hash: HashMap<String, Vec<(String, Vec<String>)>> = HashMap::new();

    for (name, resource) in &config.resources {
        if let Some(ref c) = resource.content {
            if c.is_empty() {
                continue;
            }
            let h = hash_content(c);
            let machines = resource.machine.to_vec();
            by_hash.entry(h).or_default().push((name.clone(), machines));
        }
    }

    let mut warnings = Vec::new();
    for (hash, entries) in &by_hash {
        if entries.len() < 2 {
            continue;
        }
        // Collect all unique machine names across entries.
        let all_machines: std::collections::HashSet<&String> = entries
            .iter()
            .flat_map(|(_, ms)| ms.iter())
            .collect();
        // Only warn if the duplicate content spans more than one distinct machine.
        if all_machines.len() < 2 {
            continue;
        }
        let mut names: Vec<String> = entries.iter().map(|(n, _)| n.clone()).collect();
        names.sort();
        let mut machines: Vec<String> = all_machines.into_iter().cloned().collect();
        machines.sort();
        warnings.push((names, machines, hash.clone()));
    }
    warnings.sort_by(|a, b| a.0.cmp(&b.0));
    warnings
}

// ============================================================================
// FJ-1036: Resource machine affinity
// ============================================================================

/// Check that every resource's `machine` field references a machine that is
/// actually defined in the config's `machines` section. Resources targeting
/// undefined machines will fail at apply time.
pub(crate) fn cmd_validate_check_resource_machine_reference_validity(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_machine_affinity_violations(&config);

    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(name, machine)| {
                format!(
                    "{{\"resource\":\"{}\",\"undefined_machine\":\"{}\"}}",
                    name, machine
                )
            })
            .collect();
        println!("{{\"machine_affinity_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All resource machine references are valid.");
    } else {
        for (name, machine) in &warnings {
            println!(
                "warning: {} targets machine '{}' which is not defined in machines section",
                name, machine
            );
        }
    }
    Ok(())
}

/// Returns `(resource_name, undefined_machine)` for each resource referencing
/// a machine that does not exist in the config's `machines` map.
/// The sentinel value `localhost` is always considered valid.
fn find_machine_affinity_violations(
    config: &types::ForjarConfig,
) -> Vec<(String, String)> {
    let defined: std::collections::HashSet<&String> = config.machines.keys().collect();
    let mut warnings = Vec::new();

    for (name, resource) in &config.resources {
        let targets = resource.machine.to_vec();
        for target in &targets {
            if target == "localhost" {
                continue;
            }
            if !defined.contains(target) {
                warnings.push((name.clone(), target.clone()));
            }
        }
    }
    warnings.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    warnings
}

