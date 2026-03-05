//! FJ-1348: Conda package reader — .conda (ZIP) and .tar.bz2 formats.
//!
//! Reads conda packages, extracts files, and parses `index.json` metadata.

use super::chunker::{chunk_directory, tree_hash};
use super::far::{encode_far, FarManifest, FarProvenance};
use crate::tripwire::eventlog::now_iso8601;
use crate::tripwire::hasher::hash_directory;
use std::io::Read;
use std::path::Path;

/// Parsed conda package metadata from `index.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct CondaPackageInfo {
    pub name: String,
    pub version: String,
    pub build: String,
    pub arch: String,
    pub subdir: String,
    pub files: Vec<CondaFileEntry>,
}

/// A file entry extracted from a conda package.
#[derive(Debug, Clone, PartialEq)]
pub struct CondaFileEntry {
    pub path: String,
    pub size: u64,
}

/// Auto-detect format by extension and extract to `output_dir`.
pub fn read_conda(path: &Path, output_dir: &Path) -> Result<CondaPackageInfo, String> {
    let ext = path.to_string_lossy().to_string();

    if ext.ends_with(".conda") {
        read_conda_zip(path, output_dir)
    } else if ext.ends_with(".tar.bz2") {
        read_conda_bz2(path, output_dir)
    } else {
        Err(format!(
            "unknown conda format: {} (expected .conda or .tar.bz2)",
            path.display()
        ))
    }
}

/// Decompress a zip entry by name and return the raw tar bytes.
fn decompress_zst_entry(
    archive: &mut zip::ZipArchive<std::fs::File>,
    name: &str,
) -> Result<Vec<u8>, String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|e| format!("zip entry {name}: {e}"))?;
    let mut compressed = Vec::new();
    entry
        .read_to_end(&mut compressed)
        .map_err(|e| format!("read {name}: {e}"))?;
    zstd::decode_all(compressed.as_slice()).map_err(|e| format!("zstd {name}: {e}"))
}

/// Read a modern `.conda` file (ZIP containing tar.zst members).
pub fn read_conda_zip(path: &Path, output_dir: &Path) -> Result<CondaPackageInfo, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("zip read {}: {e}", path.display()))?;

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("mkdir {}: {e}", output_dir.display()))?;

    let mut info: Option<CondaPackageInfo> = None;
    let mut all_files: Vec<CondaFileEntry> = Vec::new();

    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();

    for name in &names {
        if name.starts_with("pkg-") && name.ends_with(".tar.zst") {
            let decompressed = decompress_zst_entry(&mut archive, name)?;
            all_files.extend(extract_tar_bytes(&decompressed, output_dir)?);
        } else if name.starts_with("info-") && name.ends_with(".tar.zst") {
            let decompressed = decompress_zst_entry(&mut archive, name)?;
            info = find_index_in_tar(&decompressed)?;
            all_files.extend(extract_tar_bytes(&decompressed, output_dir)?);
        }
    }

    let mut pkg = info.ok_or_else(|| "no index.json in conda package".to_string())?;
    pkg.files = all_files;
    Ok(pkg)
}

/// Process a single bz2 tar entry: extract to output_dir and optionally parse index.json.
fn process_bz2_entry<R: std::io::Read>(
    entry: &mut tar::Entry<R>,
    output_dir: &Path,
) -> Result<Option<(CondaFileEntry, Option<CondaPackageInfo>)>, String> {
    let path_buf = entry
        .path()
        .map_err(|e| format!("entry path: {e}"))?
        .to_path_buf();
    let rel = path_buf.to_string_lossy().to_string();
    let size = entry.size();

    let dest = output_dir.join(&rel);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    if !entry.header().entry_type().is_file() {
        return Ok(None);
    }

    let mut buf = Vec::with_capacity(size as usize);
    entry
        .read_to_end(&mut buf)
        .map_err(|e| format!("read {rel}: {e}"))?;

    let index_info = if rel == "info/index.json" {
        Some(parse_conda_index(&String::from_utf8_lossy(&buf))?)
    } else {
        None
    };

    std::fs::write(&dest, &buf).map_err(|e| format!("write {}: {e}", dest.display()))?;
    Ok(Some((CondaFileEntry { path: rel, size }, index_info)))
}

/// Read a legacy `.tar.bz2` conda package.
pub fn read_conda_bz2(path: &Path, output_dir: &Path) -> Result<CondaPackageInfo, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let decoder = bzip2::read::BzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("mkdir {}: {e}", output_dir.display()))?;

    let mut info: Option<CondaPackageInfo> = None;
    let mut files: Vec<CondaFileEntry> = Vec::new();

    for entry in archive.entries().map_err(|e| format!("tar entries: {e}"))? {
        let mut entry = entry.map_err(|e| format!("tar entry: {e}"))?;
        if let Some((file_entry, index_info)) = process_bz2_entry(&mut entry, output_dir)? {
            if let Some(pkg_info) = index_info {
                info = Some(pkg_info);
            }
            files.push(file_entry);
        }
    }

    let mut pkg = info.ok_or_else(|| "no info/index.json in tar.bz2".to_string())?;
    pkg.files = files;
    Ok(pkg)
}

/// Parse conda `index.json` into `CondaPackageInfo`.
pub fn parse_conda_index(json: &str) -> Result<CondaPackageInfo, String> {
    let val: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("parse index.json: {e}"))?;

    let name = val["name"]
        .as_str()
        .ok_or("index.json missing 'name'")?
        .to_string();
    let version = val["version"]
        .as_str()
        .ok_or("index.json missing 'version'")?
        .to_string();
    let build = val
        .get("build")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let arch = val
        .get("arch")
        .and_then(|v| v.as_str())
        .unwrap_or("noarch")
        .to_string();
    let subdir = val
        .get("subdir")
        .and_then(|v| v.as_str())
        .unwrap_or("noarch")
        .to_string();

    Ok(CondaPackageInfo {
        name,
        version,
        build,
        arch,
        subdir,
        files: Vec::new(),
    })
}

/// Convert a conda package to FAR format.
///
/// 1. Extract conda package to temp dir
/// 2. Hash the extracted directory (store_hash)
/// 3. Chunk the directory (chunks + file entries)
/// 4. Compute tree hash for verified streaming
/// 5. Build FarManifest with conda provenance
/// 6. Encode FAR to output file
pub fn conda_to_far(conda_path: &Path, far_output: &Path) -> Result<FarManifest, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("forjar-conda-{}-{id}", std::process::id()));
    let extract_dir = tmp.join("extracted");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&extract_dir).map_err(|e| format!("create temp dir: {e}"))?;

    // 1. Extract
    let info = read_conda(conda_path, &extract_dir)?;

    // 2. Store hash
    let store_hash = hash_directory(&extract_dir)?;

    // 3. Chunk
    let (chunks, file_entries) = chunk_directory(&extract_dir)?;

    // 4. Tree hash
    let th = tree_hash(&chunks);
    let tree_hash_str = format!(
        "blake3:{}",
        th.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );

    // 5. Build manifest
    let total_size: u64 = file_entries.iter().map(|f| f.size).sum();
    let manifest = FarManifest {
        name: info.name,
        version: info.version,
        arch: info.arch,
        store_hash,
        tree_hash: tree_hash_str,
        file_count: file_entries.len() as u64,
        total_size,
        files: file_entries,
        provenance: FarProvenance {
            origin_provider: "conda".to_string(),
            origin_ref: Some(format!(
                "{}:{}",
                info.subdir,
                conda_path.file_name().unwrap_or_default().to_string_lossy()
            )),
            origin_hash: None,
            created_at: now_iso8601(),
            generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
        },
        kernel_contracts: None,
    };

    // 6. Encode
    let chunk_pairs: Vec<([u8; 32], Vec<u8>)> =
        chunks.into_iter().map(|c| (c.hash, c.data)).collect();
    let file = std::fs::File::create(far_output)
        .map_err(|e| format!("create {}: {e}", far_output.display()))?;
    let writer = std::io::BufWriter::new(file);
    encode_far(&manifest, &chunk_pairs, writer)?;

    // Remove the extraction staging directory
    let _ = std::fs::remove_dir_all(&tmp);

    Ok(manifest)
}

// --- internal helpers ---

fn extract_tar_bytes(tar_data: &[u8], output_dir: &Path) -> Result<Vec<CondaFileEntry>, String> {
    let mut archive = tar::Archive::new(tar_data);
    let mut files = Vec::new();

    for entry in archive.entries().map_err(|e| format!("tar entries: {e}"))? {
        let mut entry = entry.map_err(|e| format!("tar entry: {e}"))?;
        let path_buf = entry
            .path()
            .map_err(|e| format!("path: {e}"))?
            .to_path_buf();
        let rel = path_buf.to_string_lossy().to_string();
        let size = entry.size();

        let dest = output_dir.join(&rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
        if entry.header().entry_type().is_file() {
            let mut out = std::fs::File::create(&dest)
                .map_err(|e| format!("create {}: {e}", dest.display()))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("write {}: {e}", dest.display()))?;
            files.push(CondaFileEntry { path: rel, size });
        }
    }
    Ok(files)
}

fn find_index_in_tar(tar_data: &[u8]) -> Result<Option<CondaPackageInfo>, String> {
    let mut archive = tar::Archive::new(tar_data);
    for entry in archive.entries().map_err(|e| format!("tar entries: {e}"))? {
        let mut entry = entry.map_err(|e| format!("tar entry: {e}"))?;
        let path_buf = entry
            .path()
            .map_err(|e| format!("path: {e}"))?
            .to_path_buf();
        let rel = path_buf.to_string_lossy().to_string();
        if rel == "index.json" || rel.ends_with("/index.json") {
            let mut content = String::new();
            entry
                .read_to_string(&mut content)
                .map_err(|e| format!("read index.json: {e}"))?;
            return Ok(Some(parse_conda_index(&content)?));
        }
    }
    Ok(None)
}
