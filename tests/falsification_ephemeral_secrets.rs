//! FJ-3302: Ephemeral value resolution pipeline falsification.
//!
//! Popperian rejection criteria for:
//! - BLAKE3 hash of secrets (deterministic, no plaintext stored)
//! - Keyed HMAC for secret verification
//! - EphemeralRecord serialization (hash-only, no plaintext)
//! - ResolvedEphemeral → EphemeralRecord stripping
//! - Drift detection (unchanged, changed, new)
//! - Template substitution with ephemeral values
//! - Provider chain resolution and error handling
//!
//! Usage: cargo test --test falsification_ephemeral_secrets

use forjar::core::ephemeral::{
    blake3_keyed_hash, check_drift, resolve_ephemerals, substitute_ephemerals, to_records,
    DriftResult, DriftStatus, EphemeralParam, EphemeralRecord, ResolvedEphemeral,
};
use forjar::core::secret_provider::{EnvProvider, FileProvider, ProviderChain};

// ============================================================================
// FJ-3302: BLAKE3 Hash (via to_records pipeline)
// ============================================================================

#[test]
fn hash_deterministic_via_records() {
    let r1 = ResolvedEphemeral {
        key: "k".into(),
        value: "secret-value".into(),
        hash: blake3::hash(b"secret-value").to_hex().to_string(),
    };
    let r2 = ResolvedEphemeral {
        key: "k".into(),
        value: "secret-value".into(),
        hash: blake3::hash(b"secret-value").to_hex().to_string(),
    };
    assert_eq!(r1.hash, r2.hash);
    assert_eq!(r1.hash.len(), 64);
}

#[test]
fn hash_different_values_differ() {
    let h1 = blake3::hash(b"val-a").to_hex().to_string();
    let h2 = blake3::hash(b"val-b").to_hex().to_string();
    assert_ne!(h1, h2);
}

// ============================================================================
// FJ-3302: Keyed HMAC
// ============================================================================

#[test]
fn keyed_hash_deterministic() {
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
fn keyed_hash_different_data() {
    let key = [42u8; 32];
    let h1 = blake3_keyed_hash(&key, "data-a");
    let h2 = blake3_keyed_hash(&key, "data-b");
    assert_ne!(h1, h2);
}

// ============================================================================
// FJ-3302: to_records — Hash-Only Storage
// ============================================================================

#[test]
fn to_records_preserves_key_and_hash() {
    let hash = blake3::hash(b"s3cret").to_hex().to_string();
    let resolved = vec![ResolvedEphemeral {
        key: "db_pass".into(),
        value: "s3cret".into(),
        hash: hash.clone(),
    }];
    let records = to_records(&resolved);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].key, "db_pass");
    assert_eq!(records[0].hash, hash);
}

#[test]
fn to_records_multiple() {
    let resolved = vec![
        ResolvedEphemeral {
            key: "a".into(),
            value: "v1".into(),
            hash: blake3::hash(b"v1").to_hex().to_string(),
        },
        ResolvedEphemeral {
            key: "b".into(),
            value: "v2".into(),
            hash: blake3::hash(b"v2").to_hex().to_string(),
        },
    ];
    let records = to_records(&resolved);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].key, "a");
    assert_eq!(records[1].key, "b");
}

#[test]
fn to_records_empty() {
    let records = to_records(&[]);
    assert!(records.is_empty());
}

// ============================================================================
// FJ-3302: EphemeralRecord Serde
// ============================================================================

#[test]
fn ephemeral_record_serde_roundtrip() {
    let record = EphemeralRecord {
        key: "db_pass".into(),
        hash: blake3::hash(b"test").to_hex().to_string(),
    };
    let json = serde_json::to_string(&record).unwrap();
    let parsed: EphemeralRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.key, "db_pass");
    assert_eq!(parsed.hash, record.hash);
}

#[test]
fn ephemeral_record_no_value_field() {
    let record = EphemeralRecord {
        key: "k".into(),
        hash: "h".into(),
    };
    let json = serde_json::to_string(&record).unwrap();
    // JSON should contain key and hash but NOT value
    assert!(json.contains("\"key\""));
    assert!(json.contains("\"hash\""));
    assert!(!json.contains("\"value\""));
}

// ============================================================================
// FJ-3302: Drift Detection
// ============================================================================

#[test]
fn drift_unchanged() {
    let hash = blake3::hash(b"val").to_hex().to_string();
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
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, DriftStatus::Unchanged);
    assert_eq!(results[0].key, "k");
}

#[test]
fn drift_changed() {
    let current = vec![ResolvedEphemeral {
        key: "k".into(),
        value: "new-val".into(),
        hash: blake3::hash(b"new-val").to_hex().to_string(),
    }];
    let stored = vec![EphemeralRecord {
        key: "k".into(),
        hash: blake3::hash(b"old-val").to_hex().to_string(),
    }];
    let results = check_drift(&current, &stored);
    assert_eq!(results[0].status, DriftStatus::Changed);
}

#[test]
fn drift_new_key() {
    let current = vec![ResolvedEphemeral {
        key: "new-key".into(),
        value: "val".into(),
        hash: blake3::hash(b"val").to_hex().to_string(),
    }];
    let stored: Vec<EphemeralRecord> = vec![];
    let results = check_drift(&current, &stored);
    assert_eq!(results[0].status, DriftStatus::New);
}

#[test]
fn drift_multiple_keys() {
    let current = vec![
        ResolvedEphemeral {
            key: "unchanged".into(),
            value: "same".into(),
            hash: blake3::hash(b"same").to_hex().to_string(),
        },
        ResolvedEphemeral {
            key: "changed".into(),
            value: "new".into(),
            hash: blake3::hash(b"new").to_hex().to_string(),
        },
        ResolvedEphemeral {
            key: "added".into(),
            value: "fresh".into(),
            hash: blake3::hash(b"fresh").to_hex().to_string(),
        },
    ];
    let stored = vec![
        EphemeralRecord {
            key: "unchanged".into(),
            hash: blake3::hash(b"same").to_hex().to_string(),
        },
        EphemeralRecord {
            key: "changed".into(),
            hash: blake3::hash(b"old").to_hex().to_string(),
        },
    ];
    let results = check_drift(&current, &stored);
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].status, DriftStatus::Unchanged);
    assert_eq!(results[1].status, DriftStatus::Changed);
    assert_eq!(results[2].status, DriftStatus::New);
}

#[test]
fn drift_empty_current_empty() {
    let results = check_drift(&[], &[]);
    assert!(results.is_empty());
}

// ============================================================================
// FJ-3302: DriftStatus Eq/Clone
// ============================================================================

#[test]
fn drift_status_eq() {
    assert_eq!(DriftStatus::Unchanged, DriftStatus::Unchanged);
    assert_eq!(DriftStatus::Changed, DriftStatus::Changed);
    assert_eq!(DriftStatus::New, DriftStatus::New);
    assert_ne!(DriftStatus::Unchanged, DriftStatus::Changed);
    assert_ne!(DriftStatus::Changed, DriftStatus::New);
}

#[test]
fn drift_result_clone() {
    let dr = DriftResult {
        key: "k".into(),
        status: DriftStatus::Changed,
    };
    let cloned = dr.clone();
    assert_eq!(cloned.key, "k");
    assert_eq!(cloned.status, DriftStatus::Changed);
}

#[test]
fn drift_status_debug() {
    let debug = format!("{:?}", DriftStatus::New);
    assert_eq!(debug, "New");
}

// ============================================================================
// FJ-3302: Template Substitution
// ============================================================================

#[test]
fn substitute_single_ephemeral() {
    let resolved = vec![ResolvedEphemeral {
        key: "db_pass".into(),
        value: "s3cret".into(),
        hash: String::new(),
    }];
    let result = substitute_ephemerals("postgres://user:{{ephemeral.db_pass}}@host/db", &resolved);
    assert_eq!(result, "postgres://user:s3cret@host/db");
}

#[test]
fn substitute_multiple_ephemerals() {
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
    let result = substitute_ephemerals(
        "postgres://u:{{ephemeral.db_pass}}@h/db?key={{ephemeral.api_key}}",
        &resolved,
    );
    assert_eq!(result, "postgres://u:s3cret@h/db?key=abc123");
}

#[test]
fn substitute_no_match_unchanged() {
    let result = substitute_ephemerals("no {{ephemeral.x}} substitution", &[]);
    assert_eq!(result, "no {{ephemeral.x}} substitution");
}

#[test]
fn substitute_empty_template() {
    let resolved = vec![ResolvedEphemeral {
        key: "k".into(),
        value: "v".into(),
        hash: String::new(),
    }];
    let result = substitute_ephemerals("", &resolved);
    assert_eq!(result, "");
}

#[test]
fn substitute_repeated_key() {
    let resolved = vec![ResolvedEphemeral {
        key: "pass".into(),
        value: "secret".into(),
        hash: String::new(),
    }];
    let result =
        substitute_ephemerals("{{ephemeral.pass}} and {{ephemeral.pass}} again", &resolved);
    assert_eq!(result, "secret and secret again");
}

// ============================================================================
// FJ-3302: Provider Chain Resolution
// ============================================================================

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
fn resolve_with_file_provider() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("db-password"), "file-secret\n").unwrap();

    let chain = ProviderChain::new().with(Box::new(FileProvider::new(dir.path())));
    let params = vec![EphemeralParam {
        key: "db_pass".into(),
        provider_key: "db-password".into(),
    }];
    let resolved = resolve_ephemerals(&params, &chain).unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].value, "file-secret");
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
    let err = result.unwrap_err();
    assert!(err.contains("no provider resolved"), "err: {err}");
    assert!(err.contains("missing"), "err: {err}");
}

#[test]
fn resolve_multiple_params() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("key-a"), "val-a").unwrap();
    std::fs::write(dir.path().join("key-b"), "val-b").unwrap();

    let chain = ProviderChain::new().with(Box::new(FileProvider::new(dir.path())));
    let params = vec![
        EphemeralParam {
            key: "a".into(),
            provider_key: "key-a".into(),
        },
        EphemeralParam {
            key: "b".into(),
            provider_key: "key-b".into(),
        },
    ];
    let resolved = resolve_ephemerals(&params, &chain).unwrap();
    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved[0].key, "a");
    assert_eq!(resolved[1].key, "b");
}

#[test]
fn resolve_empty_params() {
    let chain = ProviderChain::new();
    let resolved = resolve_ephemerals(&[], &chain).unwrap();
    assert!(resolved.is_empty());
}

// ============================================================================
// FJ-3302: End-to-end Pipeline
// ============================================================================

#[test]
fn full_pipeline_resolve_hash_discard() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("db-pass"), "production-secret").unwrap();

    let chain = ProviderChain::new().with(Box::new(FileProvider::new(dir.path())));
    let params = vec![EphemeralParam {
        key: "db_pass".into(),
        provider_key: "db-pass".into(),
    }];

    // Step 1: Resolve
    let resolved = resolve_ephemerals(&params, &chain).unwrap();
    assert_eq!(resolved[0].value, "production-secret");

    // Step 2: Substitute into template
    let config = substitute_ephemerals("postgres://app:{{ephemeral.db_pass}}@db/prod", &resolved);
    assert_eq!(config, "postgres://app:production-secret@db/prod");

    // Step 3: Convert to records (discard plaintext)
    let records = to_records(&resolved);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].hash.len(), 64);

    // Step 4: Verify drift detection
    let resolved_again = resolve_ephemerals(&params, &chain).unwrap();
    let drift = check_drift(&resolved_again, &records);
    assert_eq!(drift[0].status, DriftStatus::Unchanged);
}

#[test]
fn pipeline_drift_detected_on_change() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("api-key"), "original-key").unwrap();

    let chain = ProviderChain::new().with(Box::new(FileProvider::new(dir.path())));
    let params = vec![EphemeralParam {
        key: "api_key".into(),
        provider_key: "api-key".into(),
    }];

    // First resolution
    let resolved = resolve_ephemerals(&params, &chain).unwrap();
    let records = to_records(&resolved);

    // Change the secret
    std::fs::write(dir.path().join("api-key"), "rotated-key").unwrap();

    // Re-resolve
    let resolved2 = resolve_ephemerals(&params, &chain).unwrap();
    let drift = check_drift(&resolved2, &records);
    assert_eq!(drift[0].status, DriftStatus::Changed);
}
