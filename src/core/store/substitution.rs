//! FJ-1322: Substitution protocol executor.
//!
//! Orchestrates the full substitution protocol:
//! 1. Compute store hash from input closure
//! 2. Check local store → HIT = substitute (skip build)
//! 3. Check SSH cache sources → HIT = pull from cache
//! 4. Cache miss → build from scratch (with sandbox if configured)
//! 5. Store result, optionally push to cache
//!
//! Returns an execution plan (dry-run) or simulated result.

use super::cache::{CacheConfig, CacheInventory, CacheSource};
use super::sandbox::SandboxConfig;
use std::path::Path;

/// Configuration for the substitution protocol.
pub struct SubstitutionContext<'a> {
    /// Content-addressed hash of the input closure.
    pub closure_hash: &'a str,
    /// BLAKE3 hashes of all inputs.
    pub input_hashes: &'a [String],
    /// Store hashes present in the local store.
    pub local_entries: &'a [String],
    /// Cache source configuration.
    pub cache_config: &'a CacheConfig,
    /// Inventories of available cache entries.
    pub cache_inventories: &'a [CacheInventory],
    /// Optional sandbox configuration for builds.
    pub sandbox: Option<&'a SandboxConfig>,
    /// Local store directory path.
    pub store_dir: &'a Path,
}

/// A step in the substitution protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum SubstitutionStep {
    /// Compute closure hash from inputs.
    ComputeClosureHash {
        /// All input hashes that compose the closure.
        input_hashes: Vec<String>,
        /// Resulting closure hash.
        closure_hash: String,
    },
    /// Check local store for existing entry.
    CheckLocalStore {
        /// Store hash to look up.
        store_hash: String,
        /// Whether the entry was found.
        found: bool,
    },
    /// Check SSH cache for existing entry.
    CheckSshCache {
        /// SSH hostname.
        host: String,
        /// SSH user.
        user: String,
        /// Store hash to look up.
        store_hash: String,
        /// Whether the entry was found.
        found: bool,
    },
    /// Pull entry from SSH cache.
    PullFromCache {
        /// Cache source identifier.
        source: String,
        /// Store hash to pull.
        store_hash: String,
        /// Shell command to execute the pull.
        command: String,
    },
    /// Build from scratch (cache miss).
    BuildFromScratch {
        /// Store hash of the artifact to build.
        store_hash: String,
        /// Sandbox isolation level.
        sandbox_level: String,
    },
    /// Store the build result.
    StoreResult {
        /// Store hash of the stored artifact.
        store_hash: String,
        /// Filesystem path where the artifact was stored.
        store_path: String,
    },
    /// Push result to SSH cache (auto_push).
    PushToCache {
        /// Cache source identifier.
        source: String,
        /// Store hash to push.
        store_hash: String,
        /// Shell command to execute the push.
        command: String,
    },
}

/// Full substitution protocol execution plan.
#[derive(Debug, Clone, PartialEq)]
pub struct SubstitutionPlan {
    /// Protocol steps in order
    pub steps: Vec<SubstitutionStep>,
    /// Final outcome
    pub outcome: SubstitutionOutcome,
    /// Store hash being resolved
    pub store_hash: String,
}

/// Outcome of the substitution protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum SubstitutionOutcome {
    /// Found in local store — no work needed.
    LocalHit {
        /// Filesystem path to the local store entry.
        store_path: String,
    },
    /// Found in SSH cache — pull required.
    CacheHit {
        /// Cache source identifier.
        source: String,
        /// Store hash found in the cache.
        store_hash: String,
    },
    /// Not found anywhere — build required.
    CacheMiss {
        /// Store hash that must be built.
        store_hash: String,
    },
}

/// Plan the full substitution protocol for a given store hash.
///
/// Does NOT execute I/O — produces a step-by-step plan.
#[allow(clippy::too_many_arguments)]
pub fn plan_substitution(ctx: &SubstitutionContext<'_>) -> SubstitutionPlan {
    let closure_hash = ctx.closure_hash;
    let store_dir = ctx.store_dir;
    let mut steps = Vec::new();

    // Step 1: Record closure hash computation
    steps.push(SubstitutionStep::ComputeClosureHash {
        input_hashes: ctx.input_hashes.to_vec(),
        closure_hash: closure_hash.to_string(),
    });

    // Step 2: Check local store
    let local_hit = ctx.local_entries.contains(&closure_hash.to_string());
    steps.push(SubstitutionStep::CheckLocalStore {
        store_hash: closure_hash.to_string(),
        found: local_hit,
    });

    if local_hit {
        let hash_bare = closure_hash.strip_prefix("blake3:").unwrap_or(closure_hash);
        let store_path = format!("{}/{hash_bare}/content", store_dir.display());
        return SubstitutionPlan {
            steps,
            outcome: SubstitutionOutcome::LocalHit { store_path },
            store_hash: closure_hash.to_string(),
        };
    }

    // Step 3: Check SSH caches in order
    for (i, source) in ctx.cache_config.sources.iter().enumerate() {
        if let CacheSource::Ssh { host, user, .. } = source {
            let found = ctx
                .cache_inventories
                .get(i)
                .map(|inv| inv.entries.contains_key(closure_hash))
                .unwrap_or(false);

            steps.push(SubstitutionStep::CheckSshCache {
                host: host.clone(),
                user: user.clone(),
                store_hash: closure_hash.to_string(),
                found,
            });

            if found {
                let pull_cmd = ssh_pull_command(source, closure_hash, store_dir);
                steps.push(SubstitutionStep::PullFromCache {
                    source: format!("{user}@{host}"),
                    store_hash: closure_hash.to_string(),
                    command: pull_cmd,
                });

                return SubstitutionPlan {
                    steps,
                    outcome: SubstitutionOutcome::CacheHit {
                        source: format!("{user}@{host}"),
                        store_hash: closure_hash.to_string(),
                    },
                    store_hash: closure_hash.to_string(),
                };
            }
        }
    }

    // Step 4: Cache miss — build from scratch
    let sandbox_level = ctx
        .sandbox
        .map(|s| format!("{:?}", s.level))
        .unwrap_or_else(|| "none".to_string());

    steps.push(SubstitutionStep::BuildFromScratch {
        store_hash: closure_hash.to_string(),
        sandbox_level,
    });

    // Step 5: Store result
    let hash_bare = closure_hash.strip_prefix("blake3:").unwrap_or(closure_hash);
    let store_path = format!("{}/{hash_bare}/content", store_dir.display());
    steps.push(SubstitutionStep::StoreResult {
        store_hash: closure_hash.to_string(),
        store_path,
    });

    // Step 6: Auto-push to first SSH source (if configured)
    if ctx.cache_config.auto_push {
        if let Some(ssh_source) = first_ssh_source(&ctx.cache_config.sources) {
            let push_cmd = ssh_push_command(ssh_source, closure_hash, store_dir);
            let (host, user) = ssh_host_user(ssh_source);
            steps.push(SubstitutionStep::PushToCache {
                source: format!("{user}@{host}"),
                store_hash: closure_hash.to_string(),
                command: push_cmd,
            });
        }
    }

    SubstitutionPlan {
        steps,
        outcome: SubstitutionOutcome::CacheMiss {
            store_hash: closure_hash.to_string(),
        },
        store_hash: closure_hash.to_string(),
    }
}

/// Check if a substitution plan requires building.
pub fn requires_build(plan: &SubstitutionPlan) -> bool {
    matches!(plan.outcome, SubstitutionOutcome::CacheMiss { .. })
}

/// Check if a substitution plan requires a cache pull.
pub fn requires_pull(plan: &SubstitutionPlan) -> bool {
    matches!(plan.outcome, SubstitutionOutcome::CacheHit { .. })
}

/// Count steps in the plan.
pub fn step_count(plan: &SubstitutionPlan) -> usize {
    plan.steps.len()
}

/// Generate the SSH command to pull an entry from a cache source.
fn ssh_pull_command(source: &CacheSource, hash: &str, store_dir: &Path) -> String {
    match source {
        CacheSource::Ssh {
            host,
            user,
            path,
            port,
        } => {
            let hash_bare = hash.strip_prefix("blake3:").unwrap_or(hash);
            let port_flag = port.map_or(String::new(), |p| format!(" -p {p}"));
            format!(
                "rsync -az -e 'ssh{port_flag}' {user}@{host}:{path}/{hash_bare}/ {}/{hash_bare}/",
                store_dir.display()
            )
        }
        CacheSource::Local { path } => {
            let hash_bare = hash.strip_prefix("blake3:").unwrap_or(hash);
            format!(
                "cp -a {path}/{hash_bare} {}/{hash_bare}",
                store_dir.display()
            )
        }
    }
}

/// Generate the SSH command to push an entry to a cache source.
fn ssh_push_command(source: &CacheSource, hash: &str, store_dir: &Path) -> String {
    match source {
        CacheSource::Ssh {
            host,
            user,
            path,
            port,
        } => {
            let hash_bare = hash.strip_prefix("blake3:").unwrap_or(hash);
            let port_flag = port.map_or(String::new(), |p| format!(" -p {p}"));
            format!(
                "rsync -az -e 'ssh{port_flag}' {}/{hash_bare}/ {user}@{host}:{path}/{hash_bare}/",
                store_dir.display()
            )
        }
        CacheSource::Local { path } => {
            let hash_bare = hash.strip_prefix("blake3:").unwrap_or(hash);
            format!(
                "cp -a {}/{hash_bare} {path}/{hash_bare}",
                store_dir.display()
            )
        }
    }
}

/// Find the first SSH source in the list.
fn first_ssh_source(sources: &[CacheSource]) -> Option<&CacheSource> {
    sources
        .iter()
        .find(|s| matches!(s, CacheSource::Ssh { .. }))
}

/// Extract host and user from an SSH source.
fn ssh_host_user(source: &CacheSource) -> (String, String) {
    match source {
        CacheSource::Ssh { host, user, .. } => (host.clone(), user.clone()),
        CacheSource::Local { path } => (path.clone(), "local".to_string()),
    }
}
