//! FJ-1280/3107: State reconstruction and rulebook event log falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1280: Event-sourced state reconstruction
//!   - reconstruct_at: replay events up to timestamp
//!   - Missing event log returns error
//!   - ResourceConverged/Failed/DriftDetected events
//!   - Timestamp cutoff respects ordering
//! - FJ-3107: Rulebook event log (JSONL)
//!   - append_entry/read_entries roundtrip
//!   - make_entry field mapping
//!   - Empty log, multiple entries
//!   - JSONL format validation
//!
//! Usage: cargo test --test falsification_reconstruct_rulebook

use forjar::core::state::reconstruct;
use forjar::core::state::rulebook_log::{self, RulebookLogEntry};
use forjar::core::types::*;
use std::collections::HashMap;

// ============================================================================
// Helpers
// ============================================================================

fn make_state_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    (dir, state_dir)
}

fn write_events(state_dir: &std::path::Path, machine: &str, events: &[&str]) {
    let machine_dir = state_dir.join(machine);
    std::fs::create_dir_all(&machine_dir).unwrap();
    let content = events.join("\n") + "\n";
    std::fs::write(machine_dir.join("events.jsonl"), content).unwrap();
}

fn sample_infra_event() -> InfraEvent {
    InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web-01".into()),
        payload: HashMap::new(),
    }
}

// ============================================================================
// FJ-1280: reconstruct_at — missing event log
// ============================================================================

#[test]
fn reconstruct_missing_event_log_returns_error() {
    let (_dir, state_dir) = make_state_dir();
    let result = reconstruct::reconstruct_at(&state_dir, "nonexistent", "2026-01-01T00:00:00Z");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no event log"));
}

// ============================================================================
// FJ-1280: reconstruct_at — ResourceConverged replay
// ============================================================================

#[test]
fn reconstruct_converged_resource() {
    let (_dir, state_dir) = make_state_dir();
    let event = serde_json::json!({
        "ts": "2026-03-09T10:00:00Z",
        "event": "resource_converged",
        "machine": "web",
        "resource": "nginx",
        "duration_seconds": 1.5,
        "hash": "abc123"
    });
    write_events(&state_dir, "web", &[&event.to_string()]);

    let lock = reconstruct::reconstruct_at(&state_dir, "web", "2026-03-09T23:59:59Z").unwrap();
    assert_eq!(lock.machine, "web");
    assert_eq!(lock.schema, "1.0");
    assert!(lock.resources.contains_key("nginx"));
    let r = &lock.resources["nginx"];
    assert_eq!(r.status, ResourceStatus::Converged);
    assert_eq!(r.hash, "abc123");
    assert_eq!(r.duration_seconds, Some(1.5));
}

// ============================================================================
// FJ-1280: reconstruct_at — ResourceFailed replay
// ============================================================================

#[test]
fn reconstruct_failed_resource() {
    let (_dir, state_dir) = make_state_dir();
    let event = serde_json::json!({
        "ts": "2026-03-09T10:00:00Z",
        "event": "resource_failed",
        "machine": "web",
        "resource": "broken-svc",
        "error": "exit code 1"
    });
    write_events(&state_dir, "web", &[&event.to_string()]);

    let lock = reconstruct::reconstruct_at(&state_dir, "web", "2026-03-09T23:59:59Z").unwrap();
    assert!(lock.resources.contains_key("broken-svc"));
    let r = &lock.resources["broken-svc"];
    assert_eq!(r.status, ResourceStatus::Failed);
    assert!(r.hash.is_empty());
}

// ============================================================================
// FJ-1280: reconstruct_at — DriftDetected replay
// ============================================================================

#[test]
fn reconstruct_drift_updates_status() {
    let (_dir, state_dir) = make_state_dir();
    let converged = serde_json::json!({
        "ts": "2026-03-09T10:00:00Z",
        "event": "resource_converged",
        "machine": "web",
        "resource": "cfg",
        "duration_seconds": 0.5,
        "hash": "original-hash"
    });
    let drifted = serde_json::json!({
        "ts": "2026-03-09T11:00:00Z",
        "event": "drift_detected",
        "machine": "web",
        "resource": "cfg",
        "expected_hash": "original-hash",
        "actual_hash": "drifted-hash"
    });
    write_events(
        &state_dir,
        "web",
        &[&converged.to_string(), &drifted.to_string()],
    );

    let lock = reconstruct::reconstruct_at(&state_dir, "web", "2026-03-09T23:59:59Z").unwrap();
    let r = &lock.resources["cfg"];
    assert_eq!(r.status, ResourceStatus::Drifted);
    assert_eq!(r.hash, "drifted-hash");
}

// ============================================================================
// FJ-1280: reconstruct_at — timestamp cutoff
// ============================================================================

#[test]
fn reconstruct_respects_timestamp_cutoff() {
    let (_dir, state_dir) = make_state_dir();
    let early = serde_json::json!({
        "ts": "2026-03-09T10:00:00Z",
        "event": "resource_converged",
        "machine": "web",
        "resource": "early-pkg",
        "duration_seconds": 0.1,
        "hash": "h1"
    });
    let late = serde_json::json!({
        "ts": "2026-03-09T20:00:00Z",
        "event": "resource_converged",
        "machine": "web",
        "resource": "late-pkg",
        "duration_seconds": 0.2,
        "hash": "h2"
    });
    write_events(&state_dir, "web", &[&early.to_string(), &late.to_string()]);

    // Reconstruct at 15:00 — should only see early-pkg
    let lock = reconstruct::reconstruct_at(&state_dir, "web", "2026-03-09T15:00:00Z").unwrap();
    assert!(lock.resources.contains_key("early-pkg"));
    assert!(!lock.resources.contains_key("late-pkg"));
}

// ============================================================================
// FJ-1280: reconstruct_at — empty events
// ============================================================================

#[test]
fn reconstruct_empty_events_returns_empty_lock() {
    let (_dir, state_dir) = make_state_dir();
    write_events(&state_dir, "web", &[]);

    let lock = reconstruct::reconstruct_at(&state_dir, "web", "2026-03-09T23:59:59Z").unwrap();
    assert!(lock.resources.is_empty());
    assert_eq!(lock.machine, "web");
}

// ============================================================================
// FJ-1280: reconstruct_at — ApplyStarted updates hostname
// ============================================================================

#[test]
fn reconstruct_apply_started_sets_hostname() {
    let (_dir, state_dir) = make_state_dir();
    let event = serde_json::json!({
        "ts": "2026-03-09T10:00:00Z",
        "event": "apply_started",
        "machine": "web-production-01",
        "run_id": "run-123",
        "forjar_version": "1.0.0"
    });
    write_events(&state_dir, "web", &[&event.to_string()]);

    let lock = reconstruct::reconstruct_at(&state_dir, "web", "2026-03-09T23:59:59Z").unwrap();
    assert_eq!(lock.hostname, "web-production-01");
}

// ============================================================================
// FJ-1280: reconstruct_at — multiple resources
// ============================================================================

#[test]
fn reconstruct_multiple_resources() {
    let (_dir, state_dir) = make_state_dir();
    let e1 = serde_json::json!({
        "ts": "2026-03-09T10:00:00Z",
        "event": "resource_converged",
        "machine": "web",
        "resource": "nginx",
        "duration_seconds": 1.0,
        "hash": "h-nginx"
    });
    let e2 = serde_json::json!({
        "ts": "2026-03-09T10:01:00Z",
        "event": "resource_converged",
        "machine": "web",
        "resource": "mysql",
        "duration_seconds": 2.0,
        "hash": "h-mysql"
    });
    write_events(&state_dir, "web", &[&e1.to_string(), &e2.to_string()]);

    let lock = reconstruct::reconstruct_at(&state_dir, "web", "2026-03-09T23:59:59Z").unwrap();
    assert_eq!(lock.resources.len(), 2);
    assert_eq!(lock.resources["nginx"].hash, "h-nginx");
    assert_eq!(lock.resources["mysql"].hash, "h-mysql");
}

// ============================================================================
// FJ-1280: reconstruct_at — generated_at field
// ============================================================================

#[test]
fn reconstruct_generated_at_uses_last_event_ts() {
    let (_dir, state_dir) = make_state_dir();
    let event = serde_json::json!({
        "ts": "2026-03-09T10:30:00Z",
        "event": "resource_converged",
        "machine": "web",
        "resource": "pkg",
        "duration_seconds": 0.5,
        "hash": "h1"
    });
    write_events(&state_dir, "web", &[&event.to_string()]);

    let lock = reconstruct::reconstruct_at(&state_dir, "web", "2026-03-09T23:59:59Z").unwrap();
    assert_eq!(lock.generated_at, "2026-03-09T10:30:00Z");
}

#[test]
fn reconstruct_generated_at_uses_target_when_no_events() {
    let (_dir, state_dir) = make_state_dir();
    write_events(&state_dir, "web", &[]);

    let lock = reconstruct::reconstruct_at(&state_dir, "web", "2026-03-09T15:00:00Z").unwrap();
    assert_eq!(lock.generated_at, "2026-03-09T15:00:00Z");
}

// ============================================================================
// FJ-3107: rulebook_log — append_entry / read_entries roundtrip
// ============================================================================

#[test]
fn rulebook_append_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let entry =
        rulebook_log::make_entry(&sample_infra_event(), "config-repair", "apply", true, None);
    rulebook_log::append_entry(dir.path(), &entry).unwrap();

    let entries = rulebook_log::read_entries(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].rulebook, "config-repair");
    assert_eq!(entries[0].action_type, "apply");
    assert!(entries[0].success);
    assert!(entries[0].error.is_none());
}

// ============================================================================
// FJ-3107: rulebook_log — multiple entries
// ============================================================================

#[test]
fn rulebook_multiple_entries() {
    let dir = tempfile::tempdir().unwrap();
    let e1 = rulebook_log::make_entry(&sample_infra_event(), "rule-a", "apply", true, None);
    let e2 = rulebook_log::make_entry(
        &sample_infra_event(),
        "rule-b",
        "script",
        false,
        Some("exit code 1".into()),
    );
    rulebook_log::append_entry(dir.path(), &e1).unwrap();
    rulebook_log::append_entry(dir.path(), &e2).unwrap();

    let entries = rulebook_log::read_entries(dir.path()).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1].rulebook, "rule-b");
    assert!(!entries[1].success);
    assert_eq!(entries[1].error.as_deref(), Some("exit code 1"));
}

// ============================================================================
// FJ-3107: rulebook_log — empty log
// ============================================================================

#[test]
fn rulebook_read_empty_log() {
    let dir = tempfile::tempdir().unwrap();
    let entries = rulebook_log::read_entries(dir.path()).unwrap();
    assert!(entries.is_empty());
}

// ============================================================================
// FJ-3107: rulebook_log — make_entry field mapping
// ============================================================================

#[test]
fn rulebook_make_entry_maps_all_fields() {
    let event = InfraEvent {
        event_type: EventType::CronFired,
        timestamp: "2026-03-09T00:00:00Z".into(),
        machine: Some("db-01".into()),
        payload: HashMap::new(),
    };
    let entry = rulebook_log::make_entry(&event, "cleanup", "script", true, None);
    assert_eq!(entry.event_type, EventType::CronFired);
    assert_eq!(entry.machine.as_deref(), Some("db-01"));
    assert_eq!(entry.timestamp, "2026-03-09T00:00:00Z");
    assert_eq!(entry.rulebook, "cleanup");
    assert_eq!(entry.action_type, "script");
    assert!(entry.success);
}

#[test]
fn rulebook_make_entry_no_machine() {
    let event = InfraEvent {
        event_type: EventType::Manual,
        timestamp: "2026-03-09T06:00:00Z".into(),
        machine: None,
        payload: HashMap::new(),
    };
    let entry = rulebook_log::make_entry(&event, "manual-run", "apply", true, None);
    assert!(entry.machine.is_none());
}

#[test]
fn rulebook_make_entry_with_error() {
    let entry = rulebook_log::make_entry(
        &sample_infra_event(),
        "broken",
        "apply",
        false,
        Some("timeout after 30s".into()),
    );
    assert!(!entry.success);
    assert_eq!(entry.error.as_deref(), Some("timeout after 30s"));
}

// ============================================================================
// FJ-3107: rulebook_log — JSONL format validation
// ============================================================================

#[test]
fn rulebook_log_is_jsonl() {
    let dir = tempfile::tempdir().unwrap();
    let entry = rulebook_log::make_entry(&sample_infra_event(), "r1", "apply", true, None);
    rulebook_log::append_entry(dir.path(), &entry).unwrap();
    rulebook_log::append_entry(dir.path(), &entry).unwrap();

    let content = std::fs::read_to_string(dir.path().join("rulebook-events.jsonl")).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        serde_json::from_str::<RulebookLogEntry>(line).unwrap();
    }
}

// ============================================================================
// FJ-3107: RulebookLogEntry serde roundtrip
// ============================================================================

#[test]
fn rulebook_entry_serde_json_roundtrip() {
    let entry = rulebook_log::make_entry(&sample_infra_event(), "test-rule", "notify", true, None);
    let json = serde_json::to_string(&entry).unwrap();
    let parsed: RulebookLogEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.rulebook, "test-rule");
    assert_eq!(parsed.action_type, "notify");
    assert!(parsed.success);
}

// ============================================================================
// FJ-3107: Event types in rulebook log
// ============================================================================

#[test]
fn rulebook_all_event_types() {
    let event_types = vec![
        EventType::FileChanged,
        EventType::ProcessExit,
        EventType::CronFired,
        EventType::WebhookReceived,
        EventType::MetricThreshold,
        EventType::Manual,
    ];
    for et in event_types {
        let event = InfraEvent {
            event_type: et.clone(),
            timestamp: "2026-03-09T12:00:00Z".into(),
            machine: None,
            payload: HashMap::new(),
        };
        let entry = rulebook_log::make_entry(&event, "test", "apply", true, None);
        assert_eq!(entry.event_type, et);
    }
}
