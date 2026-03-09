//! FJ-3302: Ephemeral value resolution pipeline.
//!
//! Resolves secrets during apply, uses them, then discards the plaintext.
//! Only a BLAKE3 hash is stored for drift detection (hash-and-discard).

use crate::core::secret_provider::ProviderChain;
use std::collections::HashMap;

/// An ephemeral parameter declaration.
#[derive(Debug, Clone)]
pub struct EphemeralParam {
    /// Parameter key name.
    pub key: String,
    /// Secret provider key to resolve.
    pub provider_key: String,
}

/// Resolved ephemeral value — plaintext + BLAKE3 hash.
#[derive(Debug, Clone)]
pub struct ResolvedEphemeral {
    /// Parameter key.
    pub key: String,
    /// Plaintext value (cleared after use).
    pub value: String,
    /// BLAKE3 hash of the plaintext for drift detection.
    pub hash: String,
}

/// Hash-only record stored in state (plaintext discarded).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EphemeralRecord {
    /// Parameter key.
    pub key: String,
    /// BLAKE3 hash of the resolved value.
    pub hash: String,
}

/// Resolve ephemeral parameters using the provider chain.
///
/// Returns resolved values with their BLAKE3 hashes. The caller must
/// discard plaintext after use (e.g., after template substitution).
pub fn resolve_ephemerals(
    params: &[EphemeralParam],
    chain: &ProviderChain,
) -> Result<Vec<ResolvedEphemeral>, String> {
    let mut results = Vec::with_capacity(params.len());
    for param in params {
        let secret = chain
            .resolve(&param.provider_key)
            .map_err(|e| format!("ephemeral '{}': {e}", param.key))?
            .ok_or_else(|| format!("ephemeral '{}': no provider resolved key", param.key))?;
        let hash = blake3_hash(&secret.value);
        results.push(ResolvedEphemeral {
            key: param.key.clone(),
            value: secret.value,
            hash,
        });
    }
    Ok(results)
}

/// Convert resolved ephemerals to hash-only records for state storage.
pub fn to_records(resolved: &[ResolvedEphemeral]) -> Vec<EphemeralRecord> {
    resolved
        .iter()
        .map(|r| EphemeralRecord {
            key: r.key.clone(),
            hash: r.hash.clone(),
        })
        .collect()
}

/// Check if current values match stored hashes (drift detection).
pub fn check_drift(current: &[ResolvedEphemeral], stored: &[EphemeralRecord]) -> Vec<DriftResult> {
    let stored_map: HashMap<&str, &str> = stored
        .iter()
        .map(|r| (r.key.as_str(), r.hash.as_str()))
        .collect();

    current
        .iter()
        .map(|c| {
            let status = match stored_map.get(c.key.as_str()) {
                Some(stored_hash) if *stored_hash == c.hash => DriftStatus::Unchanged,
                Some(_) => DriftStatus::Changed,
                None => DriftStatus::New,
            };
            DriftResult {
                key: c.key.clone(),
                status,
            }
        })
        .collect()
}

/// Result of drift checking an ephemeral value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftResult {
    /// Parameter key.
    pub key: String,
    /// Drift status.
    pub status: DriftStatus,
}

/// Ephemeral value drift status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftStatus {
    /// Value hash matches stored hash.
    Unchanged,
    /// Value hash differs from stored hash.
    Changed,
    /// No stored hash (first resolution).
    New,
}

/// Substitute ephemeral values into a template string.
///
/// Replaces `{{ephemeral.KEY}}` patterns with resolved values.
pub fn substitute_ephemerals(template: &str, resolved: &[ResolvedEphemeral]) -> String {
    let mut result = template.to_string();
    for r in resolved {
        let pattern = format!("{{{{ephemeral.{}}}}}", r.key);
        result = result.replace(&pattern, &r.value);
    }
    result
}

/// Compute BLAKE3 hash of a value, returning hex string.
fn blake3_hash(value: &str) -> String {
    let hash = blake3::hash(value.as_bytes());
    hash.to_hex().to_string()
}

/// Compute keyed BLAKE3 HMAC for secret verification.
pub fn blake3_keyed_hash(key: &[u8; 32], value: &str) -> String {
    let hash = blake3::keyed_hash(key, value.as_bytes());
    hash.to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::secret_provider::{EnvProvider, ProviderChain};

    #[test]
    fn blake3_hash_deterministic() {
        let h1 = blake3_hash("secret-value");
        let h2 = blake3_hash("secret-value");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // 32 bytes hex
    }

    #[test]
    fn blake3_hash_different_inputs() {
        let h1 = blake3_hash("secret-a");
        let h2 = blake3_hash("secret-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn keyed_hash_works() {
        let key = [0u8; 32];
        let h1 = blake3_keyed_hash(&key, "data");
        let h2 = blake3_keyed_hash(&key, "data");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn keyed_hash_different_keys() {
        let k1 = [0u8; 32];
        let k2 = [1u8; 32];
        let h1 = blake3_keyed_hash(&k1, "data");
        let h2 = blake3_keyed_hash(&k2, "data");
        assert_ne!(h1, h2);
    }

    #[test]
    fn to_records_strips_plaintext() {
        let resolved = vec![ResolvedEphemeral {
            key: "db_pass".into(),
            value: "s3cret".into(),
            hash: blake3_hash("s3cret"),
        }];
        let records = to_records(&resolved);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].key, "db_pass");
        assert_eq!(records[0].hash, blake3_hash("s3cret"));
        // record has no value field — plaintext is discarded
    }

    #[test]
    fn check_drift_unchanged() {
        let hash = blake3_hash("val");
        let current = vec![ResolvedEphemeral {
            key: "k".into(),
            value: "val".into(),
            hash: hash.clone(),
        }];
        let stored = vec![EphemeralRecord {
            key: "k".into(),
            hash,
        }];
        let results = check_drift(&current, &stored);
        assert_eq!(results[0].status, DriftStatus::Unchanged);
    }

    #[test]
    fn check_drift_changed() {
        let current = vec![ResolvedEphemeral {
            key: "k".into(),
            value: "new-val".into(),
            hash: blake3_hash("new-val"),
        }];
        let stored = vec![EphemeralRecord {
            key: "k".into(),
            hash: blake3_hash("old-val"),
        }];
        let results = check_drift(&current, &stored);
        assert_eq!(results[0].status, DriftStatus::Changed);
    }

    #[test]
    fn check_drift_new_key() {
        let current = vec![ResolvedEphemeral {
            key: "new-key".into(),
            value: "val".into(),
            hash: blake3_hash("val"),
        }];
        let stored: Vec<EphemeralRecord> = vec![];
        let results = check_drift(&current, &stored);
        assert_eq!(results[0].status, DriftStatus::New);
    }

    #[test]
    fn substitute_ephemerals_replaces() {
        let resolved = vec![
            ResolvedEphemeral {
                key: "db_pass".into(),
                value: "s3cret".into(),
                hash: String::new(),
            },
            ResolvedEphemeral {
                key: "api_key".into(),
                value: "abc123".into(),
                hash: String::new(),
            },
        ];
        let template = "postgres://user:{{ephemeral.db_pass}}@host/db?key={{ephemeral.api_key}}";
        let result = substitute_ephemerals(template, &resolved);
        assert_eq!(result, "postgres://user:s3cret@host/db?key=abc123");
    }

    #[test]
    fn substitute_no_match_unchanged() {
        let resolved = vec![];
        let template = "no {{ephemeral.x}} substitution";
        let result = substitute_ephemerals(template, &resolved);
        assert_eq!(result, "no {{ephemeral.x}} substitution");
    }

    #[test]
    fn resolve_with_env_provider() {
        // PATH is always set
        let chain = ProviderChain::new().with(Box::new(EnvProvider));
        let params = vec![EphemeralParam {
            key: "path".into(),
            provider_key: "PATH".into(),
        }];
        let resolved = resolve_ephemerals(&params, &chain).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].key, "path");
        assert!(!resolved[0].value.is_empty());
        assert_eq!(resolved[0].hash.len(), 64);
    }

    #[test]
    fn resolve_missing_key_errors() {
        let chain = ProviderChain::new();
        let params = vec![EphemeralParam {
            key: "missing".into(),
            provider_key: "NONEXISTENT_KEY_12345".into(),
        }];
        let result = resolve_ephemerals(&params, &chain);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no provider resolved"));
    }

    #[test]
    fn ephemeral_record_serde() {
        let record = EphemeralRecord {
            key: "db_pass".into(),
            hash: blake3_hash("test"),
        };
        let json = serde_json::to_string(&record).unwrap();
        let parsed: EphemeralRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.key, "db_pass");
        assert_eq!(parsed.hash, record.hash);
    }
}
