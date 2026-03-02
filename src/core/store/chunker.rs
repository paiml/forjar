//! FJ-1347: Fixed-size chunking with BLAKE3 per-chunk hashing.
//!
//! Splits data into 64KB chunks, hashes each with BLAKE3, and computes
//! a binary Merkle tree hash for verified streaming.

use super::far::FarFileEntry;
use std::path::Path;

/// Default chunk size: 64KB per spec.
pub const CHUNK_SIZE: usize = 65536;

/// A chunk of data with its BLAKE3 hash.
#[derive(Debug, Clone)]
pub struct ChunkData {
    pub hash: [u8; 32],
    pub data: Vec<u8>,
}

/// Split bytes into fixed-size chunks, each BLAKE3-hashed.
pub fn chunk_bytes(data: &[u8]) -> Vec<ChunkData> {
    data.chunks(CHUNK_SIZE)
        .map(|slice| {
            let hash = *blake3::hash(slice).as_bytes();
            ChunkData {
                hash,
                data: slice.to_vec(),
            }
        })
        .collect()
}

/// Walk a directory, tar it into memory, then chunk the result.
/// Returns (chunks, file_entries) for embedding in a FAR manifest.
pub fn chunk_directory(dir: &Path) -> Result<(Vec<ChunkData>, Vec<FarFileEntry>), String> {
    let mut entries = Vec::new();
    walk_files(dir, dir, &mut entries)?;
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // Build file entry list
    let far_entries: Vec<FarFileEntry> = entries
        .iter()
        .map(|(rel, size, hash)| FarFileEntry {
            path: rel.clone(),
            size: *size,
            blake3: format!("blake3:{hash}"),
        })
        .collect();

    // Tar into memory
    let tar_bytes = tar_directory(dir, &entries)?;
    let chunks = chunk_bytes(&tar_bytes);

    Ok((chunks, far_entries))
}

/// Compute a binary Merkle tree hash over chunk hashes.
/// Leaf: BLAKE3(chunk). Node: BLAKE3(left || right). Odd leaf promoted.
pub fn tree_hash(chunks: &[ChunkData]) -> [u8; 32] {
    if chunks.is_empty() {
        return *blake3::hash(b"").as_bytes();
    }
    let mut level: Vec<[u8; 32]> = chunks.iter().map(|c| c.hash).collect();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            if pair.len() == 2 {
                let mut hasher = blake3::Hasher::new();
                hasher.update(&pair[0]);
                hasher.update(&pair[1]);
                next.push(*hasher.finalize().as_bytes());
            } else {
                next.push(pair[0]);
            }
        }
        level = next;
    }
    level[0]
}

/// Reassemble original data from chunks.
pub fn reassemble(chunks: &[ChunkData]) -> Vec<u8> {
    let total: usize = chunks.iter().map(|c| c.data.len()).sum();
    let mut out = Vec::with_capacity(total);
    for c in chunks {
        out.extend_from_slice(&c.data);
    }
    out
}

// --- internal helpers ---

fn walk_files(
    base: &Path,
    current: &Path,
    entries: &mut Vec<(String, u64, String)>,
) -> Result<(), String> {
    let read_dir = std::fs::read_dir(current)
        .map_err(|e| format!("cannot read dir {}: {e}", current.display()))?;
    let mut children: Vec<std::fs::DirEntry> = read_dir.filter_map(|e| e.ok()).collect();
    children.sort_by_key(|e| e.file_name());

    for entry in children {
        let ft = entry.file_type().map_err(|e| format!("stat: {e}"))?;
        if ft.is_symlink() {
            continue;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(base)
            .map_err(|e| format!("prefix: {e}"))?
            .to_string_lossy()
            .to_string();
        if ft.is_file() {
            let meta = std::fs::metadata(&path)
                .map_err(|e| format!("metadata {}: {e}", path.display()))?;
            let hash = blake3::hash(
                &std::fs::read(&path)
                    .map_err(|e| format!("read {}: {e}", path.display()))?,
            )
            .to_hex()
            .to_string();
            entries.push((rel, meta.len(), hash));
        } else if ft.is_dir() {
            walk_files(base, &path, entries)?;
        }
    }
    Ok(())
}

fn tar_directory(
    base: &Path,
    entries: &[(String, u64, String)],
) -> Result<Vec<u8>, String> {
    let mut builder = tar::Builder::new(Vec::new());
    for (rel, _, _) in entries {
        let full = base.join(rel);
        builder
            .append_path_with_name(&full, rel)
            .map_err(|e| format!("tar {rel}: {e}"))?;
    }
    builder
        .into_inner()
        .map_err(|e| format!("tar finalize: {e}"))
}
