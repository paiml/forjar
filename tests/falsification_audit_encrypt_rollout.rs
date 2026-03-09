//! FJ-3308/3303/3507: Secret audit trail, state encryption, and progressive
//! rollout falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-3308: Secret access audit
//!   - make_*_event: event construction
//!   - append_audit / read_audit: JSONL persistence roundtrip
//!   - filter_by_key / filter_by_type: event filtering
//!   - audit_summary / format_audit_summary: aggregate statistics
//! - FJ-3303: State encryption
//!   - hash_data: BLAKE3 determinism
//!   - keyed_hash / verify_keyed_hash: HMAC integrity
//!   - derive_key: passphrase-to-key derivation
//!   - create_metadata / verify_metadata: roundtrip integrity
//!   - write_metadata / read_metadata: filesystem persistence
//! - FJ-3507: Progressive rollout
//!   - plan_rollout: canary, percentage, all-at-once strategies
//!
//! Usage: cargo test --test falsification_audit_encrypt_rollout

use forjar::core::rollout::plan_rollout;
use forjar::core::secret_audit::{
    append_audit, audit_summary, filter_by_key, filter_by_type, format_audit_summary,
    make_discard_event, make_inject_event, make_resolve_event, make_rotate_event, read_audit,
    SecretEventType,
};
use forjar::core::state_encryption::{
    create_metadata, derive_key, hash_data, keyed_hash, read_metadata, verify_keyed_hash,
    verify_metadata, write_metadata,
};
use forjar::core::types::environment::RolloutConfig;

// ============================================================================
// FJ-3308: make_*_event construction
// ============================================================================

#[test]
fn audit_resolve_event_fields() {
    let e = make_resolve_event("db_pass", "env", "blake3:abc", Some("web-01"));
    assert_eq!(e.key, "db_pass");
    assert_eq!(e.provider, "env");
    assert_eq!(e.value_hash, "blake3:abc");
    assert_eq!(e.machine.as_deref(), Some("web-01"));
    assert_eq!(e.event_type, SecretEventType::Resolve);
    assert!(e.pid > 0);
    assert!(!e.timestamp.is_empty());
}

#[test]
fn audit_inject_event_has_namespace() {
    let e = make_inject_event("api_key", "file", "blake3:def", "ns-apply-1");
    assert_eq!(e.event_type, SecretEventType::Inject);
    assert_eq!(e.namespace.as_deref(), Some("ns-apply-1"));
}

#[test]
fn audit_discard_event_no_provider() {
    let e = make_discard_event("token", "blake3:ghi");
    assert_eq!(e.event_type, SecretEventType::Discard);
    assert!(e.provider.is_empty());
}

#[test]
fn audit_rotate_event_tracks_old_hash() {
    let e = make_rotate_event("tls_cert", "exec", "blake3:old", "blake3:new");
    assert_eq!(e.event_type, SecretEventType::Rotate);
    assert_eq!(e.value_hash, "blake3:new");
    assert_eq!(e.namespace.as_deref(), Some("rotated_from:blake3:old"));
}

// ============================================================================
// FJ-3308: append_audit / read_audit roundtrip
// ============================================================================

#[test]
fn audit_append_and_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let e1 = make_resolve_event("k1", "env", "h1", None);
    let e2 = make_inject_event("k1", "env", "h1", "ns");
    let e3 = make_discard_event("k1", "h1");
    append_audit(dir.path(), &e1).unwrap();
    append_audit(dir.path(), &e2).unwrap();
    append_audit(dir.path(), &e3).unwrap();

    let events = read_audit(dir.path()).unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].event_type, SecretEventType::Resolve);
    assert_eq!(events[1].event_type, SecretEventType::Inject);
    assert_eq!(events[2].event_type, SecretEventType::Discard);
}

#[test]
fn audit_read_empty_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let events = read_audit(dir.path()).unwrap();
    assert!(events.is_empty());
}

#[test]
fn audit_jsonl_format_valid() {
    let dir = tempfile::tempdir().unwrap();
    append_audit(dir.path(), &make_resolve_event("k", "env", "h", None)).unwrap();
    append_audit(dir.path(), &make_discard_event("k", "h")).unwrap();

    let content = std::fs::read_to_string(dir.path().join("secret-audit.jsonl")).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    // Each line is valid JSON
    for line in &lines {
        serde_json::from_str::<serde_json::Value>(line).unwrap();
    }
}

// ============================================================================
// FJ-3308: filter_by_key / filter_by_type
// ============================================================================

#[test]
fn audit_filter_by_key_selects_correct() {
    let events = vec![
        make_resolve_event("key_a", "env", "h1", None),
        make_resolve_event("key_b", "env", "h2", None),
        make_discard_event("key_a", "h1"),
    ];
    let filtered = filter_by_key(&events, "key_a");
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().all(|e| e.key == "key_a"));
}

#[test]
fn audit_filter_by_type_selects_correct() {
    let events = vec![
        make_resolve_event("k", "env", "h", None),
        make_inject_event("k", "env", "h", "ns"),
        make_discard_event("k", "h"),
    ];
    let resolves = filter_by_type(&events, &SecretEventType::Resolve);
    assert_eq!(resolves.len(), 1);
    let injects = filter_by_type(&events, &SecretEventType::Inject);
    assert_eq!(injects.len(), 1);
}

// ============================================================================
// FJ-3308: audit_summary / format_audit_summary
// ============================================================================

#[test]
fn audit_summary_counts_correctly() {
    let events = vec![
        make_resolve_event("k1", "env", "h1", None),
        make_resolve_event("k2", "file", "h2", None),
        make_inject_event("k1", "env", "h1", "ns"),
        make_discard_event("k1", "h1"),
        make_rotate_event("k2", "file", "old", "new"),
    ];
    let summary = audit_summary(&events);
    assert_eq!(summary.total, 5);
    assert_eq!(summary.resolves, 2);
    assert_eq!(summary.injects, 1);
    assert_eq!(summary.discards, 1);
    assert_eq!(summary.rotations, 1);
    assert_eq!(summary.unique_keys, 2);
    assert_eq!(summary.unique_providers, 2);
}

#[test]
fn audit_summary_empty_provider_not_counted() {
    let events = vec![make_discard_event("k", "h")];
    let summary = audit_summary(&events);
    assert_eq!(summary.unique_providers, 0);
}

#[test]
fn audit_format_summary_contains_counts() {
    let summary = forjar::core::secret_audit::AuditSummary {
        total: 10,
        resolves: 4,
        injects: 3,
        discards: 2,
        rotations: 1,
        unique_keys: 3,
        unique_providers: 2,
    };
    let text = format_audit_summary(&summary);
    assert!(text.contains("10 events"));
    assert!(text.contains("Resolves:  4"));
    assert!(text.contains("Rotations: 1"));
}

// ============================================================================
// FJ-3303: hash_data
// ============================================================================

#[test]
fn encrypt_hash_data_deterministic() {
    let h1 = hash_data(b"hello world");
    let h2 = hash_data(b"hello world");
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64); // BLAKE3 hex = 64 chars
}

#[test]
fn encrypt_hash_data_different_input() {
    assert_ne!(hash_data(b"aaa"), hash_data(b"bbb"));
}

// ============================================================================
// FJ-3303: keyed_hash / verify_keyed_hash
// ============================================================================

#[test]
fn encrypt_keyed_hash_deterministic() {
    let key = [42u8; 32];
    let h1 = keyed_hash(b"data", &key);
    let h2 = keyed_hash(b"data", &key);
    assert_eq!(h1, h2);
}

#[test]
fn encrypt_keyed_hash_different_key() {
    let k1 = [1u8; 32];
    let k2 = [2u8; 32];
    assert_ne!(keyed_hash(b"data", &k1), keyed_hash(b"data", &k2));
}

#[test]
fn encrypt_verify_keyed_hash_match() {
    let key = [99u8; 32];
    let h = keyed_hash(b"payload", &key);
    assert!(verify_keyed_hash(b"payload", &key, &h));
}

#[test]
fn encrypt_verify_keyed_hash_mismatch() {
    let key = [99u8; 32];
    assert!(!verify_keyed_hash(b"payload", &key, "wrong_hash"));
}

// ============================================================================
// FJ-3303: derive_key
// ============================================================================

#[test]
fn encrypt_derive_key_deterministic() {
    let k1 = derive_key("my-passphrase");
    let k2 = derive_key("my-passphrase");
    assert_eq!(k1, k2);
}

#[test]
fn encrypt_derive_key_different_passphrase() {
    assert_ne!(derive_key("aaa"), derive_key("bbb"));
}

#[test]
fn encrypt_derive_key_length_32() {
    assert_eq!(derive_key("test").len(), 32);
}

// ============================================================================
// FJ-3303: create_metadata / verify_metadata
// ============================================================================

#[test]
fn encrypt_metadata_roundtrip() {
    let key = derive_key("test-key");
    let plaintext = b"state lock yaml content";
    let ciphertext = b"encrypted-blob-here";
    let meta = create_metadata(plaintext, ciphertext, &key);
    assert_eq!(meta.version, 1);
    assert!(!meta.plaintext_hash.is_empty());
    assert!(!meta.ciphertext_hmac.is_empty());
    assert!(!meta.encrypted_at.is_empty());
    assert!(verify_metadata(&meta, ciphertext, &key));
}

#[test]
fn encrypt_metadata_tamper_detects() {
    let key = derive_key("test-key");
    let meta = create_metadata(b"plain", b"cipher", &key);
    // Tamper with ciphertext
    assert!(!verify_metadata(&meta, b"tampered", &key));
}

// ============================================================================
// FJ-3303: write_metadata / read_metadata
// ============================================================================

#[test]
fn encrypt_metadata_write_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.lock.yaml");
    let key = derive_key("test");
    let meta = create_metadata(b"data", b"enc", &key);
    write_metadata(&path, &meta).unwrap();
    let loaded = read_metadata(&path).unwrap();
    assert_eq!(loaded.version, meta.version);
    assert_eq!(loaded.plaintext_hash, meta.plaintext_hash);
    assert_eq!(loaded.ciphertext_hmac, meta.ciphertext_hmac);
}

// ============================================================================
// FJ-3507: plan_rollout — canary strategy
// ============================================================================

#[test]
fn rollout_canary_default() {
    let config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 1,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![50, 100],
    };
    let steps = plan_rollout(&config, 10);
    assert!(!steps.is_empty());
    // First step should be canary (1 machine)
    assert_eq!(steps[0].machine_indices.len(), 1);
    assert_eq!(steps[0].index, 0);
}

#[test]
fn rollout_canary_zero_machines() {
    let config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 1,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![],
    };
    let steps = plan_rollout(&config, 0);
    assert!(steps.is_empty());
}

#[test]
fn rollout_canary_more_than_total() {
    let config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 100,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![],
    };
    // canary_count > total => capped at total
    let steps = plan_rollout(&config, 5);
    assert_eq!(steps[0].machine_indices.len(), 5);
}

// ============================================================================
// FJ-3507: plan_rollout — percentage strategy
// ============================================================================

#[test]
fn rollout_percentage_strategy() {
    let config = RolloutConfig {
        strategy: "percentage".into(),
        canary_count: 0,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![25, 50, 100],
    };
    let steps = plan_rollout(&config, 8);
    assert!(!steps.is_empty());
    // Steps should progressively include more machines
    let first_count = steps[0].machine_indices.len();
    assert!(
        first_count > 0,
        "first step should deploy to at least 1 machine"
    );
}

// ============================================================================
// FJ-3507: plan_rollout — all-at-once
// ============================================================================

#[test]
fn rollout_all_at_once() {
    let config = RolloutConfig {
        strategy: "all-at-once".into(),
        canary_count: 0,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![],
    };
    let steps = plan_rollout(&config, 5);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].machine_indices.len(), 5);
    assert_eq!(steps[0].percentage, 100);
}

#[test]
fn rollout_unknown_strategy_falls_back_to_all() {
    let config = RolloutConfig {
        strategy: "unknown-xyz".into(),
        canary_count: 0,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![],
    };
    let steps = plan_rollout(&config, 3);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].machine_indices.len(), 3);
}
