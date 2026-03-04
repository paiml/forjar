//! Tests: FJ-1452 drift prediction.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::drift_predict::*;
use super::helpers::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_events(dir: &Path, filename: &str, lines: &[&str]) {
        let file = dir.join(filename);
        std::fs::write(&file, lines.join("\n")).unwrap();
    }

    #[test]
    fn test_drift_predict_no_events() {
        let dir = tempfile::tempdir().unwrap();
        cmd_drift_predict(dir.path(), None, 0, false).unwrap();
    }

    #[test]
    fn test_drift_predict_no_drift() {
        let dir = tempfile::tempdir().unwrap();
        write_events(
            dir.path(),
            "events.jsonl",
            &[
                r#"{"resource":"pkg","action":"apply","timestamp":1000.0}"#,
                r#"{"resource":"pkg","action":"apply","timestamp":2000.0}"#,
            ],
        );
        cmd_drift_predict(dir.path(), None, 0, false).unwrap();
    }

    #[test]
    fn test_drift_predict_frequent_drift() {
        let dir = tempfile::tempdir().unwrap();
        write_events(
            dir.path(),
            "events.jsonl",
            &[
                r#"{"resource":"conf","action":"drift","timestamp":1000.0}"#,
                r#"{"resource":"conf","action":"drift","timestamp":2000.0}"#,
                r#"{"resource":"conf","action":"drift","timestamp":3000.0}"#,
                r#"{"resource":"conf","action":"apply","timestamp":4000.0}"#,
            ],
        );
        cmd_drift_predict(dir.path(), None, 0, false).unwrap();
    }

    #[test]
    fn test_drift_predict_trend() {
        let dir = tempfile::tempdir().unwrap();
        write_events(
            dir.path(),
            "events.jsonl",
            &[
                r#"{"resource":"svc","action":"drift","timestamp":1000.0}"#,
                r#"{"resource":"svc","action":"apply","timestamp":2000.0}"#,
                r#"{"resource":"svc","action":"drift","timestamp":2500.0}"#,
                r#"{"resource":"svc","action":"drift","timestamp":2800.0}"#,
                r#"{"resource":"svc","action":"drift","timestamp":2900.0}"#,
                r#"{"resource":"svc","action":"drift","timestamp":2950.0}"#,
            ],
        );
        cmd_drift_predict(dir.path(), None, 0, false).unwrap();
    }

    #[test]
    fn test_drift_predict_multi_machine() {
        let dir = tempfile::tempdir().unwrap();
        write_events(
            dir.path(),
            "events.jsonl",
            &[
                r#"{"resource":"pkg","machine":"web","action":"drift","timestamp":1000.0}"#,
                r#"{"resource":"pkg","machine":"db","action":"apply","timestamp":1500.0}"#,
                r#"{"resource":"pkg","machine":"web","action":"drift","timestamp":2000.0}"#,
            ],
        );
        cmd_drift_predict(dir.path(), Some("web"), 0, false).unwrap();
    }

    #[test]
    fn test_drift_predict_json() {
        let dir = tempfile::tempdir().unwrap();
        write_events(
            dir.path(),
            "events.jsonl",
            &[
                r#"{"resource":"conf","action":"drift","timestamp":1000.0}"#,
                r#"{"resource":"conf","action":"apply","timestamp":2000.0}"#,
            ],
        );
        cmd_drift_predict(dir.path(), None, 0, true).unwrap();
    }

    #[test]
    fn test_drift_predict_recursive_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("intel");
        std::fs::create_dir_all(&sub).unwrap();
        write_events(
            &sub,
            "events.jsonl",
            &[
                r#"{"resource":"pkg","action":"drift","timestamp":1000.0}"#,
                r#"{"resource":"pkg","action":"apply","timestamp":2000.0}"#,
            ],
        );
        // Events in subdirectory should be found
        cmd_drift_predict(dir.path(), None, 0, false).unwrap();
    }

    #[test]
    fn test_drift_predict_iso_timestamps() {
        let dir = tempfile::tempdir().unwrap();
        write_events(
            dir.path(),
            "events.jsonl",
            &[
                r#"{"resource":"conf","action":"drift","ts":"2026-02-16T16:32:54Z"}"#,
                r#"{"resource":"conf","action":"apply","ts":"2026-02-16T17:00:00Z"}"#,
                r#"{"resource":"conf","action":"drift","ts":"2026-02-17T10:00:00Z"}"#,
            ],
        );
        cmd_drift_predict(dir.path(), None, 0, false).unwrap();
    }

    #[test]
    fn test_drift_predict_event_field() {
        let dir = tempfile::tempdir().unwrap();
        write_events(
            dir.path(),
            "events.jsonl",
            &[
                r#"{"resource":"svc","event":"resource_drifted","machine":"web","ts":"2026-01-01T00:00:00Z"}"#,
                r#"{"resource":"svc","event":"resource_converged","machine":"web","ts":"2026-01-01T01:00:00Z"}"#,
                r#"{"resource":"svc","event":"resource_drifted","machine":"web","ts":"2026-01-02T00:00:00Z"}"#,
            ],
        );
        // Should recognize "event" field and "resource_drifted" action
        cmd_drift_predict(dir.path(), None, 0, false).unwrap();
    }

    #[test]
    fn test_drift_predict_real_forjar_format() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("intel");
        std::fs::create_dir_all(&sub).unwrap();
        write_events(
            &sub,
            "events.jsonl",
            &[
                r#"{"ts":"2026-02-16T16:32:54Z","event":"apply_started","machine":"intel","run_id":"r-abc123"}"#,
                r#"{"ts":"2026-02-16T16:32:54Z","event":"resource_started","machine":"intel","resource":"bash-aliases","action":"CREATE"}"#,
                r#"{"ts":"2026-02-16T16:32:55Z","event":"resource_converged","machine":"intel","resource":"bash-aliases","duration_seconds":0.54}"#,
                r#"{"ts":"2026-02-16T16:33:00Z","event":"resource_drifted","machine":"intel","resource":"bash-aliases"}"#,
            ],
        );
        // Should parse subdirectory, ISO timestamps, "event" field, and detect drift
        cmd_drift_predict(dir.path(), None, 0, false).unwrap();
    }

    #[test]
    fn test_drift_predict_limit() {
        let dir = tempfile::tempdir().unwrap();
        write_events(
            dir.path(),
            "events.jsonl",
            &[
                r#"{"resource":"a","action":"drift","timestamp":1000.0}"#,
                r#"{"resource":"b","action":"drift","timestamp":1000.0}"#,
                r#"{"resource":"c","action":"drift","timestamp":1000.0}"#,
            ],
        );
        cmd_drift_predict(dir.path(), None, 1, false).unwrap();
    }
}
