//! FJ-1362: Store diff/sync execution.
//!
//! Bridges `store_diff` types → actual provider re-invocation via transport.
//! Executes upstream checks, computes live diffs, and runs sync plans
//! (re-import leaf nodes, replay derivation chains).

use super::meta::read_meta;
use super::meta::StoreMeta;
use super::provider::{ImportConfig, ImportProvider, ImportResult};
use super::provider_exec::ExecutionContext;
use super::store_diff::{compute_diff, upstream_check_command, DiffResult, SyncPlan};
use crate::core::types::Machine;
use crate::transport;
use std::path::Path;

/// Result of executing a diff with live upstream query.
#[derive(Debug, Clone)]
pub struct DiffExecResult {
    /// The computed diff
    pub diff: DiffResult,
    /// The upstream check command that was executed
    pub upstream_command: Option<String>,
    /// Raw stdout from the upstream check
    pub upstream_output: Option<String>,
}

/// Result of executing a sync plan.
#[derive(Debug, Clone)]
pub struct SyncExecResult {
    /// Store entries that were re-imported
    pub re_imported: Vec<ImportResult>,
    /// Number of derivation chains replayed
    pub derivations_replayed: usize,
    /// New profile hash (if profile was updated)
    pub new_profile_hash: Option<String>,
}

/// Execute a live diff: re-invoke the upstream provider and compute hash.
///
/// 1. Read meta.yaml for the store entry
/// 2. Generate the upstream check command
/// 3. Execute via transport
/// 4. Hash the upstream output
/// 5. Compare with stored origin_hash
pub fn execute_diff(
    meta: &StoreMeta,
    machine: &Machine,
    _store_dir: &Path,
    timeout_secs: Option<u64>,
) -> Result<DiffExecResult, String> {
    let cmd = upstream_check_command(meta);

    let upstream_hash = match &cmd {
        Some(check_cmd) => {
            let output = transport::exec_script_timeout(machine, check_cmd, timeout_secs)
                .map_err(|e| format!("upstream check failed: {e}"))?;

            if output.success() && !output.stdout.trim().is_empty() {
                let hash = blake3::hash(output.stdout.as_bytes());
                Some(format!("blake3:{}", hash.to_hex()))
            } else {
                None
            }
        }
        None => None,
    };

    let upstream_output = match &cmd {
        Some(check_cmd) => transport::exec_script_timeout(machine, check_cmd, timeout_secs)
            .ok()
            .map(|o| o.stdout),
        None => None,
    };

    let diff = compute_diff(meta, upstream_hash.as_deref());

    Ok(DiffExecResult {
        diff,
        upstream_command: cmd,
        upstream_output,
    })
}

/// Execute a sync plan: re-import changed leaf nodes and replay derivations.
///
/// 1. Re-import each leaf node via provider execution
/// 2. Track derivation replay count (actual replay delegated to sandbox_run)
/// 3. Return overall result
pub fn execute_sync(
    plan: &SyncPlan,
    machine: &Machine,
    store_dir: &Path,
    timeout_secs: Option<u64>,
) -> Result<SyncExecResult, String> {
    let mut re_imported = Vec::new();

    // Re-import leaf nodes
    for step in &plan.re_imports {
        let provider = parse_provider(&step.provider)?;
        let config = ImportConfig {
            provider,
            reference: step.origin_ref.clone(),
            version: None,
            arch: machine.arch.clone(),
            options: std::collections::BTreeMap::new(),
        };

        let staging_dir = tempdir_for_reimport(&step.store_hash);
        let ctx = ExecutionContext {
            store_dir: store_dir.to_path_buf(),
            staging_dir,
            machine: machine.clone(),
            timeout_secs,
        };

        match super::provider_exec::execute_import(&config, &ctx) {
            Ok(result) => re_imported.push(result),
            Err(e) => {
                return Err(format!(
                    "re-import {} via {}: {e}",
                    step.origin_ref, step.provider
                ));
            }
        }
    }

    let derivations_replayed = plan.derivation_replays.len();

    Ok(SyncExecResult {
        re_imported,
        derivations_replayed,
        new_profile_hash: None,
    })
}

/// Diff all entries in a store directory that have provenance metadata.
pub fn diff_all_entries(
    store_dir: &Path,
    machine: &Machine,
    timeout_secs: Option<u64>,
) -> Result<Vec<DiffExecResult>, String> {
    let entries = std::fs::read_dir(store_dir).map_err(|e| format!("read store dir: {e}"))?;

    let mut results = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        if let Ok(meta) = read_meta(&path) {
            if super::store_diff::has_diffable_provenance(&meta) {
                match execute_diff(&meta, machine, store_dir, timeout_secs) {
                    Ok(result) => results.push(result),
                    Err(_) => continue,
                }
            }
        }
    }

    Ok(results)
}

/// Parse a provider string into an ImportProvider enum.
fn parse_provider(s: &str) -> Result<ImportProvider, String> {
    match s {
        "apt" => Ok(ImportProvider::Apt),
        "cargo" => Ok(ImportProvider::Cargo),
        "uv" => Ok(ImportProvider::Uv),
        "nix" => Ok(ImportProvider::Nix),
        "docker" => Ok(ImportProvider::Docker),
        "tofu" => Ok(ImportProvider::Tofu),
        "terraform" => Ok(ImportProvider::Terraform),
        "apr" => Ok(ImportProvider::Apr),
        other => Err(format!("unknown provider: {other}")),
    }
}

/// Create a temporary directory for re-import staging.
fn tempdir_for_reimport(store_hash: &str) -> std::path::PathBuf {
    let hash_bare = store_hash.strip_prefix("blake3:").unwrap_or(store_hash);
    let short = &hash_bare[..16.min(hash_bare.len())];
    std::path::PathBuf::from(format!("/tmp/forjar-reimport-{short}"))
}
