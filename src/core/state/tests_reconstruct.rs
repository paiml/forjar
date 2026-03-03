//! Tests for FJ-1280: Event-sourced state reconstruction.

use super::reconstruct::reconstruct_at;
use crate::core::types::{ProvenanceEvent, ResourceStatus, TimestampedEvent};
use tempfile::TempDir;

/// Helper: write a sequence of events to a machine's event log.
fn write_events(state_dir: &std::path::Path, machine: &str, events: &[TimestampedEvent]) {
    let dir = state_dir.join(machine);
    std::fs::create_dir_all(&dir).expect("mkdir");
    let path = dir.join("events.jsonl");
    let mut content = String::new();
    for e in events {
        content.push_str(&serde_json::to_string(e).expect("serialize"));
        content.push('\n');
    }
    std::fs::write(&path, content).expect("write events");
}

fn ts_event(ts: &str, event: ProvenanceEvent) -> TimestampedEvent {
    TimestampedEvent {
        ts: ts.to_string(),
        event,
    }
}

#[test]
fn reconstruct_at_midpoint() {
    let tmp = TempDir::new().expect("tempdir");
    let events = vec![
        ts_event(
            "2026-01-01T10:00:00Z",
            ProvenanceEvent::ApplyStarted {
                machine: "web".to_string(),
                run_id: "r-001".to_string(),
                forjar_version: "1.0.0".to_string(),
                operator: None,
                config_hash: None,
                param_count: None,
            },
        ),
        ts_event(
            "2026-01-01T10:00:01Z",
            ProvenanceEvent::ResourceConverged {
                machine: "web".to_string(),
                resource: "nginx".to_string(),
                duration_seconds: 1.5,
                hash: "abc123".to_string(),
            },
        ),
        ts_event(
            "2026-01-01T10:00:02Z",
            ProvenanceEvent::ResourceConverged {
                machine: "web".to_string(),
                resource: "app".to_string(),
                duration_seconds: 2.0,
                hash: "def456".to_string(),
            },
        ),
        // Second apply at a later time
        ts_event(
            "2026-01-02T10:00:00Z",
            ProvenanceEvent::ResourceConverged {
                machine: "web".to_string(),
                resource: "nginx".to_string(),
                duration_seconds: 0.5,
                hash: "updated123".to_string(),
            },
        ),
    ];
    write_events(tmp.path(), "web", &events);

    // Reconstruct at midpoint — should see first nginx hash, not second
    let lock = reconstruct_at(tmp.path(), "web", "2026-01-01T23:59:59Z").expect("reconstruct");
    assert_eq!(lock.resources.len(), 2);
    assert_eq!(lock.resources["nginx"].hash, "abc123");
    assert_eq!(lock.resources["app"].hash, "def456");
}

#[test]
fn reconstruct_at_latest() {
    let tmp = TempDir::new().expect("tempdir");
    let events = vec![
        ts_event(
            "2026-01-01T10:00:00Z",
            ProvenanceEvent::ResourceConverged {
                machine: "web".to_string(),
                resource: "nginx".to_string(),
                duration_seconds: 1.0,
                hash: "old-hash".to_string(),
            },
        ),
        ts_event(
            "2026-01-02T10:00:00Z",
            ProvenanceEvent::ResourceConverged {
                machine: "web".to_string(),
                resource: "nginx".to_string(),
                duration_seconds: 0.5,
                hash: "new-hash".to_string(),
            },
        ),
    ];
    write_events(tmp.path(), "web", &events);

    let lock = reconstruct_at(tmp.path(), "web", "2099-12-31T23:59:59Z").expect("reconstruct");
    assert_eq!(lock.resources["nginx"].hash, "new-hash");
}

#[test]
fn reconstruct_at_epoch_is_empty() {
    let tmp = TempDir::new().expect("tempdir");
    let events = vec![ts_event(
        "2026-01-01T10:00:00Z",
        ProvenanceEvent::ResourceConverged {
            machine: "web".to_string(),
            resource: "nginx".to_string(),
            duration_seconds: 1.0,
            hash: "abc".to_string(),
        },
    )];
    write_events(tmp.path(), "web", &events);

    // Before any events
    let lock = reconstruct_at(tmp.path(), "web", "2020-01-01T00:00:00Z").expect("reconstruct");
    assert!(lock.resources.is_empty());
}

#[test]
fn reconstruct_with_failure() {
    let tmp = TempDir::new().expect("tempdir");
    let events = vec![
        ts_event(
            "2026-01-01T10:00:00Z",
            ProvenanceEvent::ResourceConverged {
                machine: "web".to_string(),
                resource: "nginx".to_string(),
                duration_seconds: 1.0,
                hash: "abc".to_string(),
            },
        ),
        ts_event(
            "2026-01-01T10:00:01Z",
            ProvenanceEvent::ResourceFailed {
                machine: "web".to_string(),
                resource: "app".to_string(),
                error: "package not found".to_string(),
            },
        ),
    ];
    write_events(tmp.path(), "web", &events);

    let lock = reconstruct_at(tmp.path(), "web", "2099-01-01T00:00:00Z").expect("reconstruct");
    assert_eq!(lock.resources["nginx"].status, ResourceStatus::Converged);
    assert_eq!(lock.resources["app"].status, ResourceStatus::Failed);
}

#[test]
fn reconstruct_with_drift() {
    let tmp = TempDir::new().expect("tempdir");
    let events = vec![
        ts_event(
            "2026-01-01T10:00:00Z",
            ProvenanceEvent::ResourceConverged {
                machine: "web".to_string(),
                resource: "nginx".to_string(),
                duration_seconds: 1.0,
                hash: "original".to_string(),
            },
        ),
        ts_event(
            "2026-01-01T12:00:00Z",
            ProvenanceEvent::DriftDetected {
                machine: "web".to_string(),
                resource: "nginx".to_string(),
                expected_hash: "original".to_string(),
                actual_hash: "drifted".to_string(),
            },
        ),
    ];
    write_events(tmp.path(), "web", &events);

    let lock = reconstruct_at(tmp.path(), "web", "2099-01-01T00:00:00Z").expect("reconstruct");
    assert_eq!(lock.resources["nginx"].status, ResourceStatus::Drifted);
    assert_eq!(lock.resources["nginx"].hash, "drifted");
}

#[test]
fn reconstruct_reconverge_after_drift() {
    let tmp = TempDir::new().expect("tempdir");
    let events = vec![
        ts_event(
            "2026-01-01T10:00:00Z",
            ProvenanceEvent::ResourceConverged {
                machine: "web".to_string(),
                resource: "nginx".to_string(),
                duration_seconds: 1.0,
                hash: "v1".to_string(),
            },
        ),
        ts_event(
            "2026-01-01T12:00:00Z",
            ProvenanceEvent::DriftDetected {
                machine: "web".to_string(),
                resource: "nginx".to_string(),
                expected_hash: "v1".to_string(),
                actual_hash: "drifted".to_string(),
            },
        ),
        ts_event(
            "2026-01-01T14:00:00Z",
            ProvenanceEvent::ResourceConverged {
                machine: "web".to_string(),
                resource: "nginx".to_string(),
                duration_seconds: 0.8,
                hash: "v2".to_string(),
            },
        ),
    ];
    write_events(tmp.path(), "web", &events);

    let lock = reconstruct_at(tmp.path(), "web", "2099-01-01T00:00:00Z").expect("reconstruct");
    assert_eq!(lock.resources["nginx"].status, ResourceStatus::Converged);
    assert_eq!(lock.resources["nginx"].hash, "v2");
}

#[test]
fn reconstruct_missing_events() {
    let tmp = TempDir::new().expect("tempdir");
    let result = reconstruct_at(tmp.path(), "nonexistent", "2026-01-01T00:00:00Z");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no event log"));
}

#[test]
fn reconstruct_generator_contains_version() {
    let tmp = TempDir::new().expect("tempdir");
    let events = vec![ts_event(
        "2026-01-01T10:00:00Z",
        ProvenanceEvent::ResourceConverged {
            machine: "web".to_string(),
            resource: "nginx".to_string(),
            duration_seconds: 1.0,
            hash: "abc".to_string(),
        },
    )];
    write_events(tmp.path(), "web", &events);

    let lock = reconstruct_at(tmp.path(), "web", "2099-01-01T00:00:00Z").expect("reconstruct");
    assert!(lock.generator.contains("reconstructed"));
}
