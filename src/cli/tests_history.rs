//! Tests: History commands.

#![allow(unused_imports)]
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::history::*;
use super::commands::*;
use super::dispatch::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj017_history_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        cmd_history(&state, None, 10, false, None).unwrap();
    }


    #[test]
    fn test_fj017_history_with_events() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        // Write some events
        crate::tripwire::eventlog::append_event(
            &state,
            "m1",
            crate::core::types::ProvenanceEvent::ApplyStarted {
                machine: "m1".to_string(),
                run_id: "r-001".to_string(),
                forjar_version: "0.1.0".to_string(),
            },
        )
        .unwrap();
        crate::tripwire::eventlog::append_event(
            &state,
            "m1",
            crate::core::types::ProvenanceEvent::ApplyCompleted {
                machine: "m1".to_string(),
                run_id: "r-001".to_string(),
                resources_converged: 3,
                resources_unchanged: 0,
                resources_failed: 0,
                total_seconds: 5.2,
            },
        )
        .unwrap();

        cmd_history(&state, None, 10, false, None).unwrap();
    }


    #[test]
    fn test_fj017_history_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        crate::tripwire::eventlog::append_event(
            &state,
            "m1",
            crate::core::types::ProvenanceEvent::ApplyStarted {
                machine: "m1".to_string(),
                run_id: "r-002".to_string(),
                forjar_version: "0.1.0".to_string(),
            },
        )
        .unwrap();

        cmd_history(&state, None, 10, true, None).unwrap();
    }


    #[test]
    fn test_fj017_history_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        crate::tripwire::eventlog::append_event(
            &state,
            "alpha",
            crate::core::types::ProvenanceEvent::ApplyStarted {
                machine: "alpha".to_string(),
                run_id: "r-a".to_string(),
                forjar_version: "0.1.0".to_string(),
            },
        )
        .unwrap();
        crate::tripwire::eventlog::append_event(
            &state,
            "beta",
            crate::core::types::ProvenanceEvent::ApplyStarted {
                machine: "beta".to_string(),
                run_id: "r-b".to_string(),
                forjar_version: "0.1.0".to_string(),
            },
        )
        .unwrap();

        // Only show alpha
        cmd_history(&state, Some("alpha"), 10, false, None).unwrap();
    }


    #[test]
    fn test_fj017_history_limit() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        for i in 0..5 {
            crate::tripwire::eventlog::append_event(
                &state,
                "m1",
                crate::core::types::ProvenanceEvent::ApplyStarted {
                    machine: "m1".to_string(),
                    run_id: format!("r-{}", i),
                    forjar_version: "0.1.0".to_string(),
                },
            )
            .unwrap();
        }

        // Limit to 2
        cmd_history(&state, None, 2, false, None).unwrap();
    }


    #[test]
    fn test_fj017_dispatch_history() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        dispatch(
            Commands::History(HistoryArgs {
                state_dir: state,
                machine: None,
                limit: 10,
                json: false,
                since: None,
                resource: None,
            }),
            false,
            true,
        )
        .unwrap();
    }


    #[test]
    fn test_fj132_cmd_history_with_events() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        let events = [
            r#"{"ts":"2026-02-25T10:00:00Z","event":"apply_started","machine":"web","run_id":"r-1","forjar_version":"0.1.0"}"#,
            r#"{"ts":"2026-02-25T10:01:00Z","event":"resource_converged","machine":"web","resource":"pkg","duration_seconds":5.0,"hash":"blake3:abc"}"#,
            r#"{"ts":"2026-02-25T10:02:00Z","event":"apply_completed","machine":"web","run_id":"r-1","resources_converged":1,"resources_failed":0,"resources_skipped":0,"total_duration":5.0}"#,
        ];
        std::fs::write(machine_dir.join("events.jsonl"), events.join("\n")).unwrap();
        cmd_history(dir.path(), None, 10, false, None).unwrap();
    }


    #[test]
    fn test_fj132_cmd_history_json() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("db");
        std::fs::create_dir_all(&machine_dir).unwrap();
        let event = r#"{"ts":"2026-02-25T10:00:00Z","event":"apply_started","machine":"db","run_id":"r-1","forjar_version":"0.1.0"}"#;
        std::fs::write(machine_dir.join("events.jsonl"), event).unwrap();
        cmd_history(dir.path(), None, 5, true, None).unwrap();
    }


    #[test]
    fn test_fj132_cmd_history_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["web", "db"] {
            let m_dir = dir.path().join(name);
            std::fs::create_dir_all(&m_dir).unwrap();
            let event = format!(
                r#"{{"ts":"2026-02-25T10:00:00Z","event":"apply_started","machine":"{}","run_id":"r-1","forjar_version":"0.1.0"}}"#,
                name
            );
            std::fs::write(m_dir.join("events.jsonl"), event).unwrap();
        }
        cmd_history(dir.path(), Some("web"), 10, false, None).unwrap();
    }


    #[test]
    fn test_fj284_history_since_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        // --since with no events should succeed (empty result)
        cmd_history(dir.path(), None, 10, false, Some("1h")).unwrap();
    }

    // ── FJ-285: forjar plan --target ──────────────────────────


    #[test]
    fn test_fj296_history_json_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_history(&state_dir, None, 10, true, None);
        assert!(result.is_ok());
    }

    // ── FJ-297: plan --output-dir metadata headers ──


    #[test]
    fn test_fj357_history_resource_flag() {
        let cmd = Commands::History(HistoryArgs {
            state_dir: PathBuf::from("state"),
            machine: None,
            limit: 10,
            json: false,
            since: None,
            resource: Some("base-packages".to_string()),
        });
        match cmd {
            Commands::History(HistoryArgs { resource, .. }) => {
                assert_eq!(resource, Some("base-packages".to_string()));
            }
            _ => panic!("expected History"),
        }
    }

    // ── Phase 22: Infrastructure Intelligence (FJ-360→FJ-367) ──

}
