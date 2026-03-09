//! FJ-3308: Secret access audit trail falsification.
//!
//! Popperian rejection criteria for:
//! - JSONL audit log append and read roundtrip
//! - All event types: resolve, inject, discard, rotate
//! - Event factory functions (make_*_event) populate correct fields
//! - Filter by key and event type
//! - Audit summary computation (counts, unique keys/providers)
//! - format_audit_summary output formatting
//! - SecretEventType Display trait
//! - Serde JSON roundtrip for all event types
//! - Empty audit log handling
//! - Multi-event accumulation in JSONL format
//!
//! Usage: cargo test --test falsification_secret_audit

use forjar::core::secret_audit::{
    append_audit, audit_summary, filter_by_key, filter_by_type, format_audit_summary,
    make_discard_event, make_inject_event, make_resolve_event, make_rotate_event, read_audit,
    AuditSummary, SecretAccessEvent, SecretEventType,
};

// ============================================================================
// FJ-3308: Event Factory Functions
// ============================================================================

#[test]
fn resolve_event_fields() {
    let event = make_resolve_event("db_pass", "env", "abc123", Some("web-01"));
    assert_eq!(event.key, "db_pass");
    assert_eq!(event.provider, "env");
    assert_eq!(event.value_hash, "abc123");
    assert_eq!(event.machine.as_deref(), Some("web-01"));
    assert_eq!(event.event_type, SecretEventType::Resolve);
    assert!(event.namespace.is_none());
}

#[test]
fn resolve_event_no_machine() {
    let event = make_resolve_event("api_key", "file", "h1", None);
    assert!(event.machine.is_none());
}

#[test]
fn resolve_event_has_pid() {
    let event = make_resolve_event("k", "env", "h", None);
    assert!(event.pid > 0);
}

#[test]
fn resolve_event_has_timestamp() {
    let event = make_resolve_event("k", "env", "h", None);
    assert!(!event.timestamp.is_empty());
    // ISO 8601 should have a 'T' separator
    assert!(event.timestamp.contains('T') || event.timestamp.contains('-'));
}

#[test]
fn inject_event_fields() {
    let event = make_inject_event("tls_key", "exec", "h2", "ns-forjar-1");
    assert_eq!(event.key, "tls_key");
    assert_eq!(event.provider, "exec");
    assert_eq!(event.value_hash, "h2");
    assert_eq!(event.namespace.as_deref(), Some("ns-forjar-1"));
    assert_eq!(event.event_type, SecretEventType::Inject);
    assert!(event.machine.is_none());
}

#[test]
fn discard_event_fields() {
    let event = make_discard_event("db_pass", "abc123");
    assert_eq!(event.key, "db_pass");
    assert_eq!(event.value_hash, "abc123");
    assert_eq!(event.event_type, SecretEventType::Discard);
    assert!(event.provider.is_empty());
    assert!(event.machine.is_none());
    assert!(event.namespace.is_none());
}

#[test]
fn rotate_event_fields() {
    let event = make_rotate_event("tls_cert", "exec", "old_hash", "new_hash");
    assert_eq!(event.key, "tls_cert");
    assert_eq!(event.provider, "exec");
    assert_eq!(event.value_hash, "new_hash");
    assert_eq!(event.event_type, SecretEventType::Rotate);
    assert_eq!(event.namespace.as_deref(), Some("rotated_from:old_hash"));
}

// ============================================================================
// FJ-3308: JSONL Append and Read
// ============================================================================

#[test]
fn append_and_read_single() {
    let dir = tempfile::tempdir().unwrap();
    let event = make_resolve_event("k1", "env", "h1", None);
    append_audit(dir.path(), &event).unwrap();

    let events = read_audit(dir.path()).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].key, "k1");
    assert_eq!(events[0].event_type, SecretEventType::Resolve);
}

#[test]
fn append_multiple_read_all() {
    let dir = tempfile::tempdir().unwrap();
    let e1 = make_resolve_event("k1", "env", "h1", None);
    let e2 = make_inject_event("k1", "env", "h1", "ns1");
    let e3 = make_discard_event("k1", "h1");
    let e4 = make_rotate_event("k2", "file", "old", "new");
    append_audit(dir.path(), &e1).unwrap();
    append_audit(dir.path(), &e2).unwrap();
    append_audit(dir.path(), &e3).unwrap();
    append_audit(dir.path(), &e4).unwrap();

    let events = read_audit(dir.path()).unwrap();
    assert_eq!(events.len(), 4);
    assert_eq!(events[0].event_type, SecretEventType::Resolve);
    assert_eq!(events[1].event_type, SecretEventType::Inject);
    assert_eq!(events[2].event_type, SecretEventType::Discard);
    assert_eq!(events[3].event_type, SecretEventType::Rotate);
}

#[test]
fn read_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let events = read_audit(dir.path()).unwrap();
    assert!(events.is_empty());
}

#[test]
fn jsonl_format_one_line_per_event() {
    let dir = tempfile::tempdir().unwrap();
    append_audit(dir.path(), &make_resolve_event("k1", "env", "h1", None)).unwrap();
    append_audit(dir.path(), &make_discard_event("k1", "h1")).unwrap();
    append_audit(dir.path(), &make_inject_event("k2", "file", "h2", "ns")).unwrap();

    let content = std::fs::read_to_string(dir.path().join("secret-audit.jsonl")).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3);
    // Each line is valid JSON
    for line in &lines {
        serde_json::from_str::<SecretAccessEvent>(line).unwrap();
    }
}

// ============================================================================
// FJ-3308: Filter by Key
// ============================================================================

#[test]
fn filter_by_key_returns_matching() {
    let events = vec![
        make_resolve_event("api_key", "env", "h1", None),
        make_resolve_event("db_pass", "file", "h2", None),
        make_discard_event("api_key", "h1"),
        make_inject_event("db_pass", "file", "h2", "ns"),
    ];
    let api_events = filter_by_key(&events, "api_key");
    assert_eq!(api_events.len(), 2);
    assert!(api_events.iter().all(|e| e.key == "api_key"));
}

#[test]
fn filter_by_key_no_match() {
    let events = vec![make_resolve_event("k1", "env", "h", None)];
    let filtered = filter_by_key(&events, "nonexistent");
    assert!(filtered.is_empty());
}

#[test]
fn filter_by_key_empty_events() {
    let events: Vec<SecretAccessEvent> = vec![];
    let filtered = filter_by_key(&events, "k");
    assert!(filtered.is_empty());
}

// ============================================================================
// FJ-3308: Filter by Type
// ============================================================================

#[test]
fn filter_by_type_resolve() {
    let events = vec![
        make_resolve_event("k", "env", "h", None),
        make_inject_event("k", "env", "h", "ns"),
        make_discard_event("k", "h"),
        make_rotate_event("k", "env", "old", "new"),
    ];
    let resolves = filter_by_type(&events, &SecretEventType::Resolve);
    assert_eq!(resolves.len(), 1);
    assert_eq!(resolves[0].event_type, SecretEventType::Resolve);
}

#[test]
fn filter_by_type_inject() {
    let events = vec![
        make_resolve_event("k", "env", "h", None),
        make_inject_event("k1", "env", "h1", "ns1"),
        make_inject_event("k2", "file", "h2", "ns2"),
    ];
    let injects = filter_by_type(&events, &SecretEventType::Inject);
    assert_eq!(injects.len(), 2);
}

#[test]
fn filter_by_type_discard() {
    let events = vec![
        make_discard_event("k1", "h1"),
        make_discard_event("k2", "h2"),
        make_resolve_event("k3", "env", "h3", None),
    ];
    let discards = filter_by_type(&events, &SecretEventType::Discard);
    assert_eq!(discards.len(), 2);
}

#[test]
fn filter_by_type_rotate() {
    let events = vec![
        make_rotate_event("k1", "exec", "old1", "new1"),
        make_resolve_event("k2", "env", "h", None),
    ];
    let rotations = filter_by_type(&events, &SecretEventType::Rotate);
    assert_eq!(rotations.len(), 1);
}

// ============================================================================
// FJ-3308: Audit Summary
// ============================================================================

#[test]
fn summary_counts_all_types() {
    let events = vec![
        make_resolve_event("k1", "env", "h1", None),
        make_resolve_event("k2", "file", "h2", None),
        make_inject_event("k1", "env", "h1", "ns1"),
        make_discard_event("k1", "h1"),
        make_rotate_event("k2", "file", "old", "new"),
    ];
    let summary = audit_summary(&events);
    assert_eq!(summary.total, 5);
    assert_eq!(summary.resolves, 2);
    assert_eq!(summary.injects, 1);
    assert_eq!(summary.discards, 1);
    assert_eq!(summary.rotations, 1);
}

#[test]
fn summary_unique_keys() {
    let events = vec![
        make_resolve_event("k1", "env", "h1", None),
        make_resolve_event("k1", "env", "h1", None), // duplicate key
        make_resolve_event("k2", "file", "h2", None),
        make_resolve_event("k3", "exec", "h3", None),
    ];
    let summary = audit_summary(&events);
    assert_eq!(summary.unique_keys, 3);
}

#[test]
fn summary_unique_providers() {
    let events = vec![
        make_resolve_event("k1", "env", "h1", None),
        make_resolve_event("k2", "env", "h2", None), // same provider
        make_resolve_event("k3", "file", "h3", None),
        make_inject_event("k4", "exec", "h4", "ns"),
    ];
    let summary = audit_summary(&events);
    assert_eq!(summary.unique_providers, 3);
}

#[test]
fn summary_empty_provider_not_counted() {
    let events = vec![make_discard_event("k", "h")]; // provider is empty
    let summary = audit_summary(&events);
    assert_eq!(summary.unique_providers, 0);
    assert_eq!(summary.unique_keys, 1);
}

#[test]
fn summary_empty_events() {
    let events: Vec<SecretAccessEvent> = vec![];
    let summary = audit_summary(&events);
    assert_eq!(summary.total, 0);
    assert_eq!(summary.resolves, 0);
    assert_eq!(summary.unique_keys, 0);
    assert_eq!(summary.unique_providers, 0);
}

// ============================================================================
// FJ-3308: Format Audit Summary
// ============================================================================

#[test]
fn format_summary_header() {
    let summary = AuditSummary {
        total: 42,
        resolves: 10,
        injects: 15,
        discards: 12,
        rotations: 5,
        unique_keys: 8,
        unique_providers: 3,
    };
    let text = format_audit_summary(&summary);
    assert!(text.contains("42 events"));
}

#[test]
fn format_summary_all_fields() {
    let summary = AuditSummary {
        total: 10,
        resolves: 4,
        injects: 3,
        discards: 2,
        rotations: 1,
        unique_keys: 3,
        unique_providers: 2,
    };
    let text = format_audit_summary(&summary);
    assert!(text.contains("Resolves:  4"));
    assert!(text.contains("Injects:   3"));
    assert!(text.contains("Discards:  2"));
    assert!(text.contains("Rotations: 1"));
    assert!(text.contains("Unique keys:      3"));
    assert!(text.contains("Unique providers:  2"));
}

// ============================================================================
// FJ-3308: SecretEventType Display
// ============================================================================

#[test]
fn event_type_display_resolve() {
    assert_eq!(SecretEventType::Resolve.to_string(), "resolve");
}

#[test]
fn event_type_display_inject() {
    assert_eq!(SecretEventType::Inject.to_string(), "inject");
}

#[test]
fn event_type_display_discard() {
    assert_eq!(SecretEventType::Discard.to_string(), "discard");
}

#[test]
fn event_type_display_rotate() {
    assert_eq!(SecretEventType::Rotate.to_string(), "rotate");
}

// ============================================================================
// FJ-3308: Serde Roundtrip
// ============================================================================

#[test]
fn serde_roundtrip_resolve() {
    let event = make_resolve_event("tls", "exec", "hash", Some("host-1"));
    let json = serde_json::to_string(&event).unwrap();
    let parsed: SecretAccessEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.key, "tls");
    assert_eq!(parsed.event_type, SecretEventType::Resolve);
    assert_eq!(parsed.machine.as_deref(), Some("host-1"));
}

#[test]
fn serde_roundtrip_inject() {
    let event = make_inject_event("api", "env", "h", "ns-prod");
    let json = serde_json::to_string(&event).unwrap();
    let parsed: SecretAccessEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.event_type, SecretEventType::Inject);
    assert_eq!(parsed.namespace.as_deref(), Some("ns-prod"));
}

#[test]
fn serde_roundtrip_discard() {
    let event = make_discard_event("token", "h");
    let json = serde_json::to_string(&event).unwrap();
    let parsed: SecretAccessEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.event_type, SecretEventType::Discard);
    assert!(parsed.provider.is_empty());
}

#[test]
fn serde_roundtrip_rotate() {
    let event = make_rotate_event("cert", "exec", "old", "new");
    let json = serde_json::to_string(&event).unwrap();
    let parsed: SecretAccessEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.event_type, SecretEventType::Rotate);
    assert_eq!(parsed.value_hash, "new");
}

// ============================================================================
// FJ-3308: Full Lifecycle Integration
// ============================================================================

#[test]
fn full_secret_lifecycle_audit() {
    let dir = tempfile::tempdir().unwrap();

    // 1. Resolve
    let resolve = make_resolve_event("db_pass", "file", "hash_v1", Some("web-01"));
    append_audit(dir.path(), &resolve).unwrap();

    // 2. Inject
    let inject = make_inject_event("db_pass", "file", "hash_v1", "ns-apply-1");
    append_audit(dir.path(), &inject).unwrap();

    // 3. Discard
    let discard = make_discard_event("db_pass", "hash_v1");
    append_audit(dir.path(), &discard).unwrap();

    // 4. Rotate
    let rotate = make_rotate_event("db_pass", "file", "hash_v1", "hash_v2");
    append_audit(dir.path(), &rotate).unwrap();

    // Verify full trail
    let events = read_audit(dir.path()).unwrap();
    assert_eq!(events.len(), 4);

    let summary = audit_summary(&events);
    assert_eq!(summary.total, 4);
    assert_eq!(summary.resolves, 1);
    assert_eq!(summary.injects, 1);
    assert_eq!(summary.discards, 1);
    assert_eq!(summary.rotations, 1);
    assert_eq!(summary.unique_keys, 1);
    // "file" from resolve/inject/rotate, empty from discard → 1 unique non-empty
    assert_eq!(summary.unique_providers, 1);

    // Filter for db_pass only
    let db_events = filter_by_key(&events, "db_pass");
    assert_eq!(db_events.len(), 4);

    // Filter for inject events
    let injects = filter_by_type(&events, &SecretEventType::Inject);
    assert_eq!(injects.len(), 1);
    assert_eq!(injects[0].namespace.as_deref(), Some("ns-apply-1"));
}

#[test]
fn multi_key_audit_trail() {
    let dir = tempfile::tempdir().unwrap();
    // Two different keys through the same lifecycle
    for key in &["db_pass", "api_key", "tls_cert"] {
        append_audit(
            dir.path(),
            &make_resolve_event(key, "env", &format!("hash_{key}"), None),
        )
        .unwrap();
    }

    let events = read_audit(dir.path()).unwrap();
    assert_eq!(events.len(), 3);

    let summary = audit_summary(&events);
    assert_eq!(summary.unique_keys, 3);
    assert_eq!(summary.unique_providers, 1); // all "env"
    assert_eq!(summary.resolves, 3);

    let db = filter_by_key(&events, "db_pass");
    assert_eq!(db.len(), 1);
    let tls = filter_by_key(&events, "tls_cert");
    assert_eq!(tls.len(), 1);
}
