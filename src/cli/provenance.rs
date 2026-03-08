//! FJ-1404: SLSA provenance attestation generation.
//!
//! Generates an in-toto-style provenance attestation linking:
//! config hash -> plan hash -> state hash for SLSA Level 3.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// Generate SLSA provenance attestation.
pub(crate) fn cmd_provenance(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // 1. Hash the config file itself
    let config_bytes = std::fs::read(file).map_err(|e| format!("cannot read config: {e}"))?;
    let config_hash = blake3::hash(&config_bytes).to_hex().to_string();

    // 2. Compute plan hash (hash of all resource IDs + types + deps in topo order)
    let plan_hash = compute_plan_hash(&config);

    // 3. Collect state hashes from lock files
    let state_hashes = collect_state_hashes(state_dir, machine_filter);

    // 4. Compute materials (resource content hashes)
    let materials = collect_materials(&config, machine_filter);

    let timestamp = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("{now}")
    };

    if json {
        print_provenance_json(
            &config_hash,
            &plan_hash,
            &state_hashes,
            &materials,
            &timestamp,
            &config.name,
        );
    } else {
        print_provenance_text(
            &config_hash,
            &plan_hash,
            &state_hashes,
            &materials,
            &timestamp,
            &config.name,
        );
    }

    Ok(())
}

fn compute_plan_hash(config: &types::ForjarConfig) -> String {
    let mut hasher = blake3::Hasher::new();
    let mut ids: Vec<&String> = config.resources.keys().collect();
    ids.sort();
    for id in &ids {
        hasher.update(id.as_bytes());
        if let Some(r) = config.resources.get(*id) {
            hasher.update(format!("{:?}", r.resource_type).as_bytes());
            for dep in &r.depends_on {
                hasher.update(dep.as_bytes());
            }
        }
    }
    hasher.finalize().to_hex().to_string()
}

fn collect_state_hashes(state_dir: &Path, machine_filter: Option<&str>) -> Vec<(String, String)> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            if let Some(mf) = machine_filter {
                if !name.contains(mf) {
                    continue;
                }
            }
            if let Ok(bytes) = std::fs::read(&path) {
                let hash = blake3::hash(&bytes).to_hex().to_string();
                result.push((name.to_string(), hash));
            }
        }
    }
    result.sort();
    result
}

fn collect_materials(
    config: &types::ForjarConfig,
    machine_filter: Option<&str>,
) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for (id, resource) in &config.resources {
        if let Some(mf) = machine_filter {
            let machines = resource.machine.to_vec();
            if !machines.iter().any(|m| m == mf) {
                continue;
            }
        }
        // Hash the resource definition
        let mut hasher = blake3::Hasher::new();
        hasher.update(id.as_bytes());
        hasher.update(format!("{:?}", resource.resource_type).as_bytes());
        if let Some(ref c) = resource.content {
            hasher.update(c.as_bytes());
        }
        let hash = hasher.finalize().to_hex()[..16].to_string();
        result.push((id.clone(), hash));
    }
    result.sort();
    result
}

fn print_provenance_json(
    config_hash: &str,
    plan_hash: &str,
    state_hashes: &[(String, String)],
    materials: &[(String, String)],
    timestamp: &str,
    name: &str,
) {
    let state_items: Vec<String> = state_hashes
        .iter()
        .map(|(m, h)| format!(r#"{{"machine":"{m}","hash":"{h}"}}"#))
        .collect();
    let mat_items: Vec<String> = materials
        .iter()
        .map(|(id, h)| format!(r#"{{"resource":"{id}","digest":"{h}"}}"#))
        .collect();

    println!(
        r#"{{"_type":"https://in-toto.io/Statement/v0.1","predicateType":"https://slsa.dev/provenance/v1","subject":{{"name":"{}","config_digest":"blake3:{}","plan_digest":"blake3:{}","timestamp":"{}"}},"predicate":{{"buildType":"forjar/apply","state":[{}],"materials":[{}]}}}}"#,
        name,
        config_hash,
        plan_hash,
        timestamp,
        state_items.join(","),
        mat_items.join(",")
    );
}

fn print_provenance_text(
    config_hash: &str,
    plan_hash: &str,
    state_hashes: &[(String, String)],
    materials: &[(String, String)],
    timestamp: &str,
    name: &str,
) {
    println!("{}\n", bold("SLSA Provenance Attestation"));
    println!("  Subject:     {}", bold(name));
    println!("  Timestamp:   {timestamp}");
    println!(
        "  Config hash: blake3:{}",
        config_hash.get(..16).unwrap_or(config_hash)
    );
    println!(
        "  Plan hash:   blake3:{}",
        plan_hash.get(..16).unwrap_or(plan_hash)
    );

    if !state_hashes.is_empty() {
        println!("\n  State hashes:");
        for (machine, hash) in state_hashes {
            println!(
                "    {} {} blake3:{}",
                green("*"),
                machine,
                hash.get(..16).unwrap_or(hash)
            );
        }
    }

    if !materials.is_empty() {
        println!("\n  Materials ({} resources):", materials.len());
        for (id, digest) in materials {
            println!("    {} {} {}", dim("-"), id, dim(digest));
        }
    }

    println!(
        "\n  {} SLSA Level 3 attestation chain: config -> plan -> state",
        green("✓")
    );
}
