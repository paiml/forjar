//! FJ-1320–FJ-1322: Binary cache configuration and substitution protocol.
//!
//! Defines cache source configuration (SSH/local), the substitution protocol
//! (check local → check cache → build), and cache entry verification.
//! SSH-only by design — no HTTP client crate, no tokens, no TLS certificates.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A cache source for looking up pre-built store entries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CacheSource {
    /// SSH remote cache (sovereign — no HTTP)
    #[serde(rename = "ssh")]
    Ssh {
        /// SSH hostname.
        host: String,
        /// SSH user.
        user: String,
        /// Remote store path.
        path: String,
        /// SSH port override.
        #[serde(default)]
        port: Option<u16>,
    },
    /// Local filesystem cache (the store itself)
    #[serde(rename = "local")]
    Local {
        /// Local filesystem path.
        path: String,
    },
}

/// Top-level cache configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Ordered list of cache sources (checked in order)
    pub sources: Vec<CacheSource>,

    /// Whether to auto-push builds to the first SSH source
    #[serde(default)]
    pub auto_push: bool,

    /// Maximum local cache size in megabytes (0 = unlimited)
    #[serde(default)]
    pub max_size_mb: u64,
}

/// Result of a substitution lookup.
#[derive(Debug, Clone, PartialEq)]
pub enum SubstitutionResult {
    /// Found in local store
    LocalHit {
        /// Filesystem path to the local store entry.
        store_path: String,
    },
    /// Found in remote cache
    CacheHit {
        /// Index of the cache source that had the entry.
        source_index: usize,
        /// Content-addressed store hash.
        store_hash: String,
    },
    /// Not found — must build from scratch
    CacheMiss,
}

/// A cache entry's metadata (what we store alongside the artifact).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Content-addressed store hash.
    pub store_hash: String,
    /// Entry size in bytes.
    pub size_bytes: u64,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// Provider that built this entry.
    pub provider: String,
    /// Target architecture.
    pub arch: String,
}

/// A cache inventory — what's available in a given cache source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheInventory {
    /// Name or identifier of this cache source.
    pub source_name: String,
    /// Map of store hash to cache entry.
    pub entries: BTreeMap<String, CacheEntry>,
}

/// Parse cache config from YAML.
pub fn parse_cache_config(yaml: &str) -> Result<CacheConfig, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("invalid cache config: {e}"))
}

/// Validate cache configuration.
pub fn validate_cache_config(config: &CacheConfig) -> Vec<String> {
    let mut errors = Vec::new();

    if config.sources.is_empty() {
        errors.push("at least one cache source required".to_string());
    }

    for (i, source) in config.sources.iter().enumerate() {
        match source {
            CacheSource::Ssh {
                host, user, path, ..
            } => {
                if host.is_empty() {
                    errors.push(format!("source[{i}]: SSH host cannot be empty"));
                }
                if user.is_empty() {
                    errors.push(format!("source[{i}]: SSH user cannot be empty"));
                }
                if path.is_empty() {
                    errors.push(format!("source[{i}]: SSH path cannot be empty"));
                }
            }
            CacheSource::Local { path } => {
                if path.is_empty() {
                    errors.push(format!("source[{i}]: local path cannot be empty"));
                }
            }
        }
    }

    errors
}

/// Perform substitution lookup: check local store, then each cache source.
///
/// Returns where the artifact was found (or CacheMiss).
/// This is the protocol logic — actual I/O is handled by the caller.
pub fn resolve_substitution(
    store_hash: &str,
    local_entries: &[String],
    cache_inventories: &[CacheInventory],
) -> SubstitutionResult {
    // 1. Check local store
    if local_entries.contains(&store_hash.to_string()) {
        return SubstitutionResult::LocalHit {
            store_path: format!("/var/lib/forjar/store/{}", strip_blake3(store_hash)),
        };
    }

    // 2. Check remote caches in order
    for (i, inventory) in cache_inventories.iter().enumerate() {
        if inventory.entries.contains_key(store_hash) {
            return SubstitutionResult::CacheHit {
                source_index: i,
                store_hash: store_hash.to_string(),
            };
        }
    }

    // 3. Not found
    SubstitutionResult::CacheMiss
}

/// Verify a cache entry by re-hashing its content.
pub fn verify_entry(entry: &CacheEntry, actual_hash: &str) -> bool {
    entry.store_hash == actual_hash
}

/// Build a cache inventory from a list of entries.
pub fn build_inventory(source_name: &str, entries: Vec<CacheEntry>) -> CacheInventory {
    let map = entries
        .into_iter()
        .map(|e| (e.store_hash.clone(), e))
        .collect();
    CacheInventory {
        source_name: source_name.to_string(),
        entries: map,
    }
}

/// Generate the SSH command prefix for a cache source.
pub fn ssh_command(source: &CacheSource) -> Option<String> {
    match source {
        CacheSource::Ssh {
            host, user, port, ..
        } => {
            let port_flag = port.map_or(String::new(), |p| format!(" -p {p}"));
            Some(format!("ssh{port_flag} {user}@{host}"))
        }
        CacheSource::Local { .. } => None,
    }
}

fn strip_blake3(hash: &str) -> &str {
    hash.strip_prefix("blake3:").unwrap_or(hash)
}
