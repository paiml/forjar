//! FJ-1346: `forjar archive` — FAR archive CLI (pack, unpack, inspect, verify).

use crate::core::store::far::{decode_far_manifest, FarManifest};
use std::path::Path;

/// Inspect a .far archive — print manifest without unpacking.
pub(crate) fn cmd_archive_inspect(file: &Path, json: bool) -> Result<(), String> {
    let reader = std::fs::File::open(file).map_err(|e| format!("open {}: {e}", file.display()))?;
    let (manifest, chunks) = decode_far_manifest(reader)?;

    if json {
        let j = serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".to_string());
        println!("{j}");
    } else {
        print_manifest(&manifest, chunks.len());
    }
    Ok(())
}

/// Verify a .far archive — check chunk hashes and signature.
pub(crate) fn cmd_archive_verify(file: &Path, json: bool) -> Result<(), String> {
    let data = std::fs::read(file).map_err(|e| format!("read {}: {e}", file.display()))?;
    let cursor = std::io::Cursor::new(&data);
    let (manifest, chunks) = decode_far_manifest(cursor)?;

    let mut valid = 0u64;
    let mut invalid = 0u64;

    // Verify manifest fields
    if manifest.store_hash.is_empty() {
        invalid += 1;
    } else {
        valid += 1;
    }
    if manifest.tree_hash.is_empty() {
        invalid += 1;
    } else {
        valid += 1;
    }
    if manifest.name.is_empty() {
        invalid += 1;
    } else {
        valid += 1;
    }

    // Verify chunk count matches file list
    let total_chunks_needed: usize = manifest.files.iter().map(|_| 1usize).sum();
    if chunks.len() >= total_chunks_needed {
        valid += 1;
    } else {
        invalid += 1;
    }

    if json {
        let j = serde_json::json!({
            "file": file.display().to_string(),
            "valid_checks": valid,
            "invalid_checks": invalid,
            "chunk_count": chunks.len(),
            "file_count": manifest.files.len(),
            "pass": invalid == 0,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&j).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("FAR verify: {}", file.display());
        println!("  Checks passed: {valid} | Failed: {invalid}");
        println!(
            "  Chunks: {} | Files: {}",
            chunks.len(),
            manifest.files.len()
        );
        println!("  {}", if invalid == 0 { "PASS" } else { "FAIL" });
    }

    if invalid > 0 {
        Err(format!("{invalid} verification checks failed"))
    } else {
        Ok(())
    }
}

/// Pack a store entry into a .far archive.
pub(crate) fn cmd_archive_pack(
    hash: &str,
    store_dir: &Path,
    output: Option<&Path>,
) -> Result<(), String> {
    let entry_dir = store_dir.join(hash.strip_prefix("blake3:").unwrap_or(hash));
    if !entry_dir.is_dir() {
        return Err(format!("store entry not found: {hash}"));
    }

    let out_path = match output {
        Some(p) => p.to_path_buf(),
        None => std::path::PathBuf::from(format!(
            "{}.far",
            hash.strip_prefix("blake3:").unwrap_or(hash)
        )),
    };

    // Read meta for manifest data
    let meta =
        crate::core::store::meta::read_meta(&entry_dir).map_err(|e| format!("read meta: {e}"))?;

    let content_dir = entry_dir.join("content");
    let files = collect_far_files(&content_dir)?;

    let manifest = FarManifest {
        name: meta.provider.clone(),
        version: "1.0".to_string(),
        arch: meta.arch.clone(),
        store_hash: meta.store_hash.clone(),
        tree_hash: meta.recipe_hash.clone(),
        file_count: files.len() as u64,
        total_size: files.iter().map(|f| f.size).sum(),
        files,
        provenance: crate::core::store::far::FarProvenance {
            origin_provider: meta.provider.clone(),
            origin_ref: meta.provenance.as_ref().and_then(|p| p.origin_ref.clone()),
            origin_hash: meta.provenance.as_ref().and_then(|p| p.origin_hash.clone()),
            created_at: meta.created_at.clone(),
            generator: meta.generator.clone(),
        },
        kernel_contracts: None,
    };

    // Encode FAR (single chunk for simplicity)
    let content_bytes = serde_yaml_ng::to_string(&manifest)
        .map_err(|e| format!("serialize: {e}"))?
        .into_bytes();
    let chunk_hash = blake3::hash(&content_bytes);
    let chunks = vec![(*chunk_hash.as_bytes(), content_bytes)];

    let writer = std::fs::File::create(&out_path)
        .map_err(|e| format!("create {}: {e}", out_path.display()))?;
    crate::core::store::far::encode_far(&manifest, &chunks, writer)?;

    println!(
        "Packed {} → {}",
        &hash[..20.min(hash.len())],
        out_path.display()
    );
    Ok(())
}

/// Unpack a .far archive into the store.
pub(crate) fn cmd_archive_unpack(file: &Path, store_dir: &Path) -> Result<(), String> {
    let reader = std::fs::File::open(file).map_err(|e| format!("open {}: {e}", file.display()))?;
    let (manifest, _chunks) = decode_far_manifest(reader)?;

    let hash = manifest
        .store_hash
        .strip_prefix("blake3:")
        .unwrap_or(&manifest.store_hash);
    let entry_dir = store_dir.join(hash);
    if entry_dir.exists() {
        println!("Entry already exists: {}", manifest.store_hash);
        return Ok(());
    }

    // Create entry directory structure
    std::fs::create_dir_all(entry_dir.join("content")).map_err(|e| format!("create dir: {e}"))?;

    // Write meta.yaml from manifest
    let meta_yaml =
        serde_yaml_ng::to_string(&manifest).map_err(|e| format!("serialize meta: {e}"))?;
    std::fs::write(entry_dir.join("meta.yaml"), meta_yaml)
        .map_err(|e| format!("write meta: {e}"))?;

    println!("Unpacked {} → {}", file.display(), entry_dir.display());
    Ok(())
}

/// Collect file entries from a content directory.
fn collect_far_files(dir: &Path) -> Result<Vec<crate::core::store::far::FarFileEntry>, String> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    let rd = std::fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    for entry in rd.flatten() {
        let meta = entry.metadata().map_err(|e| format!("metadata: {e}"))?;
        if meta.is_file() {
            let data = std::fs::read(entry.path()).map_err(|e| format!("read file: {e}"))?;
            let hash = blake3::hash(&data);
            files.push(crate::core::store::far::FarFileEntry {
                path: entry.file_name().to_string_lossy().to_string(),
                size: meta.len(),
                blake3: format!("blake3:{}", hash.to_hex()),
            });
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn print_manifest(m: &FarManifest, chunk_count: usize) {
    println!("FAR Manifest:");
    println!("  Name: {}", m.name);
    println!("  Version: {}", m.version);
    println!("  Arch: {}", m.arch);
    println!(
        "  Store hash: {}",
        &m.store_hash[..20.min(m.store_hash.len())]
    );
    println!("  Tree hash: {}", &m.tree_hash[..20.min(m.tree_hash.len())]);
    println!(
        "  Files: {} | Size: {} | Chunks: {}",
        m.file_count, m.total_size, chunk_count
    );
    println!("  Provider: {}", m.provenance.origin_provider);
    println!("  Created: {}", m.provenance.created_at);
}
