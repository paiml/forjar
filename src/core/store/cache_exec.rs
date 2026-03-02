//! FJ-1360: Cache SSH execution.
//!
//! Bridges the substitution protocol → actual SSH rsync transport.
//! Handles pull/push to SSH caches, hash verification, and the
//! full substitution protocol execution with I/O.

use super::cache::CacheSource;
use super::substitution::{SubstitutionOutcome, SubstitutionPlan, SubstitutionStep};
use crate::core::types::Machine;
use crate::transport;
use std::path::Path;

/// Result of pulling from a cache source.
#[derive(Debug, Clone)]
pub struct CachePullResult {
    /// Store hash that was pulled
    pub store_hash: String,
    /// Local store path after pull
    pub store_path: String,
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Whether the hash was verified after pull
    pub verified: bool,
}

/// Pull a store entry from an SSH cache to the local store.
///
/// 1. rsync to a temp staging directory
/// 2. Verify hash (BLAKE3)
/// 3. Atomic rename to final store location
pub fn pull_from_cache(
    source: &CacheSource,
    store_hash: &str,
    store_dir: &Path,
    machine: &Machine,
    timeout_secs: Option<u64>,
) -> Result<CachePullResult, String> {
    let hash_bare = store_hash.strip_prefix("blake3:").unwrap_or(store_hash);
    let target = store_dir.join(hash_bare);
    let staging = store_dir.join(format!(".staging-{hash_bare}"));

    // Generate pull command
    let pull_cmd = pull_command(source, store_hash, &staging);

    // Execute rsync/cp
    let output = transport::exec_script_timeout(machine, &pull_cmd, timeout_secs)
        .map_err(|e| format!("cache pull failed: {e}"))?;

    if !output.success() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(format!(
            "cache pull exit code {}: {}",
            output.exit_code,
            output.stderr.trim()
        ));
    }

    // Verify via hash of pulled content
    let verified = verify_pulled_content(&staging, store_hash);

    // Atomic move to store
    if target.exists() {
        // Already exists (race condition safe — content-addressed = idempotent)
        let _ = std::fs::remove_dir_all(&staging);
    } else {
        std::fs::rename(&staging, &target).map_err(|e| {
            let _ = std::fs::remove_dir_all(&staging);
            format!("atomic move staging → store: {e}")
        })?;
    }

    let bytes = super::gc_exec::dir_size(&target);

    Ok(CachePullResult {
        store_hash: store_hash.to_string(),
        store_path: target.display().to_string(),
        bytes_transferred: bytes,
        verified,
    })
}

/// Push a local store entry to an SSH cache.
pub fn push_to_cache(
    source: &CacheSource,
    store_hash: &str,
    store_dir: &Path,
    machine: &Machine,
    timeout_secs: Option<u64>,
) -> Result<(), String> {
    let push_cmd = push_command(source, store_hash, store_dir);

    let output = transport::exec_script_timeout(machine, &push_cmd, timeout_secs)
        .map_err(|e| format!("cache push failed: {e}"))?;

    if !output.success() {
        return Err(format!(
            "cache push exit code {}: {}",
            output.exit_code,
            output.stderr.trim()
        ));
    }

    Ok(())
}

/// Verify a remote cache entry by checking if it exists.
pub fn verify_remote_entry(
    source: &CacheSource,
    store_hash: &str,
    machine: &Machine,
) -> Result<bool, String> {
    let check_cmd = remote_check_command(source, store_hash);
    let output = transport::exec_script_timeout(machine, &check_cmd, Some(30))
        .map_err(|e| format!("remote verify: {e}"))?;
    Ok(output.success())
}

/// Execute the full substitution protocol with I/O.
///
/// Runs the substitution plan: local check → cache pull → build → push.
/// Returns the store path of the resolved entry.
pub fn execute_substitution(
    plan: &SubstitutionPlan,
    machine: &Machine,
    store_dir: &Path,
    timeout_secs: Option<u64>,
) -> Result<String, String> {
    match &plan.outcome {
        SubstitutionOutcome::LocalHit { store_path } => Ok(store_path.clone()),

        SubstitutionOutcome::CacheHit { source, store_hash } => {
            // Find the SSH source from the plan steps
            let cache_source = extract_cache_source_from_plan(plan)
                .ok_or("cache hit but no SSH source in plan")?;

            let result =
                pull_from_cache(&cache_source, store_hash, store_dir, machine, timeout_secs)?;

            if !result.verified {
                return Err(format!("cache pull from {source} failed hash verification"));
            }

            Ok(result.store_path)
        }

        SubstitutionOutcome::CacheMiss { store_hash } => {
            // Build from scratch not handled here — caller must build.
            // We return an error indicating the caller should invoke the build pipeline.
            Err(format!(
                "cache miss for {store_hash}: build required (use sandbox_run)"
            ))
        }
    }
}

/// Generate the rsync pull command for a cache source.
pub fn pull_command(source: &CacheSource, hash: &str, staging: &Path) -> String {
    let hash_bare = hash.strip_prefix("blake3:").unwrap_or(hash);
    match source {
        CacheSource::Ssh {
            host,
            user,
            path,
            port,
        } => {
            let port_flag = port.map_or(String::new(), |p| format!(" -p {p}"));
            format!(
                "mkdir -p '{}' && rsync -az -e 'ssh{port_flag}' '{user}@{host}:{path}/{hash_bare}/' '{}'",
                staging.display(),
                staging.display()
            )
        }
        CacheSource::Local { path } => {
            format!(
                "mkdir -p '{}' && cp -a '{path}/{hash_bare}/.' '{}'",
                staging.display(),
                staging.display()
            )
        }
    }
}

/// Generate the rsync push command for a cache source.
pub fn push_command(source: &CacheSource, hash: &str, store_dir: &Path) -> String {
    let hash_bare = hash.strip_prefix("blake3:").unwrap_or(hash);
    match source {
        CacheSource::Ssh {
            host,
            user,
            path,
            port,
        } => {
            let port_flag = port.map_or(String::new(), |p| format!(" -p {p}"));
            format!(
                "rsync -az -e 'ssh{port_flag}' '{}/{hash_bare}/' '{user}@{host}:{path}/{hash_bare}/'",
                store_dir.display()
            )
        }
        CacheSource::Local { path } => {
            format!(
                "cp -a '{}/{hash_bare}' '{path}/{hash_bare}'",
                store_dir.display()
            )
        }
    }
}

/// Generate SSH command to check if an entry exists remotely.
fn remote_check_command(source: &CacheSource, hash: &str) -> String {
    let hash_bare = hash.strip_prefix("blake3:").unwrap_or(hash);
    match source {
        CacheSource::Ssh {
            host,
            user,
            path,
            port,
        } => {
            let port_flag = port.map_or(String::new(), |p| format!(" -p {p}"));
            format!("ssh{port_flag} '{user}@{host}' test -d '{path}/{hash_bare}'")
        }
        CacheSource::Local { path } => {
            format!("test -d '{path}/{hash_bare}'")
        }
    }
}

/// Verify pulled content matches expected hash via BLAKE3 re-hash.
fn verify_pulled_content(staging: &Path, expected_hash: &str) -> bool {
    let content_dir = staging.join("content");
    let dir_to_hash = if content_dir.is_dir() {
        &content_dir
    } else {
        staging
    };
    match crate::tripwire::hasher::hash_directory(dir_to_hash) {
        Ok(actual) => {
            let expected = if expected_hash.starts_with("blake3:") {
                expected_hash.to_string()
            } else {
                format!("blake3:{expected_hash}")
            };
            actual == expected
        }
        Err(_) => false,
    }
}

/// Extract the CacheSource from a substitution plan's pull step.
fn extract_cache_source_from_plan(plan: &SubstitutionPlan) -> Option<CacheSource> {
    for step in &plan.steps {
        if let SubstitutionStep::PullFromCache { source, .. } = step {
            // Parse "user@host" back into a CacheSource
            let parts: Vec<&str> = source.splitn(2, '@').collect();
            if parts.len() == 2 {
                return Some(CacheSource::Ssh {
                    host: parts[1].to_string(),
                    user: parts[0].to_string(),
                    path: "/var/lib/forjar/cache".to_string(),
                    port: None,
                });
            }
        }
    }
    None
}
