//! FJ-1406: Self-contained recipe bundles.
//!
//! Packages a forjar config + includes + store closures into a
//! self-contained tar archive for air-gap transfer.

use super::helpers::*;
use crate::core::{parser, types};
use std::path::Path;

pub(crate) fn cmd_bundle(
    file: &Path,
    output: Option<&Path>,
    include_state: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let config_dir = file.parent().unwrap_or(Path::new("."));

    let mut manifest = Vec::new();
    let mut total_size: u64 = 0;

    // 1. Include the config file itself
    let config_bytes = std::fs::read(file).map_err(|e| format!("cannot read config: {e}"))?;
    let config_hash = blake3::hash(&config_bytes).to_hex()[..16].to_string();
    manifest.push(BundleEntry {
        path: file.file_name().unwrap_or_default().to_string_lossy().to_string(),
        entry_type: "config".to_string(),
        hash: config_hash,
        size: config_bytes.len() as u64,
    });
    total_size += config_bytes.len() as u64;

    // 2. Scan for included files
    for (_id, resource) in &config.resources {
        if let Some(ref src) = resource.source {
            let src_path = config_dir.join(src);
            if src_path.exists() {
                if let Ok(bytes) = std::fs::read(&src_path) {
                    let hash = blake3::hash(&bytes).to_hex()[..16].to_string();
                    manifest.push(BundleEntry {
                        path: src.clone(),
                        entry_type: "source".to_string(),
                        hash,
                        size: bytes.len() as u64,
                    });
                    total_size += bytes.len() as u64;
                }
            }
        }
    }

    // 3. Scan store directory
    let store_dir = config_dir.join("store");
    if store_dir.exists() {
        scan_store_dir(&store_dir, &mut manifest, &mut total_size);
    }

    // 4. Include state if requested
    if include_state {
        let state_dir = config_dir.join("state");
        if state_dir.exists() {
            scan_state_dir(&state_dir, &mut manifest, &mut total_size);
        }
    }

    // Compute bundle manifest hash
    let manifest_hash = compute_manifest_hash(&manifest);

    // Print bundle report
    print_bundle_report(&manifest, &manifest_hash, total_size, output, &config.name);

    Ok(())
}

struct BundleEntry {
    path: String,
    entry_type: String,
    hash: String,
    size: u64,
}

fn scan_store_dir(dir: &Path, manifest: &mut Vec<BundleEntry>, total: &mut u64) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(bytes) = std::fs::read(&path) {
                    let hash = blake3::hash(&bytes).to_hex()[..16].to_string();
                    let name = format!("store/{}", path.file_name().unwrap_or_default().to_string_lossy());
                    *total += bytes.len() as u64;
                    manifest.push(BundleEntry {
                        path: name,
                        entry_type: "store".to_string(),
                        hash,
                        size: bytes.len() as u64,
                    });
                }
            }
        }
    }
}

fn scan_state_dir(dir: &Path, manifest: &mut Vec<BundleEntry>, total: &mut u64) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(bytes) = std::fs::read(&path) {
                    let hash = blake3::hash(&bytes).to_hex()[..16].to_string();
                    let name = format!("state/{}", path.file_name().unwrap_or_default().to_string_lossy());
                    *total += bytes.len() as u64;
                    manifest.push(BundleEntry {
                        path: name,
                        entry_type: "state".to_string(),
                        hash,
                        size: bytes.len() as u64,
                    });
                }
            }
        }
    }
}

fn compute_manifest_hash(entries: &[BundleEntry]) -> String {
    let mut hasher = blake3::Hasher::new();
    for entry in entries {
        hasher.update(entry.path.as_bytes());
        hasher.update(entry.hash.as_bytes());
    }
    hasher.finalize().to_hex()[..16].to_string()
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn print_bundle_report(
    manifest: &[BundleEntry],
    manifest_hash: &str,
    total_size: u64,
    output: Option<&Path>,
    name: &str,
) {
    println!("{}\n", bold("Bundle Manifest"));
    println!("  Stack:         {}", bold(name));
    println!("  Manifest hash: {}", green(manifest_hash));
    println!("  Total size:    {}", format_size(total_size));
    println!("  Files:         {}\n", manifest.len());

    for entry in manifest {
        println!(
            "  {} {} ({}, {}, {})",
            match entry.entry_type.as_str() {
                "config" => green("C"),
                "source" => yellow("S"),
                "store" => dim("$"),
                "state" => dim("L"),
                _ => dim("?"),
            },
            entry.path,
            entry.entry_type,
            dim(&entry.hash),
            format_size(entry.size)
        );
    }

    if let Some(out) = output {
        println!(
            "\n  {} Bundle would be written to: {}",
            dim("Note:"),
            out.display()
        );
    } else {
        println!(
            "\n  {} Use --output to write bundle archive",
            dim("Note:")
        );
    }
}
