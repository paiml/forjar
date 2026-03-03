use super::eventlog::*;
use crate::core::types::ProvenanceEvent;
use std::path::{Path, PathBuf};

#[test]
fn test_fj015_timestamp_day_range() {
    let ts = now_iso8601();
    let day: u32 = ts[8..10].parse().unwrap();
    assert!((1..=31).contains(&day), "day should be 1-31: {}", day);
}

#[test]
fn test_fj015_timestamp_hour_range() {
    let ts = now_iso8601();
    let hour: u32 = ts[11..13].parse().unwrap();
    assert!(hour < 24, "hour should be 0-23: {}", hour);
}

#[test]
fn test_fj015_append_resource_started_event() {
    let dir = tempfile::tempdir().unwrap();
    let event = ProvenanceEvent::ResourceStarted {
        machine: "m".to_string(),
        resource: "nginx-config".to_string(),
        action: "UPDATE".to_string(),
    };
    append_event(dir.path(), "m", event).unwrap();
    let content = std::fs::read_to_string(dir.path().join("m/events.jsonl")).unwrap();
    assert!(content.contains("resource_started"));
    assert!(content.contains("UPDATE"));
    assert!(content.contains("nginx-config"));
}

#[test]
fn test_fj015_append_all_event_types_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let events: Vec<(ProvenanceEvent, &str)> = vec![
        (
            ProvenanceEvent::ApplyStarted {
                machine: "m".to_string(),
                run_id: "r-1".to_string(),
                forjar_version: "0.1.0".to_string(),
                operator: None,
                config_hash: None,
            },
            "apply_started",
        ),
        (
            ProvenanceEvent::ResourceStarted {
                machine: "m".to_string(),
                resource: "r".to_string(),
                action: "CREATE".to_string(),
            },
            "resource_started",
        ),
        (
            ProvenanceEvent::ResourceConverged {
                machine: "m".to_string(),
                resource: "r".to_string(),
                duration_seconds: 0.5,
                hash: "blake3:abc".to_string(),
            },
            "resource_converged",
        ),
        (
            ProvenanceEvent::ResourceFailed {
                machine: "m".to_string(),
                resource: "r".to_string(),
                error: "fail".to_string(),
            },
            "resource_failed",
        ),
        (
            ProvenanceEvent::ApplyCompleted {
                machine: "m".to_string(),
                run_id: "r-1".to_string(),
                resources_converged: 1,
                resources_unchanged: 0,
                resources_failed: 0,
                total_seconds: 1.0,
            },
            "apply_completed",
        ),
        (
            ProvenanceEvent::DriftDetected {
                machine: "m".to_string(),
                resource: "r".to_string(),
                expected_hash: "blake3:aaa".to_string(),
                actual_hash: "blake3:bbb".to_string(),
            },
            "drift_detected",
        ),
    ];
    for (event, expected_tag) in events {
        append_event(dir.path(), "m", event).unwrap();
        let content = std::fs::read_to_string(dir.path().join("m/events.jsonl")).unwrap();
        assert!(
            content.contains(expected_tag),
            "event log should contain tag '{}': {}",
            expected_tag,
            content,
        );
    }
}

#[test]
fn test_fj015_generate_run_id_prefix_length() {
    for _ in 0..10 {
        let id = generate_run_id();
        assert_eq!(
            id.len(),
            14,
            "run ID should be r- + 12 hex = 14 chars: {}",
            id
        );
    }
}

#[test]
fn test_fj015_is_leap_boundary_years() {
    assert!(!is_leap(1800));
    assert!(!is_leap(1700));
    assert!(is_leap(400));
    assert!(is_leap(800));
    assert!(!is_leap(100));
    assert!(!is_leap(200));
    assert!(!is_leap(300));
}

// -- FJ-132: Additional eventlog edge case tests --

#[test]
fn test_fj132_append_event_idempotent_dir_creation() {
    let dir = tempfile::tempdir().unwrap();
    let event1 = ProvenanceEvent::ApplyStarted {
        machine: "m".to_string(),
        run_id: "r-1".to_string(),
        forjar_version: "0.1.0".to_string(),
                operator: None,
                config_hash: None,
    };
    let event2 = ProvenanceEvent::ApplyCompleted {
        machine: "m".to_string(),
        run_id: "r-1".to_string(),
        resources_converged: 1,
        resources_unchanged: 0,
        resources_failed: 0,
        total_seconds: 1.0,
    };
    append_event(dir.path(), "m", event1).unwrap();
    append_event(dir.path(), "m", event2).unwrap();
    let content = std::fs::read_to_string(dir.path().join("m/events.jsonl")).unwrap();
    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("apply_started"));
    assert!(lines[1].contains("apply_completed"));
}

#[test]
fn test_fj132_event_log_path_with_dots() {
    let p = event_log_path(Path::new("/state"), "machine.with.dots");
    assert_eq!(p, PathBuf::from("/state/machine.with.dots/events.jsonl"));
}

#[test]
fn test_fj132_timestamp_minute_range() {
    let ts = now_iso8601();
    let minute: u32 = ts[14..16].parse().unwrap();
    assert!(minute < 60, "minute should be 0-59: {}", minute);
}

#[test]
fn test_fj132_timestamp_second_range() {
    let ts = now_iso8601();
    let second: u32 = ts[17..19].parse().unwrap();
    assert!(second < 60, "second should be 0-59: {}", second);
}

#[test]
fn test_fj132_event_json_has_ts_field() {
    let dir = tempfile::tempdir().unwrap();
    let event = ProvenanceEvent::ResourceConverged {
        machine: "m".to_string(),
        resource: "r".to_string(),
        duration_seconds: 0.1,
        hash: "blake3:abc".to_string(),
    };
    append_event(dir.path(), "m", event).unwrap();
    let content = std::fs::read_to_string(dir.path().join("m/events.jsonl")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
    assert!(parsed["ts"].is_string());
    let ts = parsed["ts"].as_str().unwrap();
    assert!(ts.ends_with('Z'));
    assert!(ts.contains('T'));
}

#[test]
fn test_fj132_multiple_machines_separate_logs() {
    let dir = tempfile::tempdir().unwrap();
    let event1 = ProvenanceEvent::ApplyStarted {
        machine: "web".to_string(),
        run_id: "r-1".to_string(),
        forjar_version: "0.1.0".to_string(),
                operator: None,
                config_hash: None,
    };
    let event2 = ProvenanceEvent::ApplyStarted {
        machine: "db".to_string(),
        run_id: "r-2".to_string(),
        forjar_version: "0.1.0".to_string(),
                operator: None,
                config_hash: None,
    };
    append_event(dir.path(), "web", event1).unwrap();
    append_event(dir.path(), "db", event2).unwrap();
    let web_log = std::fs::read_to_string(dir.path().join("web/events.jsonl")).unwrap();
    let db_log = std::fs::read_to_string(dir.path().join("db/events.jsonl")).unwrap();
    assert!(web_log.contains("r-1"));
    assert!(!web_log.contains("r-2"));
    assert!(db_log.contains("r-2"));
    assert!(!db_log.contains("r-1"));
}

#[test]
fn test_fj132_event_log_path_construction() {
    let state_dir = Path::new("/tmp/forjar-state");
    let path = event_log_path(state_dir, "web-01");
    assert_eq!(path, PathBuf::from("/tmp/forjar-state/web-01/events.jsonl"));
}

#[test]
fn test_fj132_generate_run_id_format() {
    let id = generate_run_id();
    assert!(id.starts_with("r-"), "should start with 'r-'");
    assert!(id.len() > 2, "should have hex digits after prefix");
    assert!(id[2..].chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_fj132_append_event_creates_directory() {
    let dir = tempfile::tempdir().unwrap();
    let event = ProvenanceEvent::ApplyStarted {
        machine: "new-machine".to_string(),
        run_id: "run-123".to_string(),
        forjar_version: "0.1.0".to_string(),
                operator: None,
                config_hash: None,
    };
    append_event(dir.path(), "new-machine", event).unwrap();
    assert!(dir.path().join("new-machine/events.jsonl").exists());
}

#[test]
fn test_fj132_append_multiple_events_jsonl_format() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        let event = ProvenanceEvent::ResourceConverged {
            machine: "m".to_string(),
            resource: format!("r-{}", i),
            duration_seconds: 0.1,
            hash: "blake3:abc".to_string(),
        };
        append_event(dir.path(), "m", event).unwrap();
    }
    let content = std::fs::read_to_string(dir.path().join("m/events.jsonl")).unwrap();
    let lines: Vec<&str> = content.trim().lines().collect();
    assert_eq!(lines.len(), 3, "should have 3 JSONL lines");
    for line in &lines {
        let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(parsed.is_object());
    }
}

#[test]
fn test_fj132_is_leap_year() {
    assert!(is_leap(2000), "2000 is leap (div by 400)");
    assert!(!is_leap(1900), "1900 is not leap (div by 100 but not 400)");
    assert!(is_leap(2024), "2024 is leap (div by 4)");
    assert!(!is_leap(2023), "2023 is not leap");
}
