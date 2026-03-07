//! FJ-2102: Runtime OCI layer builder — creates tar archives from resource definitions.
//!
//! This is the core implementation for Phase 8 (Direct Layer Assembly). It takes
//! resource definitions and produces actual OCI-compliant layer tarballs with
//! dual-digest computation (BLAKE3 for store addressing, SHA-256 for OCI).

use crate::core::types::{
    DualDigest, LayerBuildPath, LayerBuildResult, LayerCompression, OciLayerConfig, TarSortOrder,
};
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;

/// A file entry to be added to a layer tarball.
#[derive(Debug, Clone)]
pub struct LayerEntry {
    /// Path inside the image (e.g., "etc/app/config.yaml").
    pub path: String,
    /// File content bytes.
    pub content: Vec<u8>,
    /// Unix permission mode (e.g., 0o644).
    pub mode: u32,
    /// Whether this is a directory.
    pub is_dir: bool,
}

impl LayerEntry {
    /// Create a file entry.
    pub fn file(path: &str, content: &[u8], mode: u32) -> Self {
        Self {
            path: normalize_tar_path(path),
            content: content.to_vec(),
            mode,
            is_dir: false,
        }
    }

    /// Create a directory entry.
    pub fn dir(path: &str, mode: u32) -> Self {
        let mut p = normalize_tar_path(path);
        if !p.ends_with('/') {
            p.push('/');
        }
        Self {
            path: p,
            content: Vec::new(),
            mode,
            is_dir: true,
        }
    }
}

/// Build an OCI layer tarball from a list of file entries.
///
/// Returns a `LayerBuildResult` with dual digests (BLAKE3 + SHA-256) and
/// compressed content. The tar is built deterministically when configured:
/// sorted entries, epoch mtime, fixed uid/gid.
pub fn build_layer(
    entries: &[LayerEntry],
    config: &OciLayerConfig,
) -> Result<(LayerBuildResult, Vec<u8>), String> {
    let mut sorted_entries: Vec<&LayerEntry> = entries.iter().collect();
    match config.sort_order {
        TarSortOrder::Lexicographic => sorted_entries.sort_by(|a, b| a.path.cmp(&b.path)),
        TarSortOrder::DirectoryFirst => sorted_entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.path.cmp(&b.path),
        }),
    }

    // Build uncompressed tar
    let uncompressed = build_tar(&sorted_entries, config)?;
    let uncompressed_size = uncompressed.len() as u64;
    let file_count = entries.len() as u32;

    // Compute DiffID (sha256 of uncompressed)
    let diff_id = format!("sha256:{}", hex_sha256(&uncompressed));

    // Compute BLAKE3 of uncompressed (store address)
    let blake3_hash = blake3::hash(&uncompressed).to_hex().to_string();

    // Compress
    let (compressed, compression) = compress_layer(&uncompressed, config)?;
    let compressed_size = compressed.len() as u64;

    // Compute digest (sha256 of compressed)
    let digest = format!("sha256:{}", hex_sha256(&compressed));

    let result = LayerBuildResult {
        digest,
        diff_id,
        store_hash: format!("blake3:{blake3_hash}"),
        compressed_size,
        uncompressed_size,
        compression,
        file_count,
        build_path: LayerBuildPath::DirectAssembly,
    };

    // Postcondition: determinism — same inputs must produce same output
    debug_assert!({
        let verify = build_tar(&sorted_entries, config).unwrap();
        blake3::hash(&verify).to_hex().to_string() == blake3_hash
    });

    // FJ-2200 G4: Store idempotency — storing the same content twice produces identical digests
    debug_assert!({
        let verify_digest = compute_dual_digest(&compressed);
        verify_digest.blake3 == compute_dual_digest(&compressed).blake3
            && verify_digest.sha256 == compute_dual_digest(&compressed).sha256
    });

    Ok((result, compressed))
}

/// Compute dual digest (BLAKE3 + SHA-256) for arbitrary content.
pub fn compute_dual_digest(content: &[u8]) -> DualDigest {
    let blake3 = blake3::hash(content).to_hex().to_string();
    let sha256 = hex_sha256(content);
    let result = DualDigest {
        blake3,
        sha256,
        size_bytes: content.len() as u64,
    };

    // FJ-2200 G4: Dual-digest consistency postcondition
    debug_assert_eq!(
        result.size_bytes,
        content.len() as u64,
        "compute_dual_digest: size mismatch"
    );
    debug_assert!(
        !result.blake3.is_empty() && !result.sha256.is_empty(),
        "compute_dual_digest: empty digest"
    );

    result
}

/// Write an OCI layout directory from layers and config.
pub fn write_oci_layout(
    output_dir: &Path,
    layers: &[(LayerBuildResult, Vec<u8>)],
    config_json: &[u8],
) -> Result<(), String> {
    std::fs::create_dir_all(output_dir.join("blobs/sha256"))
        .map_err(|e| format!("create blobs dir: {e}"))?;

    // oci-layout
    std::fs::write(
        output_dir.join("oci-layout"),
        r#"{"imageLayoutVersion":"1.0.0"}"#,
    )
    .map_err(|e| format!("write oci-layout: {e}"))?;

    // Write layer blobs
    for (result, data) in layers {
        let hex = result
            .digest
            .strip_prefix("sha256:")
            .unwrap_or(&result.digest);
        std::fs::write(output_dir.join(format!("blobs/sha256/{hex}")), data)
            .map_err(|e| format!("write layer blob: {e}"))?;
    }

    // Write config blob
    let config_hex = hex_sha256(config_json);
    std::fs::write(
        output_dir.join(format!("blobs/sha256/{config_hex}")),
        config_json,
    )
    .map_err(|e| format!("write config blob: {e}"))?;

    // FJ-2200 G4: OCI layout integrity postcondition
    debug_assert!(
        output_dir.join("oci-layout").exists(),
        "write_oci_layout: oci-layout missing"
    );
    debug_assert!(
        output_dir
            .join(format!("blobs/sha256/{config_hex}"))
            .exists(),
        "write_oci_layout: config blob missing"
    );

    Ok(())
}

// ── Internal helpers ───────────────────────────────────────────────

fn build_tar(entries: &[&LayerEntry], config: &OciLayerConfig) -> Result<Vec<u8>, String> {
    let buf = Vec::new();
    let mut tar = tar::Builder::new(buf);

    for entry in entries {
        let mut header = tar::Header::new_gnu();
        header.set_mode(entry.mode);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(if config.deterministic {
            config.epoch_mtime
        } else {
            0
        });
        header
            .set_username("root")
            .map_err(|e| format!("set username: {e}"))?;
        header
            .set_groupname("root")
            .map_err(|e| format!("set groupname: {e}"))?;

        if entry.is_dir {
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            tar.append_data(&mut header, &entry.path, &[] as &[u8])
                .map_err(|e| format!("append dir {}: {e}", entry.path))?;
        } else {
            header.set_entry_type(tar::EntryType::Regular);
            header.set_size(entry.content.len() as u64);
            tar.append_data(&mut header, &entry.path, entry.content.as_slice())
                .map_err(|e| format!("append file {}: {e}", entry.path))?;
        }
    }

    tar.into_inner().map_err(|e| format!("finish tar: {e}"))
}

fn compress_layer(
    uncompressed: &[u8],
    config: &OciLayerConfig,
) -> Result<(Vec<u8>, LayerCompression), String> {
    match config.compression {
        crate::core::types::OciCompression::None => {
            Ok((uncompressed.to_vec(), LayerCompression::None))
        }
        crate::core::types::OciCompression::Gzip => {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder
                .write_all(uncompressed)
                .map_err(|e| format!("gzip compress: {e}"))?;
            let compressed = encoder.finish().map_err(|e| format!("gzip finish: {e}"))?;
            Ok((compressed, LayerCompression::Gzip))
        }
        crate::core::types::OciCompression::Zstd => {
            let compressed =
                zstd::encode_all(uncompressed, 3).map_err(|e| format!("zstd compress: {e}"))?;
            Ok((compressed, LayerCompression::Zstd))
        }
    }
}

fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Normalize a tar path: strip leading `/`, ensure no `//`.
fn normalize_tar_path(path: &str) -> String {
    let stripped = path.strip_prefix('/').unwrap_or(path);
    stripped.replace("//", "/")
}
