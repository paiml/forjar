//! FJ-3308: Secret access audit trail.
//!
//! Logs every secret access (resolve, read, inject) to a JSONL audit log.
//! Audit entries track: who accessed the secret, which provider resolved it,
//! timestamp, and the BLAKE3 hash of the value (never plaintext).

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// A secret access audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretAccessEvent {
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Event type: resolve, inject, discard, rotate.
    pub event_type: SecretEventType,
    /// Secret key name.
    pub key: String,
    /// Provider that resolved the secret (env, file, exec, age).
    pub provider: String,
    /// BLAKE3 hash of the secret value (never store plaintext).
    pub value_hash: String,
    /// Machine where the access occurred.
    #[serde(default)]
    pub machine: Option<String>,
    /// Process ID that accessed the secret.
    pub pid: u32,
    /// Namespace the secret was injected into (if applicable).
    #[serde(default)]
    pub namespace: Option<String>,
}

/// Types of secret access events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretEventType {
    /// Secret resolved from provider.
    Resolve,
    /// Secret injected into process environment.
    Inject,
    /// Secret value discarded from memory.
    Discard,
    /// Secret key rotated.
    Rotate,
}

impl std::fmt::Display for SecretEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Resolve => "resolve",
            Self::Inject => "inject",
            Self::Discard => "discard",
            Self::Rotate => "rotate",
        };
        write!(f, "{s}")
    }
}

const AUDIT_FILENAME: &str = "secret-audit.jsonl";

/// Append a secret access event to the audit log.
pub fn append_audit(state_dir: &Path, event: &SecretAccessEvent) -> Result<(), String> {
    let path = state_dir.join(AUDIT_FILENAME);
    let line = serde_json::to_string(event).map_err(|e| format!("serialize secret audit: {e}"))?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open {}: {e}", path.display()))?;

    writeln!(file, "{line}").map_err(|e| format!("write {}: {e}", path.display()))
}

/// Read all secret access events from the audit log.
pub fn read_audit(state_dir: &Path) -> Result<Vec<SecretAccessEvent>, String> {
    let path = state_dir.join(AUDIT_FILENAME);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut events = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let event: SecretAccessEvent =
            serde_json::from_str(line).map_err(|e| format!("parse audit entry: {e}"))?;
        events.push(event);
    }
    Ok(events)
}

/// Create a resolve event for a secret key.
pub fn make_resolve_event(
    key: &str,
    provider: &str,
    value_hash: &str,
    machine: Option<&str>,
) -> SecretAccessEvent {
    SecretAccessEvent {
        timestamp: crate::tripwire::eventlog::now_iso8601(),
        event_type: SecretEventType::Resolve,
        key: key.to_string(),
        provider: provider.to_string(),
        value_hash: value_hash.to_string(),
        machine: machine.map(|s| s.to_string()),
        pid: std::process::id(),
        namespace: None,
    }
}

/// Create an inject event (secret pushed to namespace/environment).
pub fn make_inject_event(
    key: &str,
    provider: &str,
    value_hash: &str,
    namespace: &str,
) -> SecretAccessEvent {
    SecretAccessEvent {
        timestamp: crate::tripwire::eventlog::now_iso8601(),
        event_type: SecretEventType::Inject,
        key: key.to_string(),
        provider: provider.to_string(),
        value_hash: value_hash.to_string(),
        machine: None,
        pid: std::process::id(),
        namespace: Some(namespace.to_string()),
    }
}

/// Create a discard event (secret cleared from memory).
pub fn make_discard_event(key: &str, value_hash: &str) -> SecretAccessEvent {
    SecretAccessEvent {
        timestamp: crate::tripwire::eventlog::now_iso8601(),
        event_type: SecretEventType::Discard,
        key: key.to_string(),
        provider: String::new(),
        value_hash: value_hash.to_string(),
        machine: None,
        pid: std::process::id(),
        namespace: None,
    }
}

/// Create a rotate event (secret key rotated to new value).
pub fn make_rotate_event(
    key: &str,
    provider: &str,
    old_hash: &str,
    new_hash: &str,
) -> SecretAccessEvent {
    SecretAccessEvent {
        timestamp: crate::tripwire::eventlog::now_iso8601(),
        event_type: SecretEventType::Rotate,
        key: key.to_string(),
        provider: provider.to_string(),
        // Store new hash as value_hash, old hash in namespace for rotation tracking
        value_hash: new_hash.to_string(),
        machine: None,
        pid: std::process::id(),
        namespace: Some(format!("rotated_from:{old_hash}")),
    }
}

/// Filter audit events by key.
pub fn filter_by_key<'a>(events: &'a [SecretAccessEvent], key: &str) -> Vec<&'a SecretAccessEvent> {
    events.iter().filter(|e| e.key == key).collect()
}

/// Filter audit events by event type.
pub fn filter_by_type<'a>(
    events: &'a [SecretAccessEvent],
    event_type: &SecretEventType,
) -> Vec<&'a SecretAccessEvent> {
    events
        .iter()
        .filter(|e| &e.event_type == event_type)
        .collect()
}

/// Summary of secret access audit.
#[derive(Debug, Clone)]
pub struct AuditSummary {
    /// Total events.
    pub total: usize,
    /// Resolve events.
    pub resolves: usize,
    /// Inject events.
    pub injects: usize,
    /// Discard events.
    pub discards: usize,
    /// Rotate events.
    pub rotations: usize,
    /// Unique keys accessed.
    pub unique_keys: usize,
    /// Unique providers used.
    pub unique_providers: usize,
}

/// Compute summary of audit events.
pub fn audit_summary(events: &[SecretAccessEvent]) -> AuditSummary {
    use std::collections::HashSet;
    let mut keys = HashSet::new();
    let mut providers = HashSet::new();
    let mut resolves = 0;
    let mut injects = 0;
    let mut discards = 0;
    let mut rotations = 0;

    for event in events {
        keys.insert(&event.key);
        if !event.provider.is_empty() {
            providers.insert(&event.provider);
        }
        match event.event_type {
            SecretEventType::Resolve => resolves += 1,
            SecretEventType::Inject => injects += 1,
            SecretEventType::Discard => discards += 1,
            SecretEventType::Rotate => rotations += 1,
        }
    }

    AuditSummary {
        total: events.len(),
        resolves,
        injects,
        discards,
        rotations,
        unique_keys: keys.len(),
        unique_providers: providers.len(),
    }
}

/// Format audit summary as human-readable text.
pub fn format_audit_summary(summary: &AuditSummary) -> String {
    let mut lines = Vec::new();
    lines.push(format!("Secret Access Audit: {} events", summary.total));
    lines.push(format!("  Resolves:  {}", summary.resolves));
    lines.push(format!("  Injects:   {}", summary.injects));
    lines.push(format!("  Discards:  {}", summary.discards));
    lines.push(format!("  Rotations: {}", summary.rotations));
    lines.push(format!("  Unique keys:      {}", summary.unique_keys));
    lines.push(format!("  Unique providers:  {}", summary.unique_providers));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn append_and_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let event = make_resolve_event("db_pass", "env", "abc123hash", Some("web-01"));
        append_audit(dir.path(), &event).unwrap();

        let events = read_audit(dir.path()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].key, "db_pass");
        assert_eq!(events[0].provider, "env");
        assert_eq!(events[0].event_type, SecretEventType::Resolve);
    }

    #[test]
    fn multiple_event_types() {
        let dir = TempDir::new().unwrap();
        let e1 = make_resolve_event("api_key", "file", "hash1", None);
        let e2 = make_inject_event("api_key", "file", "hash1", "ns-forjar-1");
        let e3 = make_discard_event("api_key", "hash1");
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
    fn rotate_event() {
        let dir = TempDir::new().unwrap();
        let event = make_rotate_event("tls_cert", "exec", "old_hash", "new_hash");
        append_audit(dir.path(), &event).unwrap();

        let events = read_audit(dir.path()).unwrap();
        assert_eq!(events[0].event_type, SecretEventType::Rotate);
        assert_eq!(events[0].value_hash, "new_hash");
        assert_eq!(
            events[0].namespace.as_deref(),
            Some("rotated_from:old_hash")
        );
    }

    #[test]
    fn read_empty_audit() {
        let dir = TempDir::new().unwrap();
        let events = read_audit(dir.path()).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn filter_by_key_works() {
        let events = vec![
            make_resolve_event("key_a", "env", "h1", None),
            make_resolve_event("key_b", "env", "h2", None),
            make_discard_event("key_a", "h1"),
        ];
        let filtered = filter_by_key(&events, "key_a");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_by_type_works() {
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

    #[test]
    fn audit_summary_counts() {
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
        assert_eq!(summary.unique_keys, 2);
        assert_eq!(summary.unique_providers, 2);
    }

    #[test]
    fn format_summary_output() {
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
        assert!(text.contains("10 events"));
        assert!(text.contains("Resolves:  4"));
        assert!(text.contains("Rotations: 1"));
    }

    #[test]
    fn event_type_display() {
        assert_eq!(SecretEventType::Resolve.to_string(), "resolve");
        assert_eq!(SecretEventType::Inject.to_string(), "inject");
        assert_eq!(SecretEventType::Discard.to_string(), "discard");
        assert_eq!(SecretEventType::Rotate.to_string(), "rotate");
    }

    #[test]
    fn serde_roundtrip() {
        let event = make_inject_event("tls_key", "exec", "blake3hash", "ns-apply");
        let json = serde_json::to_string(&event).unwrap();
        let parsed: SecretAccessEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.key, "tls_key");
        assert_eq!(parsed.event_type, SecretEventType::Inject);
        assert_eq!(parsed.namespace.as_deref(), Some("ns-apply"));
    }

    #[test]
    fn audit_jsonl_format() {
        let dir = TempDir::new().unwrap();
        let e1 = make_resolve_event("k1", "env", "h1", None);
        let e2 = make_discard_event("k1", "h1");
        append_audit(dir.path(), &e1).unwrap();
        append_audit(dir.path(), &e2).unwrap();

        let content = std::fs::read_to_string(dir.path().join("secret-audit.jsonl")).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            serde_json::from_str::<SecretAccessEvent>(line).unwrap();
        }
    }

    #[test]
    fn empty_provider_not_counted() {
        let events = vec![make_discard_event("k", "h")];
        let summary = audit_summary(&events);
        assert_eq!(summary.unique_providers, 0);
    }

    #[test]
    fn make_resolve_populates_pid() {
        let event = make_resolve_event("k", "env", "h", None);
        assert!(event.pid > 0);
    }

    #[test]
    fn make_inject_has_namespace() {
        let event = make_inject_event("k", "env", "h", "ns-test");
        assert_eq!(event.namespace.as_deref(), Some("ns-test"));
    }
}
