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
///
/// GH-249: `derivations_planned` and `derivations_replayed` are separate fields
/// on purpose. They were one field that held the PLAN's length, so a caller
/// could not tell "3 replayed" from "3 planned, 0 executed" — and 0 executed was
/// always the truth. This is the same planned-vs-actual split
/// `apply-summary-distinguishability-v1` already draws for `apply`.
#[derive(Debug, Clone)]
pub struct SyncExecResult {
    /// Store entries that were re-imported
    pub re_imported: Vec<ImportResult>,
    /// Number of derivation chains the plan called for.
    pub derivations_planned: usize,
    /// Number of derivation chains actually replayed.
    ///
    /// Currently always 0 — see [`execute_sync`] for why replay cannot yet be
    /// driven from a [`SyncPlan`]. Read [`SyncExecResult::is_complete`] rather
    /// than assuming a returned `Ok` means the store is in sync.
    pub derivations_replayed: usize,
    /// New profile hash (if profile was updated)
    pub new_profile_hash: Option<String>,
}

impl SyncExecResult {
    /// Whether the plan was carried out in full.
    ///
    /// `execute_sync` returns `Ok` when the re-imports it *can* do succeeded.
    /// That is not the same as the store being in sync: any planned derivation
    /// chain that was not replayed leaves derived entries stale. Callers that
    /// report success to an operator MUST gate on this.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.derivations_replayed >= self.derivations_planned
    }
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

/// Execute a sync plan: re-import changed leaf nodes.
///
/// 1. Re-import each leaf node via provider execution
/// 2. Report planned vs replayed derivation chains — see below
///
/// # Derivations are NOT replayed here
///
/// GH-249. A [`SyncPlan`]'s `derivation_replays` are
/// [`DerivationReplayStep`](super::store_diff::DerivationReplayStep)s, which
/// carry `store_hash`, `derived_from` and `derivation_depth` — the *fact* that
/// an entry is derived. They do not carry the derivation itself (builder, args,
/// env), so there is nothing in a plan that could drive
/// [`execute_derivation_dag_live`](super::derivation_exec::execute_derivation_dag_live),
/// which in turn has no caller under `src/cli/` or `src/core/executor/`.
///
/// This function previously reported `plan.derivation_replays.len()` as the
/// number replayed. That is intent reported as outcome: `forjar store sync
/// --apply` printed "Derivations replayed: 1" over an entry it had not touched,
/// and exited 0, so an operator read a stale store as fresh.
///
/// Wiring real replay requires the plan to carry recipes, which is a store
/// meta-format change. Until then the count is 0 and
/// [`SyncExecResult::is_complete`] reports the shortfall.
///
/// # Errors
///
/// Returns `Err` if any leaf re-import fails. A returned `Ok` means the
/// re-imports succeeded — NOT that the plan completed; check `is_complete`.
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

    Ok(SyncExecResult {
        re_imported,
        derivations_planned: plan.derivation_replays.len(),
        // Not a placeholder to be filled in later by whoever reads this: it is
        // the measured truth. Nothing in this function replays a derivation.
        derivations_replayed: 0,
        new_profile_hash: None,
    })
}

/// Collect store subdirectories that have diffable provenance metadata.
fn collect_diffable_entries(store_dir: &Path) -> Result<Vec<StoreMeta>, String> {
    let entries = std::fs::read_dir(store_dir).map_err(|e| format!("read store dir: {e}"))?;
    let mut metas = Vec::new();
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
                metas.push(meta);
            }
        }
    }
    Ok(metas)
}

/// Diff all entries in a store directory that have provenance metadata.
pub fn diff_all_entries(
    store_dir: &Path,
    machine: &Machine,
    timeout_secs: Option<u64>,
) -> Result<Vec<DiffExecResult>, String> {
    let metas = collect_diffable_entries(store_dir)?;
    let mut results = Vec::new();
    for meta in &metas {
        if let Ok(result) = execute_diff(meta, machine, store_dir, timeout_secs) {
            results.push(result);
        }
    }
    Ok(results)
}

/// Parse a provider string into an ImportProvider enum.
pub fn parse_provider(s: &str) -> Result<ImportProvider, String> {
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
pub fn tempdir_for_reimport(store_hash: &str) -> std::path::PathBuf {
    let hash_bare = store_hash.strip_prefix("blake3:").unwrap_or(store_hash);
    let short = &hash_bare[..16.min(hash_bare.len())];
    std::path::PathBuf::from(format!("/tmp/forjar-reimport-{short}"))
}
