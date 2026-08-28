//! FJ-1301: Store metadata with provenance tracking.
//!
//! Each store entry has a `meta.yaml` recording its recipe hash, input hashes,
//! architecture, provider, creation time, and provenance chain.

use super::content;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Schema version written by this build.
///
/// GH-236: 1.1 adds `output_hash` and `addressing`. Both are `#[serde(default)]`,
/// so a 1.0 `meta.yaml` already on disk still loads — it reports
/// `output_hash: None`, which [`super::verify`] renders as `Unsealed` rather
/// than as a false corruption report.
pub const SCHEMA_VERSION: &str = "1.1";

/// Which scheme an entry's `store_hash` was derived from.
///
/// The store has always carried both kinds of address and never said which was
/// which: the derivation path addresses by recipe + inputs
/// (`path::store_path`), the import path addresses by the bytes it staged
/// (`provider_exec::hash_staging_dir`). Downstream code was left to guess, and
/// guessed wrong — see the note on [`super::verify`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Addressing {
    /// `store_hash` is a hash of the recipe and its inputs.
    #[default]
    Derivation,
    /// `store_hash` is a hash of the bytes the entry holds.
    Content,
}

/// Metadata for a content-addressed store entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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

    /// GH-236: BLAKE3 over the entry's `content/` tree, computed after the
    /// artifact lands. This is the digest of what the entry HOLDS, as opposed
    /// to the derivation that asked for it — the only thing corruption
    /// detection and output dedup can be built on. `None` for entries written
    /// before schema 1.1, which are reported as unsealed, never as corrupt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_hash: Option<String>,

    /// GH-236: which scheme `store_hash` was derived from.
    #[serde(default)]
    pub addressing: Addressing,
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
        schema: SCHEMA_VERSION.to_string(),
        store_hash: store_hash.to_string(),
        recipe_hash: recipe_hash.to_string(),
        input_hashes: input_hashes.to_vec(),
        arch: arch.to_string(),
        provider: provider.to_string(),
        created_at: now_iso8601(),
        generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
        references: Vec::new(),
        provenance: None,
        output_hash: None,
        addressing: Addressing::Derivation,
    }
}

/// GH-236: record the digest of the bytes this entry now holds.
///
/// Call AFTER the artifact has landed under `<entry>/content/`, then
/// `write_meta`. Sealing before the content is in place would record the digest
/// of a half-written tree, which is worse than recording nothing.
pub fn seal_output(
    entry_dir: &Path,
    meta: &mut StoreMeta,
    addressing: Addressing,
) -> Result<(), String> {
    meta.output_hash = Some(content::content_hash(&entry_dir.join("content"))?);
    meta.addressing = addressing;
    Ok(())
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
