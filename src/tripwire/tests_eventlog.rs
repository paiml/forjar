use super::eventlog::*;
use crate::core::types::ProvenanceEvent;
use std::path::{Path, PathBuf};

#[test]
fn test_fj015_now_iso8601() {
    let ts = now_iso8601();
    assert!(ts.starts_with("20"));
    assert!(ts.ends_with('Z'));
    assert!(ts.contains('T'));
}

#[test]
fn test_fj015_generate_run_id() {
    let id = generate_run_id();
    assert!(id.starts_with("r-"));
    assert!(id.len() > 4);
}

#[test]
fn test_fj015_event_log_path() {
    let p = event_log_path(Path::new("/state"), "lambda");
    assert_eq!(p, PathBuf::from("/state/lambda/events.jsonl"));
}

#[test]
fn test_fj015_append_event() {
    let dir = tempfile::tempdir().unwrap();
    let event = ProvenanceEvent::ApplyStarted {
        machine: "test".to_string(),
        run_id: "r-abc".to_string(),
        forjar_version: "0.1.0".to_string(),
    };
    append_event(dir.path(), "test", event).unwrap();

    let content = std::fs::read_to_string(dir.path().join("test/events.jsonl")).unwrap();
    assert!(content.contains("apply_started"));
    assert!(content.contains("r-abc"));
}

#[test]
fn test_fj015_append_multiple() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        let event = ProvenanceEvent::ResourceConverged {
            machine: "m".to_string(),
            resource: format!("r{}", i),
            duration_seconds: 1.0,
            hash: "blake3:xxx".to_string(),
        };
        append_event(dir.path(), "m", event).unwrap();
    }
    let content = std::fs::read_to_string(dir.path().join("m/events.jsonl")).unwrap();
    let lines: Vec<_> = content.lines().collect();
    assert_eq!(lines.len(), 3);
}

#[test]
fn test_fj015_append_resource_failed_event() {
    let dir = tempfile::tempdir().unwrap();
    let event = ProvenanceEvent::ResourceFailed {
        machine: "test".to_string(),
        resource: "bad-pkg".to_string(),
        error: "apt failed".to_string(),
    };
    append_event(dir.path(), "test", event).unwrap();

    let content = std::fs::read_to_string(dir.path().join("test/events.jsonl")).unwrap();
    assert!(content.contains("resource_failed"));
    assert!(content.contains("bad-pkg"));
    assert!(content.contains("apt failed"));
}

#[test]
fn test_fj015_append_apply_completed_event() {
    let dir = tempfile::tempdir().unwrap();
    let event = ProvenanceEvent::ApplyCompleted {
        machine: "m1".to_string(),
        run_id: "r-xyz".to_string(),
        resources_converged: 4,
        resources_unchanged: 0,
        resources_failed: 1,
        total_seconds: 12.5,
    };
    append_event(dir.path(), "m1", event).unwrap();

    let content = std::fs::read_to_string(dir.path().join("m1/events.jsonl")).unwrap();
    assert!(content.contains("apply_completed"));
    assert!(content.contains("r-xyz"));
}

#[test]
fn test_fj015_run_id_uniqueness() {
    // Two consecutive IDs should be different
    let id1 = generate_run_id();
    let id2 = generate_run_id();
    assert_ne!(id1, id2, "consecutive run IDs must be unique");
}

#[test]
fn test_fj015_creates_machine_dir() {
    let dir = tempfile::tempdir().unwrap();
    let event = ProvenanceEvent::ApplyStarted {
        machine: "new-machine".to_string(),
        run_id: "r-test".to_string(),
        forjar_version: "0.1.0".to_string(),
    };
    // Machine directory doesn't exist yet
    assert!(!dir.path().join("new-machine").exists());
    append_event(dir.path(), "new-machine", event).unwrap();
    assert!(dir.path().join("new-machine").exists());
}

#[test]
fn test_fj015_events_are_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    let events = vec![
        ProvenanceEvent::ApplyStarted {
            machine: "m".to_string(),
            run_id: "r-1".to_string(),
            forjar_version: "0.1.0".to_string(),
        },
        ProvenanceEvent::ResourceConverged {
            machine: "m".to_string(),
            resource: "r".to_string(),
            duration_seconds: 0.5,
            hash: "blake3:abc".to_string(),
        },
        ProvenanceEvent::ApplyCompleted {
            machine: "m".to_string(),
            run_id: "r-1".to_string(),
            resources_converged: 1,
            resources_unchanged: 0,
            resources_failed: 0,
            total_seconds: 0.5,
        },
    ];
    for event in events {
        append_event(dir.path(), "m", event).unwrap();
    }
    let content = std::fs::read_to_string(dir.path().join("m/events.jsonl")).unwrap();
    for line in content.lines() {
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("invalid JSON: {} in line: {}", e, line));
        assert!(parsed["ts"].is_string(), "every event must have ts field");
        assert!(
            parsed["event"].is_string(),
            "every event must have event field"
        );
    }
}

#[test]
fn test_fj015_timestamp_format() {
    let ts = now_iso8601();
    // Format: YYYY-MM-DDTHH:MM:SSZ
    assert_eq!(ts.len(), 20, "ISO 8601 timestamp should be 20 chars");
    assert_eq!(&ts[4..5], "-");
    assert_eq!(&ts[7..8], "-");
    assert_eq!(&ts[10..11], "T");
    assert_eq!(&ts[13..14], ":");
    assert_eq!(&ts[16..17], ":");
    assert_eq!(&ts[19..20], "Z");
}

#[test]
fn test_fj015_append_drift_detected_event() {
    let dir = tempfile::tempdir().unwrap();
    let event = ProvenanceEvent::DriftDetected {
        machine: "web1".to_string(),
        resource: "config-file".to_string(),
        expected_hash: "blake3:aaa".to_string(),
        actual_hash: "blake3:bbb".to_string(),
    };
    append_event(dir.path(), "web1", event).unwrap();

    let content = std::fs::read_to_string(dir.path().join("web1/events.jsonl")).unwrap();
    assert!(content.contains("drift_detected"));
    assert!(content.contains("blake3:aaa"));
    assert!(content.contains("blake3:bbb"));
}

#[test]
fn test_fj015_append_creates_nested_dirs() {
    let dir = tempfile::tempdir().unwrap();
    // State dir structure: base/machine/events.jsonl
    // When machine dir doesn't exist, create_dir_all creates it
    let state_dir = dir.path().join("deep").join("nested").join("state");
    let event = ProvenanceEvent::ApplyStarted {
        machine: "m".to_string(),
        run_id: "r-1".to_string(),
        forjar_version: "0.1.0".to_string(),
    };
    append_event(&state_dir, "m", event).unwrap();
    assert!(state_dir.join("m/events.jsonl").exists());
}

#[test]
fn test_fj015_run_id_hex_format() {
    let id = generate_run_id();
    // Format: r-XXXXXXXXXXXX (r- + 12 hex chars)
    assert!(id.starts_with("r-"));
    let hex_part = &id[2..];
    assert_eq!(hex_part.len(), 12);
    assert!(
        hex_part.chars().all(|c| c.is_ascii_hexdigit()),
        "run ID hex part must be valid hex: {}",
        hex_part
    );
}

#[test]
fn test_fj015_event_log_path_special_chars() {
    let p = event_log_path(Path::new("/var/lib/forjar/state"), "web-server-01");
    assert_eq!(
        p,
        PathBuf::from("/var/lib/forjar/state/web-server-01/events.jsonl")
    );
}

#[test]
fn test_fj015_is_leap() {
    // BH-MUT-0001: Each assertion kills a specific mutation of
    // `(y % 4 == 0 && y % 100 != 0) || y % 400 == 0`

    // Divisible by 400 → leap (kills: remove `|| y % 400 == 0`)
    assert!(is_leap(2000));
    assert!(is_leap(1600));

    // Divisible by 100 but NOT 400 → NOT leap (kills: flip `y % 100 != 0`)
    assert!(!is_leap(1900));
    assert!(!is_leap(2100));

    // Divisible by 4 but NOT 100 → leap (kills: flip `y % 4 == 0`, flip `&&` to `||`)
    assert!(is_leap(2024));
    assert!(is_leap(2028));
    assert!(is_leap(1996));

    // NOT divisible by 4 → NOT leap (kills: negate entire expression)
    assert!(!is_leap(2023));
    assert!(!is_leap(2025));
    assert!(!is_leap(2026));
}

// ── FJ-128: Eventlog edge case tests ────────────────────────

#[test]
fn test_fj015_timestamp_year_plausible() {
    let ts = now_iso8601();
    let year: i64 = ts[0..4].parse().unwrap();
    assert!(
        (2025..=2100).contains(&year),
        "year should be plausible: {}",
        year
    );
}

#[test]
fn test_fj015_timestamp_month_range() {
    let ts = now_iso8601();
    let month: u32 = ts[5..7].parse().unwrap();
    assert!((1..=12).contains(&month), "month should be 1-12: {}", month);
}
