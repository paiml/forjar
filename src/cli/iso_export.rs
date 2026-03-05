//! FJ-1422: ISO distribution generation.
//!
//! `forjar export --format iso` packages config, state, store closures,
//! and the forjar binary itself into a self-contained directory structure
//! suitable for burning to ISO media or transferring across air gaps.

use super::helpers::*;
use std::path::Path;

/// ISO export manifest.
#[derive(Debug, serde::Serialize)]
pub struct IsoManifest {
    pub name: String,
    pub version: String,
    pub files: Vec<IsoFile>,
    pub total_size: u64,
    pub blake3_root: String,
}

#[derive(Debug, serde::Serialize)]
pub struct IsoFile {
    pub path: String,
    pub size: u64,
    pub blake3: String,
    pub category: String,
}

/// Generate an ISO-ready distribution directory.
pub fn cmd_iso_export(
    file: &Path,
    state_dir: &Path,
    output: &Path,
    include_binary: bool,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Create output directory structure
    std::fs::create_dir_all(output.join("config"))
        .map_err(|e| format!("create config dir: {e}"))?;
    std::fs::create_dir_all(output.join("state")).map_err(|e| format!("create state dir: {e}"))?;
    std::fs::create_dir_all(output.join("store")).map_err(|e| format!("create store dir: {e}"))?;

    let mut files = Vec::new();
    let mut total_size = 0u64;

    // Copy config file
    let config_dest = output.join("config").join("forjar.yaml");
    std::fs::copy(file, &config_dest).map_err(|e| format!("copy config: {e}"))?;
    let config_size = std::fs::metadata(&config_dest)
        .map(|m| m.len())
        .unwrap_or(0);
    let config_hash = hash_file_blake3(&config_dest);
    files.push(IsoFile {
        path: "config/forjar.yaml".to_string(),
        size: config_size,
        blake3: config_hash,
        category: "config".to_string(),
    });
    total_size += config_size;

    // Copy state directory if it exists
    if state_dir.exists() {
        copy_state_dir(
            state_dir,
            &output.join("state"),
            &mut files,
            &mut total_size,
        )?;
    }

    // Collect store artifacts from resources
    for (_id, res) in &config.resources {
        for artifact in &res.output_artifacts {
            let artifact_path = Path::new(artifact);
            if artifact_path.exists() {
                let fname = artifact_path.file_name().unwrap_or_default();
                let dest = output.join("store").join(fname);
                std::fs::copy(artifact_path, &dest)
                    .map_err(|e| format!("copy artifact {artifact}: {e}"))?;
                let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
                let hash = hash_file_blake3(&dest);
                files.push(IsoFile {
                    path: format!("store/{}", fname.to_string_lossy()),
                    size,
                    blake3: hash,
                    category: "artifact".to_string(),
                });
                total_size += size;
            }
        }
    }

    // Optionally include forjar binary
    if include_binary {
        copy_binary(output, &mut files, &mut total_size);
    }

    // Compute root hash from all file hashes
    let root_hash = compute_root_hash(&files);

    // Write manifest
    let manifest = IsoManifest {
        name: config.name.clone(),
        version: config.version.clone(),
        files,
        total_size,
        blake3_root: root_hash,
    };

    let manifest_json =
        serde_json::to_string_pretty(&manifest).map_err(|e| format!("JSON error: {e}"))?;
    std::fs::write(output.join("manifest.json"), &manifest_json)
        .map_err(|e| format!("write manifest: {e}"))?;

    if json {
        println!("{manifest_json}");
    } else {
        println!("ISO Export: {}", output.display());
        println!(
            "Files: {} | Size: {} bytes | Root: {}",
            manifest.files.len(),
            manifest.total_size,
            &manifest.blake3_root[..16]
        );
    }

    Ok(())
}

fn hash_file_blake3(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(data) => {
            let hash = blake3::hash(&data);
            hash.to_hex().to_string()
        }
        Err(_) => "0".repeat(64),
    }
}

fn compute_root_hash(files: &[IsoFile]) -> String {
    let mut hasher = blake3::Hasher::new();
    for f in files {
        hasher.update(f.blake3.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

fn copy_binary(output: &Path, files: &mut Vec<IsoFile>, total_size: &mut u64) {
    if let Ok(exe) = std::env::current_exe() {
        let bin_dir = output.join("bin");
        if std::fs::create_dir_all(&bin_dir).is_ok() {
            let dest = bin_dir.join("forjar");
            if std::fs::copy(&exe, &dest).is_ok() {
                let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
                let hash = hash_file_blake3(&dest);
                files.push(IsoFile {
                    path: "bin/forjar".to_string(),
                    size,
                    blake3: hash,
                    category: "binary".to_string(),
                });
                *total_size += size;
            }
        }
    }
}

fn copy_state_dir(
    src: &Path,
    dest: &Path,
    files: &mut Vec<IsoFile>,
    total_size: &mut u64,
) -> Result<(), String> {
    let entries = std::fs::read_dir(src).map_err(|e| format!("read state dir: {e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            let sub_dest = dest.join(&name);
            std::fs::create_dir_all(&sub_dest).map_err(|e| format!("create state subdir: {e}"))?;
            copy_state_dir(&path, &sub_dest, files, total_size)?;
        } else if path.is_file() {
            let file_dest = dest.join(&name);
            std::fs::copy(&path, &file_dest).map_err(|e| format!("copy state file: {e}"))?;
            let size = std::fs::metadata(&file_dest).map(|m| m.len()).unwrap_or(0);
            let hash = hash_file_blake3(&file_dest);
            files.push(IsoFile {
                path: format!("state/{}", name.to_string_lossy()),
                size,
                blake3: hash,
                category: "state".to_string(),
            });
            *total_size += size;
        }
    }
    Ok(())
}
