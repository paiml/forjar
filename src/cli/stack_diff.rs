//! FJ-1389: Unified stack diff — resource, machine, and param comparison.
//!
//! `forjar stack-diff networking.yaml compute.yaml` shows all differences
//! between two configs: resources, machines, params, and outputs.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// A single diff entry with section and type.
struct DiffEntry {
    section: &'static str,
    key: String,
    kind: DiffKind,
    detail: Option<String>,
}

enum DiffKind {
    Added,
    Removed,
    Modified,
}

/// Compare two forjar configs and print unified diff.
pub(crate) fn cmd_stack_diff(file1: &Path, file2: &Path, json: bool) -> Result<(), String> {
    let config1 = parse_and_validate(file1)?;
    let config2 = parse_and_validate(file2)?;

    let mut diffs = Vec::new();

    diff_top_level(&config1, &config2, &mut diffs);
    diff_resources(&config1, &config2, &mut diffs);
    diff_machines(&config1, &config2, &mut diffs);
    diff_params(&config1, &config2, &mut diffs);
    diff_outputs(&config1, &config2, &mut diffs);

    if json {
        print_json(&diffs, file1, file2)?;
    } else {
        print_text(&diffs, file1, file2);
    }

    Ok(())
}

/// Compare the scalar/top-level fields of two configs.
///
/// WHY THIS DESTRUCTURES INSTEAD OF LISTING FIELDS. stack-diff compared
/// resources, machines, params and outputs — a hand-written list — and was
/// therefore silent about `name`, `version`, `policy` and `policies`. Two
/// configs differing in name AND failure policy reported "No differences
/// found." (ledger id stack-diff-reports-no-differences-for-differing-configs,
/// confirmed at 1.12.3, still live at 1.16.0).
///
/// A hand-written field list is the defect, not the missing entries: it goes
/// stale silently every time the struct grows. Destructuring `ForjarConfig`
/// exhaustively means a NEW FIELD IS A COMPILE ERROR here — the next person to
/// extend the config has to decide whether it is comparable, rather than
/// discovering years later that the diff never mentioned it.
fn diff_top_level(c1: &types::ForjarConfig, c2: &types::ForjarConfig, diffs: &mut Vec<DiffEntry>) {
    // Bind every field. `..` is deliberately NOT used.
    let types::ForjarConfig {
        version: v1,
        name: n1,
        description: d1,
        policy: p1,
        policies: pol1,
        // Handled by their own dedicated diff fns above, which report per-key
        // adds/removes/modifications rather than a whole-value comparison.
        // Collections with their own per-key diff fns above.
        params: _,
        machines: _,
        resources: _,
        outputs: _,
        // Compared as counts/shape below. The compiler surfaced these when the
        // destructuring was made exhaustive — stack-diff had been silent about
        // all eight, on top of name/version/policy.
        data: data1,
        checks: checks1,
        moved: moved1,
        secrets: secrets1,
        environments: envs1,
        dist: dist1,
        // Provenance of the merge, not a property of the stack: two configs
        // assembled from different include paths can describe the same stack,
        // and reporting the paths as a difference would be noise.
        includes: _,
        include_provenance: _,
    } = c1;
    let types::ForjarConfig {
        version: v2,
        name: n2,
        description: d2,
        policy: p2,
        policies: pol2,
        // Collections with their own per-key diff fns above.
        params: _,
        machines: _,
        resources: _,
        outputs: _,
        // Compared as counts/shape below. The compiler surfaced these when the
        // destructuring was made exhaustive — stack-diff had been silent about
        // all eight, on top of name/version/policy.
        data: data2,
        checks: checks2,
        moved: moved2,
        secrets: secrets2,
        environments: envs2,
        dist: dist2,
        // Provenance of the merge, not a property of the stack: two configs
        // assembled from different include paths can describe the same stack,
        // and reporting the paths as a difference would be noise.
        includes: _,
        include_provenance: _,
    } = c2;

    let mut note = |field: &str, a: String, b: String| {
        if a != b {
            diffs.push(DiffEntry {
                section: "config",
                key: field.to_string(),
                kind: DiffKind::Modified,
                detail: Some(format!("{a} -> {b}")),
            });
        }
    };

    note("version", v1.clone(), v2.clone());
    note("name", n1.clone(), n2.clone());
    note(
        "description",
        d1.clone().unwrap_or_else(|| "(none)".into()),
        d2.clone().unwrap_or_else(|| "(none)".into()),
    );
    note("policy", format!("{p1:?}"), format!("{p2:?}"));
    note(
        "policies",
        format!("{} rule(s)", pol1.len()),
        format!("{} rule(s)", pol2.len()),
    );
    note(
        "data",
        format!("{} entr(ies)", data1.len()),
        format!("{} entr(ies)", data2.len()),
    );
    note(
        "checks",
        format!("{} check(s)", checks1.len()),
        format!("{} check(s)", checks2.len()),
    );
    note(
        "moved",
        format!("{} block(s)", moved1.len()),
        format!("{} block(s)", moved2.len()),
    );
    // Provider and path only — never a secret's value. Where secrets come from
    // is a legitimate difference to report; what they contain is not something
    // a diff should print.
    note(
        "secrets.provider",
        secrets1.provider.clone().unwrap_or_else(|| "(none)".into()),
        secrets2.provider.clone().unwrap_or_else(|| "(none)".into()),
    );
    note(
        "secrets.path",
        secrets1.path.clone().unwrap_or_else(|| "(none)".into()),
        secrets2.path.clone().unwrap_or_else(|| "(none)".into()),
    );
    note(
        "environments",
        format!("{} env(s)", envs1.len()),
        format!("{} env(s)", envs2.len()),
    );
    note(
        "dist",
        format!("{}", dist1.is_some()),
        format!("{}", dist2.is_some()),
    );
}

fn diff_resources(c1: &types::ForjarConfig, c2: &types::ForjarConfig, diffs: &mut Vec<DiffEntry>) {
    let keys1: std::collections::HashSet<&String> = c1.resources.keys().collect();
    let keys2: std::collections::HashSet<&String> = c2.resources.keys().collect();

    for key in keys1.difference(&keys2) {
        diffs.push(DiffEntry {
            section: "resources",
            key: (*key).clone(),
            kind: DiffKind::Removed,
            detail: Some(format!("type: {}", c1.resources[*key].resource_type)),
        });
    }
    for key in keys2.difference(&keys1) {
        diffs.push(DiffEntry {
            section: "resources",
            key: (*key).clone(),
            kind: DiffKind::Added,
            detail: Some(format!("type: {}", c2.resources[*key].resource_type)),
        });
    }
    for key in keys1.intersection(&keys2) {
        let s1 = format!("{:?}", c1.resources[*key]);
        let s2 = format!("{:?}", c2.resources[*key]);
        if s1 != s2 {
            let detail = resource_field_diff(&c1.resources[*key], &c2.resources[*key]);
            diffs.push(DiffEntry {
                section: "resources",
                key: (*key).clone(),
                kind: DiffKind::Modified,
                detail: Some(detail),
            });
        }
    }
}

fn diff_machines(c1: &types::ForjarConfig, c2: &types::ForjarConfig, diffs: &mut Vec<DiffEntry>) {
    let keys1: std::collections::HashSet<&String> = c1.machines.keys().collect();
    let keys2: std::collections::HashSet<&String> = c2.machines.keys().collect();

    for key in keys1.difference(&keys2) {
        diffs.push(DiffEntry {
            section: "machines",
            key: (*key).clone(),
            kind: DiffKind::Removed,
            detail: None,
        });
    }
    for key in keys2.difference(&keys1) {
        diffs.push(DiffEntry {
            section: "machines",
            key: (*key).clone(),
            kind: DiffKind::Added,
            detail: Some(format!("addr: {}", c2.machines[*key].addr)),
        });
    }
    for key in keys1.intersection(&keys2) {
        let m1 = &c1.machines[*key];
        let m2 = &c2.machines[*key];
        if m1.addr != m2.addr || m1.arch != m2.arch {
            let mut changes = Vec::new();
            if m1.addr != m2.addr {
                changes.push(format!("addr: {} → {}", m1.addr, m2.addr));
            }
            if m1.arch != m2.arch {
                changes.push(format!("arch: {} → {}", m1.arch, m2.arch));
            }
            diffs.push(DiffEntry {
                section: "machines",
                key: (*key).clone(),
                kind: DiffKind::Modified,
                detail: Some(changes.join(", ")),
            });
        }
    }
}

fn diff_params(c1: &types::ForjarConfig, c2: &types::ForjarConfig, diffs: &mut Vec<DiffEntry>) {
    let keys1: std::collections::HashSet<&String> = c1.params.keys().collect();
    let keys2: std::collections::HashSet<&String> = c2.params.keys().collect();

    for key in keys1.difference(&keys2) {
        diffs.push(DiffEntry {
            section: "params",
            key: (*key).clone(),
            kind: DiffKind::Removed,
            detail: None,
        });
    }
    for key in keys2.difference(&keys1) {
        diffs.push(DiffEntry {
            section: "params",
            key: (*key).clone(),
            kind: DiffKind::Added,
            detail: param_value_str(&c2.params[*key]),
        });
    }
    for key in keys1.intersection(&keys2) {
        let v1 = format!("{:?}", c1.params[*key]);
        let v2 = format!("{:?}", c2.params[*key]);
        if v1 != v2 {
            diffs.push(DiffEntry {
                section: "params",
                key: (*key).clone(),
                kind: DiffKind::Modified,
                detail: Some(format!("{v1} → {v2}")),
            });
        }
    }
}

fn diff_outputs(c1: &types::ForjarConfig, c2: &types::ForjarConfig, diffs: &mut Vec<DiffEntry>) {
    let keys1: std::collections::HashSet<&String> = c1.outputs.keys().collect();
    let keys2: std::collections::HashSet<&String> = c2.outputs.keys().collect();

    for key in keys1.difference(&keys2) {
        diffs.push(DiffEntry {
            section: "outputs",
            key: (*key).clone(),
            kind: DiffKind::Removed,
            detail: None,
        });
    }
    for key in keys2.difference(&keys1) {
        diffs.push(DiffEntry {
            section: "outputs",
            key: (*key).clone(),
            kind: DiffKind::Added,
            detail: None,
        });
    }
    for key in keys1.intersection(&keys2) {
        let v1 = &c1.outputs[*key].value;
        let v2 = &c2.outputs[*key].value;
        if v1 != v2 {
            diffs.push(DiffEntry {
                section: "outputs",
                key: (*key).clone(),
                kind: DiffKind::Modified,
                detail: Some(format!("{v1} → {v2}")),
            });
        }
    }
}

/// Summarize which fields differ between two resources.
fn resource_field_diff(r1: &types::Resource, r2: &types::Resource) -> String {
    let mut changes = Vec::new();
    if r1.resource_type != r2.resource_type {
        changes.push("type");
    }
    if r1.state != r2.state {
        changes.push("state");
    }
    if r1.path != r2.path {
        changes.push("path");
    }
    if r1.content != r2.content {
        changes.push("content");
    }
    if r1.packages != r2.packages {
        changes.push("packages");
    }
    if r1.version != r2.version {
        changes.push("version");
    }
    if r1.mode != r2.mode {
        changes.push("mode");
    }
    if r1.owner != r2.owner {
        changes.push("owner");
    }
    if r1.tags != r2.tags {
        changes.push("tags");
    }
    if changes.is_empty() {
        "modified".to_string()
    } else {
        format!("changed: {}", changes.join(", "))
    }
}

fn param_value_str(v: &serde_yaml_ng::Value) -> Option<String> {
    match v {
        serde_yaml_ng::Value::String(s) => Some(s.clone()),
        other => Some(format!("{other:?}")),
    }
}

fn print_text(diffs: &[DiffEntry], file1: &Path, file2: &Path) {
    println!("Stack diff: {} vs {}\n", file1.display(), file2.display(),);

    let sections = ["resources", "machines", "params", "outputs"];
    for section in &sections {
        let section_diffs: Vec<&DiffEntry> =
            diffs.iter().filter(|d| d.section == *section).collect();
        if section_diffs.is_empty() {
            continue;
        }
        println!("  {section}:");
        for d in &section_diffs {
            let icon = match d.kind {
                DiffKind::Added => green("+"),
                DiffKind::Removed => red("-"),
                DiffKind::Modified => yellow("~"),
            };
            let detail = d
                .detail
                .as_deref()
                .map(|s| format!(" ({s})"))
                .unwrap_or_default();
            println!("    {icon} {}{detail}", d.key);
        }
    }

    let total = diffs.len();
    let added = diffs
        .iter()
        .filter(|d| matches!(d.kind, DiffKind::Added))
        .count();
    let removed = diffs
        .iter()
        .filter(|d| matches!(d.kind, DiffKind::Removed))
        .count();
    let modified = diffs
        .iter()
        .filter(|d| matches!(d.kind, DiffKind::Modified))
        .count();
    if total == 0 {
        println!("  No differences found.");
    } else {
        println!("\n  {total} difference(s): +{added} -{removed} ~{modified}");
    }
}

fn print_json(diffs: &[DiffEntry], file1: &Path, file2: &Path) -> Result<(), String> {
    let items: Vec<serde_json::Value> = diffs
        .iter()
        .map(|d| {
            serde_json::json!({
                "section": d.section,
                "key": d.key,
                "change": match d.kind {
                    DiffKind::Added => "added",
                    DiffKind::Removed => "removed",
                    DiffKind::Modified => "modified",
                },
                "detail": d.detail,
            })
        })
        .collect();
    let result = serde_json::json!({
        "from": file1.display().to_string(),
        "to": file2.display().to_string(),
        "diffs": items,
        "total": diffs.len(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|e| format!("JSON error: {e}"))?
    );
    Ok(())
}
