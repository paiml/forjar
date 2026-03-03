//! FJ-1400: Cryptographic Bill of Materials (CBOM) generation.
//!
//! Scans forjar config and state for cryptographic algorithm usage:
//! - BLAKE3 hashing in state locks and tripwire
//! - age/X25519 encryption in secrets
//! - TLS certificates in file resources
//! - SSH key types in machine definitions
//! - Docker image digest algorithms

use crate::core::{parser, state, types};
use std::path::Path;

/// Generate CBOM from config and state.
pub(crate) fn cmd_cbom(
    file: &Path,
    state_dir: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parser::parse_and_validate(file)?;
    let entries = collect_crypto_entries(&config, state_dir);

    if json {
        print_cbom_json(&config, &entries)?;
    } else {
        print_cbom_text(&config, &entries);
    }
    Ok(())
}

struct CryptoEntry {
    algorithm: String,
    usage: String,
    key_size: String,
    location: String,
}

fn collect_crypto_entries(
    config: &types::ForjarConfig,
    state_dir: &Path,
) -> Vec<CryptoEntry> {
    let mut entries = Vec::new();

    // BLAKE3 hashing — always present in forjar state
    entries.push(CryptoEntry {
        algorithm: "BLAKE3".to_string(),
        usage: "state-hashing".to_string(),
        key_size: "256-bit".to_string(),
        location: "state/*.lock.yaml".to_string(),
    });

    // Check for age encryption usage
    scan_age_encryption(config, &mut entries);

    // Check SSH key types from machine definitions
    scan_ssh_keys(config, &mut entries);

    // Check for TLS/cert resources
    scan_tls_resources(config, &mut entries);

    // Check state locks for hash algorithms
    scan_state_hashes(state_dir, &mut entries);

    // Check docker image digests
    scan_docker_digests(config, &mut entries);

    entries
}

fn scan_age_encryption(config: &types::ForjarConfig, entries: &mut Vec<CryptoEntry>) {
    for (_id, resource) in &config.resources {
        if let Some(ref content) = resource.content {
            if content.contains("age-encryption.org") || content.starts_with("-----BEGIN AGE") {
                entries.push(CryptoEntry {
                    algorithm: "X25519".to_string(),
                    usage: "secrets-encryption".to_string(),
                    key_size: "256-bit".to_string(),
                    location: "age-encrypted values".to_string(),
                });
                return; // Only need one entry
            }
        }
    }
    // Check params for age-encrypted values
    for (_key, value) in &config.params {
        if let Some(s) = value.as_str() {
            if s.contains("age-encryption.org") {
                entries.push(CryptoEntry {
                    algorithm: "X25519".to_string(),
                    usage: "secrets-encryption".to_string(),
                    key_size: "256-bit".to_string(),
                    location: "params (age-encrypted)".to_string(),
                });
                return;
            }
        }
    }
}

fn scan_ssh_keys(config: &types::ForjarConfig, entries: &mut Vec<CryptoEntry>) {
    let mut seen_ssh = false;
    for (_name, machine) in &config.machines {
        if machine.ssh_key.is_some() && !seen_ssh {
            entries.push(CryptoEntry {
                algorithm: "Ed25519/RSA".to_string(),
                usage: "ssh-transport".to_string(),
                key_size: "variable".to_string(),
                location: "machine SSH keys".to_string(),
            });
            seen_ssh = true;
        }
    }
}

fn scan_tls_resources(config: &types::ForjarConfig, entries: &mut Vec<CryptoEntry>) {
    for (id, resource) in &config.resources {
        if let Some(ref path) = resource.path {
            if path.contains("ssl") || path.contains("tls") || path.contains(".pem") || path.contains(".crt") {
                entries.push(CryptoEntry {
                    algorithm: "X.509/TLS".to_string(),
                    usage: "certificate-management".to_string(),
                    key_size: "variable".to_string(),
                    location: format!("resource:{id}"),
                });
            }
        }
    }
}

fn scan_state_hashes(state_dir: &Path, entries: &mut Vec<CryptoEntry>) {
    let mut has_hashes = false;
    if let Ok(dir_entries) = std::fs::read_dir(state_dir) {
        for entry in dir_entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(Some(lock)) = state::load_lock(state_dir, &name) {
                for (_id, res) in &lock.resources {
                    if !res.hash.is_empty() && !has_hashes {
                        has_hashes = true;
                        entries.push(CryptoEntry {
                            algorithm: "BLAKE3".to_string(),
                            usage: "resource-integrity".to_string(),
                            key_size: "256-bit".to_string(),
                            location: format!("state/{name}/*.lock.yaml"),
                        });
                    }
                }
            }
        }
    }
}

fn scan_docker_digests(config: &types::ForjarConfig, entries: &mut Vec<CryptoEntry>) {
    for (id, resource) in &config.resources {
        if resource.resource_type == types::ResourceType::Docker {
            if let Some(ref image) = resource.image {
                if image.contains("sha256:") {
                    entries.push(CryptoEntry {
                        algorithm: "SHA-256".to_string(),
                        usage: "container-digest".to_string(),
                        key_size: "256-bit".to_string(),
                        location: format!("resource:{id}"),
                    });
                }
            }
        }
    }
}

fn print_cbom_json(
    config: &types::ForjarConfig,
    entries: &[CryptoEntry],
) -> Result<(), String> {
    let algorithms: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "algorithm": e.algorithm,
                "usage": e.usage,
                "keySize": e.key_size,
                "location": e.location,
            })
        })
        .collect();

    let doc = serde_json::json!({
        "cbomVersion": "1.0",
        "name": format!("forjar-cbom-{}", config.name),
        "cryptoAlgorithms": algorithms,
        "totalAlgorithms": entries.len(),
    });

    let output = serde_json::to_string_pretty(&doc)
        .map_err(|e| format!("JSON error: {e}"))?;
    println!("{output}");
    Ok(())
}

fn print_cbom_text(
    config: &types::ForjarConfig,
    entries: &[CryptoEntry],
) {
    println!("CBOM: {} ({} crypto algorithms)", config.name, entries.len());
    println!("{:-<72}", "");
    println!(
        "{:<16} {:<24} {:<12} {:<20}",
        "ALGORITHM", "USAGE", "KEY SIZE", "LOCATION"
    );
    println!("{:-<72}", "");
    for e in entries {
        println!(
            "{:<16} {:<24} {:<12} {:<20}",
            e.algorithm, e.usage, e.key_size, e.location
        );
    }
    println!("{:-<72}", "");
    println!("Total: {} crypto algorithms", entries.len());
}
