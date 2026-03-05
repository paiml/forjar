//! FJ-1301: Store metadata with provenance tracking.
//!
//! Each store entry has a `meta.yaml` recording its recipe hash, input hashes,
//! architecture, provider, creation time, and provenance chain.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Metadata for a content-addressed store entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoreMeta {
    /// Schema version
    pub schema: String,

    /// BLAKE3 store hash (content address)
    pub store_hash: String,

    /// BLAKE3 hash of the recipe that produced this entry
    pub recipe_hash: String,

    /// Sorted input hashes that contributed to this entry
    pub input_hashes: Vec<String>,

    /// Target architecture (e.g., "x86_64", "aarch64")
    pub arch: String,

    /// Package provider (e.g., "apt", "cargo")
    pub provider: String,

    /// ISO 8601 creation timestamp
    pub created_at: String,

    /// Generator string (e.g., "forjar 1.0.0")
    pub generator: String,

    /// Store hashes referenced by this entry's outputs
    #[serde(default)]
    pub references: Vec<String>,

    /// Optional provenance chain
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

/// Provenance chain — tracks where a store entry came from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Provenance {
    /// Original provider (e.g., "apt", "cargo", "nix")
    pub origin_provider: String,

    /// Upstream reference (e.g., git URL, registry name)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_ref: Option<String>,

    /// Upstream hash / commit
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_hash: Option<String>,

    /// Store hash this was derived from (for multi-step builds)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<String>,

    /// Number of derivation steps from the original source
    #[serde(default)]
    pub derivation_depth: u32,
}

/// Create a new `StoreMeta` with required fields.
pub fn new_meta(
    store_hash: &str,
    recipe_hash: &str,
    input_hashes: &[String],
    arch: &str,
    provider: &str,
) -> StoreMeta {
    use crate::tripwire::eventlog::now_iso8601;
    StoreMeta {
        schema: "1.0".to_string(),
        store_hash: store_hash.to_string(),
        recipe_hash: recipe_hash.to_string(),
        input_hashes: input_hashes.to_vec(),
        arch: arch.to_string(),
        provider: provider.to_string(),
        created_at: now_iso8601(),
        generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
        references: Vec::new(),
        provenance: None,
    }
}

/// Write store metadata atomically (temp file + rename).
pub fn write_meta(dir: &Path, meta: &StoreMeta) -> Result<(), String> {
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create dir {}: {}", parent.display(), e))?;
    }
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("cannot create dir {}: {}", dir.display(), e))?;

    let path = dir.join("meta.yaml");
    let yaml = serde_yaml_ng::to_string(meta).map_err(|e| format!("serialize meta error: {e}"))?;

    let tmp_path = path.with_extension("yaml.tmp");
    std::fs::write(&tmp_path, &yaml)
        .map_err(|e| format!("cannot write {}: {}", tmp_path.display(), e))?;
    std::fs::rename(&tmp_path, &path).map_err(|e| {
        format!(
            "cannot rename {} → {}: {}",
            tmp_path.display(),
            path.display(),
            e
        )
    })?;
    Ok(())
}

/// Read store metadata from a directory.
pub fn read_meta(dir: &Path) -> Result<StoreMeta, String> {
    let path = dir.join("meta.yaml");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("invalid meta.yaml {}: {}", path.display(), e))
}
