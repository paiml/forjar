//! FJ-1345: Upstream diff and sync model.
//!
//! Provenance enables diffing store entries against their upstream origin.
//! `meta.yaml` records `origin_provider`, `origin_ref`, `origin_hash` for
//! traceability. Diff detects upstream changes; sync re-imports and replays
//! derivation chains.

use super::meta::StoreMeta;
use serde::{Deserialize, Serialize};

/// Result of diffing a store entry against its upstream origin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffResult {
    /// Store hash of the local entry
    pub store_hash: String,

    /// Whether the upstream has changed
    pub upstream_changed: bool,

    /// Local origin hash (from meta.yaml)
    pub local_origin_hash: Option<String>,

    /// Current upstream hash (from re-invocation)
    pub upstream_hash: Option<String>,

    /// Origin provider
    pub provider: String,

    /// Origin reference
    pub origin_ref: Option<String>,

    /// Steps in derivation chain that need replay
    pub derivation_chain_depth: u32,
}

/// Plan for syncing a store entry with upstream changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncPlan {
    /// Store entries that need re-import (leaf nodes)
    pub re_imports: Vec<ReImportStep>,

    /// Derivations to replay after re-import
    pub derivation_replays: Vec<DerivationReplayStep>,

    /// Total steps in the sync plan
    pub total_steps: usize,
}

/// A re-import step: re-invoke the origin provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReImportStep {
    pub store_hash: String,
    pub provider: String,
    pub origin_ref: String,
}

/// A derivation replay step: re-derive with updated inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivationReplayStep {
    pub store_hash: String,
    pub derived_from: String,
    pub derivation_depth: u32,
}

/// Compute diff between a store entry and its upstream.
///
/// The `upstream_hash` parameter comes from re-invoking the origin provider
/// and hashing the current upstream output.
pub fn compute_diff(meta: &StoreMeta, upstream_hash: Option<&str>) -> DiffResult {
    let provenance = meta.provenance.as_ref();
    let local_origin = provenance.and_then(|p| p.origin_hash.clone());
    let provider = provenance.map_or("unknown".to_string(), |p| p.origin_provider.clone());
    let origin_ref = provenance.and_then(|p| p.origin_ref.clone());
    let depth = provenance.map_or(0, |p| p.derivation_depth);

    let changed = match (&local_origin, upstream_hash) {
        (Some(local), Some(upstream)) => local != upstream,
        (None, Some(_)) => true,
        _ => false,
    };

    DiffResult {
        store_hash: meta.store_hash.clone(),
        upstream_changed: changed,
        local_origin_hash: local_origin,
        upstream_hash: upstream_hash.map(|s| s.to_string()),
        provider,
        origin_ref,
        derivation_chain_depth: depth,
    }
}

/// Build a sync plan for entries with upstream changes.
///
/// Walks the derivation chain bottom-up: re-import leaf nodes first,
/// then replay each derivation step.
pub fn build_sync_plan(entries: &[(StoreMeta, Option<String>)]) -> SyncPlan {
    let mut re_imports = Vec::new();
    let mut replays = Vec::new();

    for (meta, upstream_hash) in entries {
        let diff = compute_diff(meta, upstream_hash.as_deref());
        if !diff.upstream_changed {
            continue;
        }

        let provenance = meta.provenance.as_ref();
        let depth = provenance.map_or(0, |p| p.derivation_depth);

        if depth == 0 {
            // Direct import — re-invoke provider
            re_imports.push(ReImportStep {
                store_hash: meta.store_hash.clone(),
                provider: diff.provider.clone(),
                origin_ref: diff.origin_ref.unwrap_or_default(),
            });
        } else {
            // Derived entry — replay derivation
            let derived_from = provenance
                .and_then(|p| p.derived_from.clone())
                .unwrap_or_default();
            replays.push(DerivationReplayStep {
                store_hash: meta.store_hash.clone(),
                derived_from,
                derivation_depth: depth,
            });
        }
    }

    // Sort replays by depth (bottom-up)
    replays.sort_by_key(|r| r.derivation_depth);

    let total = re_imports.len() + replays.len();
    SyncPlan {
        re_imports,
        derivation_replays: replays,
        total_steps: total,
    }
}

/// Check if a store entry has provenance metadata for diffing.
pub fn has_diffable_provenance(meta: &StoreMeta) -> bool {
    meta.provenance
        .as_ref()
        .is_some_and(|p| p.origin_hash.is_some() || p.origin_ref.is_some())
}

/// Generate the CLI command to re-check upstream for a store entry.
pub fn upstream_check_command(meta: &StoreMeta) -> Option<String> {
    let prov = meta.provenance.as_ref()?;
    let origin_ref = prov.origin_ref.as_ref()?;
    match prov.origin_provider.as_str() {
        "apt" => Some(format!("apt-cache policy {origin_ref}")),
        "cargo" => Some(format!("cargo search {origin_ref}")),
        "nix" => Some(format!("nix flake metadata {origin_ref}")),
        "docker" => Some(format!("docker manifest inspect {origin_ref}")),
        "tofu" | "terraform" => Some(format!("tofu plan -refresh-only {origin_ref}")),
        "apr" => Some(format!("apr info {origin_ref}")),
        _ => None,
    }
}
