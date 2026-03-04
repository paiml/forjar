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
