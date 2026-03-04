//! FJ-1414: Data sovereignty tagging.
//!
//! Tags every piece of state with jurisdiction, classification, and
//! residency zone metadata for compliance.

use super::helpers::*;
use std::path::Path;

pub(crate) fn cmd_sovereignty(file: &Path, state_dir: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let mut entries = Vec::new();

    for (id, resource) in &config.resources {
        let jurisdiction = extract_tag(&resource.tags, "jurisdiction:");
        let classification = extract_tag(&resource.tags, "classification:");
        let residency = extract_tag(&resource.tags, "residency:");
        let machines = resource.machine.to_vec();

        entries.push(SovereigntyEntry {
            id: id.clone(),
            resource_type: format!("{:?}", resource.resource_type),
            machines,
            jurisdiction,
            classification,
            residency,
        });
    }

    // Check state directory for lock files and their sovereignty
    let state_sov = scan_state_sovereignty(state_dir);

    if json {
        print_sovereignty_json(&entries, &state_sov, &config.name);
    } else {
        print_sovereignty_text(&entries, &state_sov, &config.name);
    }

    Ok(())
}

fn extract_tag(tags: &[String], prefix: &str) -> Option<String> {
    tags.iter()
        .find(|t| t.starts_with(prefix))
        .map(|t| t[prefix.len()..].to_string())
}

struct SovereigntyEntry {
    id: String,
    resource_type: String,
    machines: Vec<String>,
    jurisdiction: Option<String>,
    classification: Option<String>,
    residency: Option<String>,
}

struct StateSov {
    file: String,
    hash: String,
}

fn scan_state_sovereignty(state_dir: &Path) -> Vec<StateSov> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "yaml").unwrap_or(false) {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let hash = std::fs::read(&path)
                    .ok()
                    .map(|bytes| blake3::hash(&bytes).to_hex()[..16].to_string())
                    .unwrap_or_default();
                results.push(StateSov { file: name, hash });
            }
        }
    }
    results
}

fn print_sovereignty_json(entries: &[SovereigntyEntry], state: &[StateSov], name: &str) {
    let resources: Vec<String> = entries
        .iter()
        .map(|e| {
            let j = e.jurisdiction.as_deref().unwrap_or("untagged");
            let c = e.classification.as_deref().unwrap_or("untagged");
            let r = e.residency.as_deref().unwrap_or("untagged");
            let machines: Vec<String> = e.machines.iter().map(|m| format!(r#""{m}""#)).collect();
            format!(
                r#"{{"id":"{id}","type":"{rt}","machines":[{m}],"jurisdiction":"{j}","classification":"{c}","residency":"{r}"}}"#,
                id = e.id,
                rt = e.resource_type,
                m = machines.join(","),
            )
        })
        .collect();

    let state_items: Vec<String> = state
        .iter()
        .map(|s| format!(r#"{{"file":"{f}","hash":"{h}"}}"#, f = s.file, h = s.hash))
        .collect();

    println!(
        r#"{{"stack":"{name}","resources":[{r}],"state_files":[{s}]}}"#,
        r = resources.join(","),
        s = state_items.join(","),
    );
}

fn print_sovereignty_text(entries: &[SovereigntyEntry], state: &[StateSov], name: &str) {
    println!("{}\n", bold("Data Sovereignty Report"));
    println!("  Stack: {}", bold(name));
    println!("  Resources: {}\n", entries.len());

    let tagged = entries.iter().filter(|e| e.jurisdiction.is_some()).count();
    let untagged = entries.len() - tagged;

    for e in entries {
        let j = e.jurisdiction.as_deref().unwrap_or("—");
        let c = e.classification.as_deref().unwrap_or("—");
        let r = e.residency.as_deref().unwrap_or("—");
        let icon = if e.jurisdiction.is_some() {
            green("✓")
        } else {
            yellow("?")
        };
        println!(
            "  {icon} {} ({}) [J:{j} C:{c} R:{r}]",
            e.id, e.resource_type
        );
    }

    if !state.is_empty() {
        println!("\n  State files ({}):", state.len());
        for s in state {
            println!("    {} {} {}", dim("-"), s.file, dim(&s.hash));
        }
    }

    println!("\n  Tagged: {tagged} | Untagged: {untagged}");
    if untagged > 0 {
        println!(
            "  {} Tag resources with jurisdiction:/classification:/residency: tags",
            yellow("hint:")
        );
    }
}
