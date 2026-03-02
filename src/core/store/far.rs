//! FJ-1346: FAR (Forjar ARchive) binary format — encode and decode.
//!
//! Layout: magic → manifest_len → zstd(manifest_yaml) → chunk_count
//!       → chunk_table(hash+offset+len) → zstd(chunks) → sig_len → sig

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// 12-byte magic identifying a FAR archive.
pub const FAR_MAGIC: &[u8; 12] = b"FORJAR-FAR\x00\x01";

/// A single chunk entry in the chunk table.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkEntry {
    pub hash: [u8; 32],
    pub offset: u64,
    pub length: u64,
}

/// Manifest embedded in a FAR archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FarManifest {
    pub name: String,
    pub version: String,
    pub arch: String,
    pub store_hash: String,
    pub tree_hash: String,
    pub file_count: u64,
    pub total_size: u64,
    pub files: Vec<FarFileEntry>,
    pub provenance: FarProvenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_contracts: Option<KernelContractInfo>,
}

/// Kernel contract metadata embedded in a FAR manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KernelContractInfo {
    pub model_type: String,
    pub required_ops: Vec<String>,
    pub coverage_pct: f64,
}

/// A file entry within the FAR manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FarFileEntry {
    pub path: String,
    pub size: u64,
    pub blake3: String,
}

/// Provenance metadata for the FAR archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FarProvenance {
    pub origin_provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_hash: Option<String>,
    pub created_at: String,
    pub generator: String,
}

/// Encode a FAR archive to a writer.
///
/// Writes: magic → manifest_len(u64) → zstd(manifest_yaml)
///       → chunk_count(u64) → chunk_table → zstd(chunks)
///       → signature_len(u64=0)
pub fn encode_far<W: Write>(
    manifest: &FarManifest,
    chunks: &[([u8; 32], Vec<u8>)],
    mut writer: W,
) -> Result<(), String> {
    // Magic
    writer
        .write_all(FAR_MAGIC)
        .map_err(|e| format!("write magic: {e}"))?;

    // Manifest → YAML → zstd
    let yaml =
        serde_yaml_ng::to_string(manifest).map_err(|e| format!("serialize manifest: {e}"))?;
    let compressed =
        zstd::encode_all(yaml.as_bytes(), 3).map_err(|e| format!("zstd manifest: {e}"))?;

    writer
        .write_all(&(compressed.len() as u64).to_le_bytes())
        .map_err(|e| format!("write manifest_len: {e}"))?;
    writer
        .write_all(&compressed)
        .map_err(|e| format!("write manifest: {e}"))?;

    // Chunk count
    let chunk_count = chunks.len() as u64;
    writer
        .write_all(&chunk_count.to_le_bytes())
        .map_err(|e| format!("write chunk_count: {e}"))?;

    // Compress all chunks and build table
    let mut compressed_chunks: Vec<Vec<u8>> = Vec::with_capacity(chunks.len());
    for (_, data) in chunks {
        let cc = zstd::encode_all(data.as_slice(), 3).map_err(|e| format!("zstd chunk: {e}"))?;
        compressed_chunks.push(cc);
    }

    // Chunk table: hash(32) + offset(u64) + length(u64) per entry
    let mut offset: u64 = 0;
    for (i, (hash, _)) in chunks.iter().enumerate() {
        let len = compressed_chunks[i].len() as u64;
        writer
            .write_all(hash)
            .map_err(|e| format!("write chunk hash: {e}"))?;
        writer
            .write_all(&offset.to_le_bytes())
            .map_err(|e| format!("write chunk offset: {e}"))?;
        writer
            .write_all(&len.to_le_bytes())
            .map_err(|e| format!("write chunk length: {e}"))?;
        offset += len;
    }

    // Chunk data
    for cc in &compressed_chunks {
        writer
            .write_all(cc)
            .map_err(|e| format!("write chunk data: {e}"))?;
    }

    // Signature (0 = unsigned)
    writer
        .write_all(&0u64.to_le_bytes())
        .map_err(|e| format!("write sig_len: {e}"))?;

    writer.flush().map_err(|e| format!("flush: {e}"))?;
    Ok(())
}

/// Decode the manifest and chunk table from a FAR archive (streaming — no full load).
pub fn decode_far_manifest<R: Read>(
    mut reader: R,
) -> Result<(FarManifest, Vec<ChunkEntry>), String> {
    // Magic
    let mut magic = [0u8; 12];
    reader
        .read_exact(&mut magic)
        .map_err(|e| format!("read magic: {e}"))?;
    if magic != *FAR_MAGIC {
        return Err("invalid FAR magic".to_string());
    }

    // Manifest length
    let mut len_buf = [0u8; 8];
    reader
        .read_exact(&mut len_buf)
        .map_err(|e| format!("read manifest_len: {e}"))?;
    let manifest_len = u64::from_le_bytes(len_buf) as usize;

    // Compressed manifest
    let mut compressed = vec![0u8; manifest_len];
    reader
        .read_exact(&mut compressed)
        .map_err(|e| format!("read manifest: {e}"))?;
    let yaml_bytes =
        zstd::decode_all(compressed.as_slice()).map_err(|e| format!("zstd decompress: {e}"))?;
    let manifest: FarManifest =
        serde_yaml_ng::from_slice(&yaml_bytes).map_err(|e| format!("parse manifest: {e}"))?;

    // Chunk count
    reader
        .read_exact(&mut len_buf)
        .map_err(|e| format!("read chunk_count: {e}"))?;
    let chunk_count = u64::from_le_bytes(len_buf);

    // Chunk table
    let mut entries = Vec::with_capacity(chunk_count as usize);
    for _ in 0..chunk_count {
        let mut hash = [0u8; 32];
        reader
            .read_exact(&mut hash)
            .map_err(|e| format!("read chunk hash: {e}"))?;
        reader
            .read_exact(&mut len_buf)
            .map_err(|e| format!("read chunk offset: {e}"))?;
        let offset = u64::from_le_bytes(len_buf);
        reader
            .read_exact(&mut len_buf)
            .map_err(|e| format!("read chunk length: {e}"))?;
        let length = u64::from_le_bytes(len_buf);
        entries.push(ChunkEntry {
            hash,
            offset,
            length,
        });
    }

    Ok((manifest, entries))
}
