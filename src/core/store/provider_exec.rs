//! FJ-1359: Provider execution bridge.
//!
//! Bridges `provider::import_command()` → `transport::exec_script()` with
//! staging directory lifecycle, BLAKE3 hashing, and atomic store placement.

use super::meta::{write_meta, Provenance, StoreMeta};
use super::provider::{
    import_command, origin_ref_string, validate_import, ImportConfig, ImportResult,
};
use crate::core::purifier;
use crate::core::types::Machine;
use crate::transport;
use crate::tripwire::hasher::composite_hash;
use std::path::{Path, PathBuf};

/// Context for provider execution.
pub struct ExecutionContext {
    /// Root store directory (e.g., /var/lib/forjar/store)
    pub store_dir: PathBuf,
    /// Staging directory for in-flight captures
    pub staging_dir: PathBuf,
    /// Target machine for transport layer
    pub machine: Machine,
    /// Optional timeout for script execution (seconds)
    pub timeout_secs: Option<u64>,
}

/// Execute a provider import: generate CLI, validate, execute, hash, store.
///
/// Steps:
/// 1. Validate import config
/// 2. Generate CLI command via `provider::import_command()`
/// 3. Validate script via `purifier::validate_script()` (I8 invariant)
/// 4. Create staging directory
/// 5. Execute via `transport::exec_script_timeout()`
/// 6. Hash staging output via composite hash
/// 7. Atomic move staging → store/<hash>
/// 8. Write meta.yaml with provenance
pub fn execute_import(
    config: &ImportConfig,
    ctx: &ExecutionContext,
) -> Result<ImportResult, String> {
    // Step 1: Validate config
    let errors = validate_import(config);
    if !errors.is_empty() {
        return Err(format!("import validation failed: {}", errors.join("; ")));
    }

    // Step 2: Generate CLI command
    let cli_command = import_command(config);

    // Step 3: I8 validation gate
    let staging_script = build_staging_script(&cli_command, &ctx.staging_dir);
    purifier::validate_script(&staging_script)
        .map_err(|e| format!("I8 validation failed for import script: {e}"))?;

    // Step 4: Create staging directory
    std::fs::create_dir_all(&ctx.staging_dir).map_err(|e| {
        format!(
            "cannot create staging dir {}: {e}",
            ctx.staging_dir.display()
        )
    })?;

    // Step 5: Execute via transport
    let output = transport::exec_script_timeout(&ctx.machine, &staging_script, ctx.timeout_secs)
        .map_err(|e| {
            cleanup_staging(&ctx.staging_dir);
            format!("provider import execution failed: {e}")
        })?;

    if !output.success() {
        cleanup_staging(&ctx.staging_dir);
        return Err(format!(
            "provider import returned exit code {}: {}",
            output.exit_code,
            output.stderr.trim()
        ));
    }

    // Step 6: Hash staging directory
    let store_hash = hash_staging_dir(&ctx.staging_dir)?;

    // Step 7: Atomic move to store
    let hash_bare = store_hash.strip_prefix("blake3:").unwrap_or(&store_hash);
    let store_entry = ctx.store_dir.join(hash_bare);
    let content_dir = store_entry.join("content");
    atomic_move_to_store(&ctx.staging_dir, &content_dir)?;

    // Step 8: Write meta.yaml
    let (file_count, total_size) = dir_stats(&content_dir);
    let origin_ref = origin_ref_string(config);

    write_import_meta(&store_entry, config, &store_hash, &origin_ref)?;

    Ok(ImportResult {
        store_hash,
        store_path: store_entry.display().to_string(),
        file_count,
        total_size,
        provider: config.provider,
        origin_ref,
        cli_command,
    })
}

/// Build the staging-wrapped import script.
///
/// Sets `$STAGING` to the staging directory so providers like cargo/uv
/// can use it as their output target.
pub fn build_staging_script(cli_command: &str, staging_dir: &Path) -> String {
    format!(
        "export STAGING='{}'\nmkdir -p \"$STAGING\"\n{cli_command}",
        staging_dir.display()
    )
}

/// Hash a staging directory tree (BLAKE3 composite) for store path derivation.
pub fn hash_staging_dir(dir: &Path) -> Result<String, String> {
    let mut components = Vec::new();

    collect_file_hashes(dir, dir, &mut components)?;

    if components.is_empty() {
        return Err("staging directory is empty — nothing to hash".to_string());
    }

    components.sort();
    let refs: Vec<&str> = components.iter().map(|s| s.as_str()).collect();
    Ok(composite_hash(&refs))
}

/// Recursively collect (relative_path + content_hash) components.
fn collect_file_hashes(
    base: &Path,
    dir: &Path,
    components: &mut Vec<String>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_file_hashes(base, &path, components)?;
        } else if path.is_file() {
            let content =
                std::fs::read(&path).map_err(|e| format!("read file {}: {e}", path.display()))?;
            let hash = blake3::hash(&content);
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .display()
                .to_string();
            components.push(format!("{rel}:{}", hash.to_hex()));
        }
    }
    Ok(())
}

/// Write meta.yaml for a newly imported store entry.
fn write_import_meta(
    store_entry: &Path,
    config: &ImportConfig,
    store_hash: &str,
    origin_ref: &str,
) -> Result<(), String> {
    use crate::tripwire::eventlog::now_iso8601;

    let provider_str = format!("{:?}", config.provider).to_lowercase();
    let meta = StoreMeta {
        schema: "1.0".to_string(),
        store_hash: store_hash.to_string(),
        recipe_hash: format!("import:{}", provider_str),
        input_hashes: vec![origin_ref.to_string()],
        arch: config.arch.clone(),
        provider: provider_str.clone(),
        created_at: now_iso8601(),
        generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
        references: Vec::new(),
        provenance: Some(Provenance {
            origin_provider: provider_str,
            origin_ref: Some(origin_ref.to_string()),
            origin_hash: Some(store_hash.to_string()),
            derived_from: None,
            derivation_depth: 0,
        }),
    };

    write_meta(store_entry, &meta)
}

/// Atomic move: rename staging → store content dir.
pub fn atomic_move_to_store(staging: &Path, target: &Path) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create store entry dir {}: {e}", parent.display()))?;
    }
    std::fs::rename(staging, target).map_err(|e| {
        format!(
            "atomic move {} → {}: {e}",
            staging.display(),
            target.display()
        )
    })
}

/// Clean up staging directory on failure.
fn cleanup_staging(staging: &Path) {
    let _ = std::fs::remove_dir_all(staging);
}

/// Count files and total bytes in a directory.
pub fn dir_stats(dir: &Path) -> (u64, u64) {
    let mut count = 0u64;
    let mut size = 0u64;
    if let Ok(entries) = walkdir(dir) {
        for (_, file_size) in entries {
            count += 1;
            size += file_size;
        }
    }
    (count, size)
}

/// Walk directory recursively, returning (path, size) pairs for files.
pub fn walkdir(dir: &Path) -> Result<Vec<(PathBuf, u64)>, String> {
    let mut results = Vec::new();
    walkdir_inner(dir, &mut results)?;
    Ok(results)
}

fn walkdir_inner(dir: &Path, results: &mut Vec<(PathBuf, u64)>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walkdir_inner(&path, results)?;
        } else if path.is_file() {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            results.push((path, size));
        }
    }
    Ok(())
}
