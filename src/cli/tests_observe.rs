//! Tests: Observability.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::observe::*;
use super::commands::*;
use super::dispatch::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj131_cmd_anomaly_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        // No machine directories → should succeed with no output
        cmd_anomaly(dir.path(), None, 1, false).unwrap();
    }


    #[test]
    fn test_fj131_cmd_anomaly_no_events() {
        let dir = tempfile::tempdir().unwrap();
        // Create machine dir but no events.jsonl
        std::fs::create_dir_all(dir.path().join("web")).unwrap();
        cmd_anomaly(dir.path(), None, 1, false).unwrap();
    }


    #[test]
    fn test_fj131_cmd_anomaly_with_events() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();

        // Write some events
        let events = [
            r#"{"ts":"2026-02-25T00:00:00Z","event":"resource_converged","machine":"web","resource":"pkg","duration_seconds":1.0,"hash":"blake3:abc"}"#,
            r#"{"ts":"2026-02-25T01:00:00Z","event":"resource_converged","machine":"web","resource":"pkg","duration_seconds":1.0,"hash":"blake3:abc"}"#,
            r#"{"ts":"2026-02-25T02:00:00Z","event":"resource_converged","machine":"web","resource":"pkg","duration_seconds":1.0,"hash":"blake3:abc"}"#,
        ];
        std::fs::write(machine_dir.join("events.jsonl"), events.join("\n")).unwrap();

        cmd_anomaly(dir.path(), None, 1, false).unwrap();
    }


    #[test]
    fn test_fj131_cmd_anomaly_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        // Create two machine dirs
        for name in ["web", "db"] {
            let machine_dir = dir.path().join(name);
            std::fs::create_dir_all(&machine_dir).unwrap();
            let event = format!(
                r#"{{"ts":"2026-02-25T00:00:00Z","event":"resource_converged","machine":"{}","resource":"pkg","duration_seconds":1.0,"hash":"blake3:abc"}}"#,
                name
            );
            std::fs::write(machine_dir.join("events.jsonl"), event).unwrap();
        }

        // Filter to only "web"
        cmd_anomaly(dir.path(), Some("web"), 1, false).unwrap();
    }


    #[test]
    fn test_fj131_cmd_anomaly_nonexistent_state_dir() {
        let err = cmd_anomaly(
            std::path::Path::new("/tmp/nonexistent-forjar-state"),
            None,
            1,
            false,
        );
        assert!(err.is_err());
    }


    #[test]
    fn test_fj135_cmd_trace_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_trace(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj135_cmd_trace_empty_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_trace(dir.path(), None, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj135_cmd_trace_with_data() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = crate::tripwire::tracer::TraceSession::start("r-test-trace");
        session.record_noop("r1", "file", "m1");
        session.record_span(
            "r2",
            "package",
            "m1",
            "create",
            std::time::Duration::from_millis(100),
            0,
            None,
        );
        crate::tripwire::tracer::write_trace(dir.path(), "m1", &session).unwrap();

        let result = cmd_trace(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj135_cmd_trace_json_with_data() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = crate::tripwire::tracer::TraceSession::start("r-test-json");
        session.record_noop("r1", "file", "m1");
        crate::tripwire::tracer::write_trace(dir.path(), "m1", &session).unwrap();

        let result = cmd_trace(dir.path(), None, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj135_cmd_trace_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = crate::tripwire::tracer::TraceSession::start("r-filter");
        session.record_noop("r1", "file", "web");
        crate::tripwire::tracer::write_trace(dir.path(), "web", &session).unwrap();

        // Filter to nonexistent machine — should find nothing
        let result = cmd_trace(dir.path(), Some("nonexistent"), false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj135_cmd_trace_nonexistent_dir() {
        let result = cmd_trace(Path::new("/tmp/forjar-nonexistent-12345"), None, false);
        assert!(result.is_err());
    }


    #[test]
    fn test_fj267_watch_requires_yes_with_apply() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test:\n    type: file\n    machine: local\n    path: /tmp/fj267-test.txt\n    content: test\n").unwrap();
        let result = dispatch(
            Commands::Watch(WatchArgs {
                file: config_path,
                state_dir: dir.path().join("state"),
                interval: 2,
                apply: true,
                yes: false,
            }),
            false,
            true,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--yes"));
    }


    #[test]
    fn test_fj267_watch_command_parse() {
        // Verify Watch command variant has correct fields
        let cmd = Commands::Watch(WatchArgs {
            file: PathBuf::from("forjar.yaml"),
            state_dir: PathBuf::from("state"),
            interval: 5,
            apply: false,
            yes: false,
        });
        match cmd {
            Commands::Watch(WatchArgs {
                interval, apply, ..
            }) => {
                assert_eq!(interval, 5);
                assert!(!apply);
            }
            _ => panic!("expected Watch"),
        }
    }


    #[test]
    fn test_fj314_watch_flag_parse() {
        let cmd = Commands::Status(StatusArgs {
            state_dir: PathBuf::from("state"),
            machine: None,
            json: false,
            file: None,
            summary: false,
            watch: Some(5),
            stale: None,
            health: false,
            drift_details: false,
            timeline: false,
            changes_since: None,
            summary_by: None,
            prometheus: false,
            expired: None,
            count: false,
            format: None,
            anomalies: false,
            diff_from: None,
            resources_by_type: false,
            machines_only: false,
            stale_resources: false,
            health_threshold: None,
            json_lines: false,
            since: None,
            export: None,
            compact: false,
            alerts: false,
            diff_lock: None,
            compliance: None,
            histogram: false,
            dependency_health: false,
            top_failures: false,
            convergence_rate: false,
            drift_summary: false,
            resource_age: false,
            sla_report: false,
            compliance_report: None,
            mttr: false,
            trend: None,
            prediction: false,
            capacity: false,
            cost_estimate: false,
            staleness_report: None,
            health_score: false,
            executive_summary: false,
            audit_trail: false,
            resource_graph: false,
            drift_velocity: false,
            fleet_overview: false,
            machine_health: false,
            config_drift: false,
            convergence_time: false,
            resource_timeline: false,
            error_summary: false,
            security_posture: false,
            resource_cost: false,
            drift_forecast: false,
            pipeline_status: false,
            resource_dependencies: false,
            diagnostic: false,
            uptime: false,
            recommendations: false,
            machine_summary: false,
            change_frequency: false,
            lock_age: false,
            failed_since: None,
            hash_verify: false,
            resource_size: false,
            drift_details_all: false,
            last_apply_duration: false,
            config_hash: false,
            convergence_history: false,
            resource_inputs: false,
            drift_trend: false,
            failed_resources: false,
            resource_types_summary: false,
            resource_health: false,
            machine_health_summary: false,
            dependency_count: false,
            last_apply_status: false,
            resource_staleness: false,
            convergence_percentage: false,
            failed_count: false,
            drift_count: false,
            resource_duration: false,
            machine_resource_map: false,
            fleet_convergence: false,
            resource_hash: false,
            machine_drift_summary: false,
            apply_history_count: false,
            lock_file_count: false,
            resource_type_distribution: false,
            resource_apply_age: false,
            machine_uptime: false,
            resource_churn: false,
            last_drift_time: false,
            machine_resource_count: false,
            convergence_score: false,
            apply_success_rate: false,
            error_rate: false,
            fleet_health_summary: false, machine_convergence_history: false, drift_history: false, resource_failure_rate: false, machine_last_apply: false, fleet_drift_summary: false, resource_apply_duration: false, machine_resource_health: false, fleet_convergence_trend: false, resource_state_distribution: false, machine_apply_count: false, fleet_apply_history: false, resource_hash_changes: false, machine_uptime_estimate: false, fleet_resource_type_breakdown: false, resource_convergence_time: false,
                machine_drift_age: false,
                fleet_failed_resources: false,
                resource_dependency_health: false,
                machine_resource_age_distribution: false,
                fleet_convergence_velocity: false, resource_failure_correlation: false,
                machine_resource_churn_rate: false, fleet_resource_staleness: false, machine_convergence_trend: false, machine_capacity_utilization: false, fleet_configuration_entropy: false, machine_resource_freshness: false, machine_error_budget: false, fleet_compliance_score: false, machine_mean_time_to_recovery: false, machine_resource_dependency_health: false, fleet_resource_type_health: false, machine_resource_convergence_rate: false, machine_resource_failure_correlation: false, fleet_resource_age_distribution: false, machine_resource_rollback_readiness: false, machine_resource_health_trend: false, fleet_resource_drift_velocity: false, machine_resource_apply_success_trend: false, machine_resource_mttr_estimate: false, fleet_resource_convergence_forecast: false, machine_resource_error_budget_forecast: false, machine_resource_dependency_lag: false, fleet_resource_dependency_lag: false, machine_resource_config_drift_rate: false, machine_resource_convergence_lag: false, fleet_resource_convergence_lag: false, machine_resource_dependency_depth: false, machine_resource_convergence_velocity: false, fleet_resource_convergence_velocity: false, machine_resource_failure_recurrence: false, machine_resource_drift_frequency: false, fleet_resource_drift_frequency: false, machine_resource_apply_duration_trend: false, machine_resource_convergence_streak: false, fleet_resource_convergence_streak: false, machine_resource_error_distribution: false, machine_resource_drift_age: false, fleet_resource_drift_age: false, machine_resource_recovery_rate: false, machine_resource_drift_velocity: false, fleet_resource_recovery_rate: false, machine_resource_convergence_efficiency: false, machine_resource_apply_frequency: false, fleet_resource_health_score: false, machine_resource_staleness_index: false, machine_resource_drift_recurrence: false, fleet_resource_drift_heatmap: false, machine_resource_convergence_trend_p90: false, machine_resource_drift_age_hours: false, fleet_resource_convergence_percentile: false, machine_resource_error_rate: false, machine_resource_convergence_gap: false, fleet_resource_error_distribution: false, machine_resource_convergence_stability: false, machine_resource_apply_latency_p95: false, fleet_resource_security_posture_score: false, fleet_apply_success_rate_trend: false, machine_resource_drift_flapping: false, fleet_resource_type_drift_heatmap: false, machine_ssh_connection_health: false, lock_file_staleness_report: false, fleet_transport_method_summary: false, fleet_state_churn_analysis: false, config_maturity_score: false, fleet_capacity_utilization: false,
            fleet_drift_velocity_trend: false,
            machine_convergence_window: false,
            fleet_resource_age_histogram: false,
            fleet_security_posture_summary: false,
            machine_resource_freshness_index: false,
            fleet_resource_type_coverage: false, fleet_apply_cadence: false, machine_resource_error_classification: false, fleet_resource_convergence_summary: false, fleet_resource_staleness_report: false, machine_resource_type_distribution: false, fleet_machine_health_score: false, fleet_resource_dependency_lag_report: false, machine_resource_convergence_rate_trend: false, fleet_resource_apply_lag: false, fleet_resource_error_rate_trend: false, machine_resource_drift_recovery_time: false, fleet_resource_config_complexity_score: false, fleet_resource_maturity_index: false, machine_resource_convergence_stability_index: false, fleet_resource_drift_pattern_analysis: false, fleet_resource_apply_success_trend: false, machine_resource_drift_age_distribution_report: false, fleet_resource_convergence_gap_analysis: false, fleet_resource_type_drift_correlation: false, machine_resource_apply_cadence_report: false, fleet_resource_drift_recovery_trend: false,
        });
        match cmd {
            Commands::Status(StatusArgs { watch, .. }) => assert_eq!(watch, Some(5)),
            _ => panic!("expected Status"),
        }
    }

    // ── FJ-317: apply --notify webhook ──

}
