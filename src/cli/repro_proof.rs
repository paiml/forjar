//! FJ-1409: Training reproducibility proof.
//!
//! Generates a reproducibility certificate that proves identical training
//! output given identical inputs: config BLAKE3 + store hashes + git SHA.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

pub(crate) fn cmd_repro_proof(
    file: &Path,
    state_dir: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // 1. Config hash
    let config_bytes = std::fs::read(file).map_err(|e| format!("cannot read config: {e}"))?;
    let config_hash = blake3::hash(&config_bytes).to_hex()[..16].to_string();

    // 2. Git SHA (if in a git repo)
    let git_sha = get_git_sha(file);

    // 3. Store artifact hashes
    let store_hashes = collect_store_hashes(file);

    // 4. Model/training resource hashes
    let training_hashes = collect_training_hashes(&config);

    // 5. State hashes
    let state_hash = compute_state_hash(state_dir);

    // 6. Compute reproducibility certificate hash
    let cert_hash = compute_cert_hash(&config_hash, &git_sha, &store_hashes, &state_hash);

    let cert = ReproCert {
        name: &config.name,
        config_hash: &config_hash,
        git_sha: &git_sha,
        store_hashes: &store_hashes,
        training_hashes: &training_hashes,
        state_hash: &state_hash,
        cert_hash: &cert_hash,
    };

    if json {
        print_repro_json(&cert);
    } else {
        print_repro_text(&cert);
    }

    Ok(())
}

fn get_git_sha(file: &Path) -> Option<String> {
    let dir = file.parent()?;
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn collect_store_hashes(file: &Path) -> Vec<(String, String)> {
    let store_dir = file.parent().unwrap_or(Path::new(".")).join("store");
    let mut hashes = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&store_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(bytes) = std::fs::read(&path) {
                    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let hash = blake3::hash(&bytes).to_hex()[..16].to_string();
                    hashes.push((name, hash));
                }
            }
        }
    }
    hashes.sort();
    hashes
}

fn collect_training_hashes(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut hashes = Vec::new();
    for (id, resource) in &config.resources {
        let is_training = matches!(resource.resource_type, types::ResourceType::Model)
            || resource.tags.iter().any(|t| t.contains("training") || t.contains("ml"));
        if !is_training {
            continue;
        }
        let mut hasher = blake3::Hasher::new();
        hasher.update(id.as_bytes());
        let rtype = &resource.resource_type;
        hasher.update(format!("{rtype:?}").as_bytes());
        if let Some(ref c) = resource.content {
            hasher.update(c.as_bytes());
        }
        if let Some(ref s) = resource.source {
            hasher.update(s.as_bytes());
        }
        let hash = hasher.finalize().to_hex()[..16].to_string();
        hashes.push((id.clone(), hash));
    }
    hashes.sort();
    hashes
}

fn compute_state_hash(state_dir: &Path) -> Option<String> {
    let global = state_dir.join("forjar.lock.yaml");
    if global.exists() {
        if let Ok(bytes) = std::fs::read(&global) {
            return Some(blake3::hash(&bytes).to_hex()[..16].to_string());
        }
    }
    None
}

fn compute_cert_hash(
    config_hash: &str,
    git_sha: &Option<String>,
    store_hashes: &[(String, String)],
    state_hash: &Option<String>,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(config_hash.as_bytes());
    if let Some(ref sha) = git_sha {
        hasher.update(sha.as_bytes());
    }
    for (name, hash) in store_hashes {
        hasher.update(name.as_bytes());
        hasher.update(hash.as_bytes());
    }
    if let Some(ref sh) = state_hash {
        hasher.update(sh.as_bytes());
    }
    hasher.finalize().to_hex()[..16].to_string()
}

struct ReproCert<'a> {
    name: &'a str,
    config_hash: &'a str,
    git_sha: &'a Option<String>,
    store_hashes: &'a [(String, String)],
    training_hashes: &'a [(String, String)],
    state_hash: &'a Option<String>,
    cert_hash: &'a str,
}

fn print_repro_json(c: &ReproCert<'_>) {
    let git = c.git_sha.as_deref().unwrap_or("null");
    let state = c.state_hash.as_deref().unwrap_or("null");
    let stores: Vec<String> = c
        .store_hashes
        .iter()
        .map(|(n, h)| format!(r#"{{"name":"{n}","hash":"{h}"}}"#))
        .collect();
    let training: Vec<String> = c
        .training_hashes
        .iter()
        .map(|(n, h)| format!(r#"{{"resource":"{n}","hash":"{h}"}}"#))
        .collect();
    let cert = c.cert_hash;
    let name = c.name;
    let ch = c.config_hash;
    let s_join = stores.join(",");
    let t_join = training.join(",");

    println!(
        r#"{{"certificate":"{cert}","stack":"{name}","config_hash":"{ch}","git_sha":"{git}","state_hash":"{state}","store_artifacts":[{s_join}],"training_resources":[{t_join}]}}"#,
    );
}

fn print_repro_text(c: &ReproCert<'_>) {
    println!("{}\n", bold("Reproducibility Certificate"));
    println!("  Stack:       {}", bold(c.name));
    println!("  Certificate: {}", green(c.cert_hash));
    let ch = c.config_hash;
    println!("  Config hash: blake3:{ch}");
    if let Some(ref sha) = c.git_sha {
        println!("  Git SHA:     {}", &sha[..std::cmp::min(sha.len(), 12)]);
    }
    if let Some(ref sh) = c.state_hash {
        println!("  State hash:  blake3:{sh}");
    }

    if !c.store_hashes.is_empty() {
        println!("\n  Store artifacts ({}):", c.store_hashes.len());
        for (name, hash) in c.store_hashes {
            println!("    {} {name} {}", dim("-"), dim(hash));
        }
    }

    if !c.training_hashes.is_empty() {
        println!("\n  Training resources ({}):", c.training_hashes.len());
        for (name, hash) in c.training_hashes {
            println!("    {} {name} {}", green("*"), dim(hash));
        }
    }

    println!(
        "\n  {} Identical inputs produce identical certificate hash",
        green("✓")
    );
}
