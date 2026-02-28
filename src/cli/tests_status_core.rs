//! Tests: Core status command.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::status_core::*;
use super::commands::*;
use super::dispatch::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj017_status_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("state")).unwrap();
        cmd_status(&dir.path().join("state"), None, false, None, false).unwrap();
    }


    #[test]
    fn test_fj017_status_with_lock() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let lock = crate::core::state::new_lock("mybox", "mybox-host");
        crate::core::state::save_lock(&state, &lock).unwrap();

        cmd_status(&state, None, false, None, false).unwrap();
    }


    #[test]
    fn test_fj017_status_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let lock = crate::core::state::new_lock("target", "target-host");
        crate::core::state::save_lock(&state, &lock).unwrap();

        cmd_status(&state, Some("target"), false, None, false).unwrap();
        cmd_status(&state, Some("nonexistent"), false, None, false).unwrap();
    }


    #[test]
    fn test_fj017_dispatch_status() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        dispatch(
            Commands::Status(StatusArgs {
                state_dir: state,
                machine: None,
                json: false,
                file: None,
                summary: false,
                watch: None,
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
                fleet_convergence_velocity: false, resource_failure_correlation: false, machine_resource_churn_rate: false, fleet_resource_staleness: false, machine_convergence_trend: false, machine_capacity_utilization: false, fleet_configuration_entropy: false, machine_resource_freshness: false, machine_error_budget: false, fleet_compliance_score: false, machine_mean_time_to_recovery: false, machine_resource_dependency_health: false, fleet_resource_type_health: false, machine_resource_convergence_rate: false, machine_resource_failure_correlation: false, fleet_resource_age_distribution: false, machine_resource_rollback_readiness: false, machine_resource_health_trend: false, fleet_resource_drift_velocity: false, machine_resource_apply_success_trend: false, machine_resource_mttr_estimate: false, fleet_resource_convergence_forecast: false, machine_resource_error_budget_forecast: false, machine_resource_dependency_lag: false, fleet_resource_dependency_lag: false, machine_resource_config_drift_rate: false, machine_resource_convergence_lag: false, fleet_resource_convergence_lag: false, machine_resource_dependency_depth: false, machine_resource_convergence_velocity: false, fleet_resource_convergence_velocity: false, machine_resource_failure_recurrence: false, machine_resource_drift_frequency: false, fleet_resource_drift_frequency: false, machine_resource_apply_duration_trend: false, machine_resource_convergence_streak: false, fleet_resource_convergence_streak: false, machine_resource_error_distribution: false, machine_resource_drift_age: false, fleet_resource_drift_age: false, machine_resource_recovery_rate: false, machine_resource_drift_velocity: false, fleet_resource_recovery_rate: false, machine_resource_convergence_efficiency: false, machine_resource_apply_frequency: false, fleet_resource_health_score: false, machine_resource_staleness_index: false, machine_resource_drift_recurrence: false, fleet_resource_drift_heatmap: false, machine_resource_convergence_trend_p90: false, machine_resource_drift_age_hours: false, fleet_resource_convergence_percentile: false, machine_resource_error_rate: false, machine_resource_convergence_gap: false, fleet_resource_error_distribution: false, machine_resource_convergence_stability: false, machine_resource_apply_latency_p95: false, fleet_resource_security_posture_score: false, fleet_apply_success_rate_trend: false, machine_resource_drift_flapping: false, fleet_resource_type_drift_heatmap: false, machine_ssh_connection_health: false, lock_file_staleness_report: false, fleet_transport_method_summary: false, fleet_state_churn_analysis: false, config_maturity_score: false, fleet_capacity_utilization: false,
            fleet_drift_velocity_trend: false,
            machine_convergence_window: false,
            fleet_resource_age_histogram: false,
            }),
            false,
            true,
        )
        .unwrap();
    }


    #[test]
    fn test_fj017_status_with_resources_and_duration() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "web-pkg".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::Package,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-02-16T14:00:00Z".to_string()),
                duration_seconds: Some(2.34),
                hash: "blake3:abc".to_string(),
                details: std::collections::HashMap::new(),
            },
        );
        resources.insert(
            "web-svc".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::Service,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-02-16T14:00:01Z".to_string()),
                duration_seconds: None, // no duration — exercises unwrap_or_default branch
                hash: "blake3:def".to_string(),
                details: std::collections::HashMap::new(),
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "webbox".to_string(),
            hostname: "webbox.example.com".to_string(),
            generated_at: "2026-02-16T14:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // Exercises the full resource iteration path with duration display
        cmd_status(&state, None, false, None, false).unwrap();
    }


    #[test]
    fn test_fj017_status_dir_with_non_dir_entry() {
        // Tests the `!entry.path().is_dir()` skip path
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        // Create a regular file inside state/ — should be skipped
        std::fs::write(state.join("not-a-machine"), "junk").unwrap();
        cmd_status(&state, None, false, None, false).unwrap();
    }


    #[test]
    fn test_fj132_cmd_status_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        cmd_status(dir.path(), None, false, None, false).unwrap();
    }


    #[test]
    fn test_fj132_cmd_status_with_global_lock() {
        let dir = tempfile::tempdir().unwrap();
        let lock_yaml = r#"
schema: '1.0'
name: my-infra
last_apply: '2026-02-25T10:00:00Z'
generator: 'forjar 0.1.0'
machines:
  web:
    resources: 5
    converged: 5
    failed: 0
    last_apply: '2026-02-25T10:00:00Z'
"#;
        std::fs::write(dir.path().join("forjar.lock.yaml"), lock_yaml).unwrap();
        cmd_status(dir.path(), None, false, None, false).unwrap();
    }


    #[test]
    fn test_fj017_cmd_status_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_status(&state, None, false, None, false);
        assert!(
            result.is_ok(),
            "cmd_status on empty state dir should succeed"
        );
    }


    #[test]
    fn test_fj205_status_json_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        // Should succeed and produce valid JSON even with no machines
        let result = cmd_status(&state, None, true, None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj205_status_json_with_machine() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let machine_dir = state.join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        let lock = r#"
schema: "1.0"
machine: web
hostname: web-server
generated_at: "2026-02-26T00:00:00Z"
generator: "forjar 0.1.0"
blake3_version: "1.8"
resources: {}
"#;
        std::fs::write(machine_dir.join("forjar-lock.yaml"), lock).unwrap();
        let result = cmd_status(&state, None, true, None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj292_status_file_flag_parse() {
        let cmd = Commands::Status(StatusArgs {
            state_dir: PathBuf::from("state"),
            machine: None,
            json: true,
            file: Some(PathBuf::from("forjar.yaml")),
            summary: false,
            watch: None,
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
                fleet_convergence_velocity: false, resource_failure_correlation: false, machine_resource_churn_rate: false, fleet_resource_staleness: false, machine_convergence_trend: false, machine_capacity_utilization: false, fleet_configuration_entropy: false, machine_resource_freshness: false, machine_error_budget: false, fleet_compliance_score: false, machine_mean_time_to_recovery: false, machine_resource_dependency_health: false, fleet_resource_type_health: false, machine_resource_convergence_rate: false, machine_resource_failure_correlation: false, fleet_resource_age_distribution: false, machine_resource_rollback_readiness: false, machine_resource_health_trend: false, fleet_resource_drift_velocity: false, machine_resource_apply_success_trend: false, machine_resource_mttr_estimate: false, fleet_resource_convergence_forecast: false, machine_resource_error_budget_forecast: false, machine_resource_dependency_lag: false, fleet_resource_dependency_lag: false, machine_resource_config_drift_rate: false, machine_resource_convergence_lag: false, fleet_resource_convergence_lag: false, machine_resource_dependency_depth: false, machine_resource_convergence_velocity: false, fleet_resource_convergence_velocity: false, machine_resource_failure_recurrence: false, machine_resource_drift_frequency: false, fleet_resource_drift_frequency: false, machine_resource_apply_duration_trend: false, machine_resource_convergence_streak: false, fleet_resource_convergence_streak: false, machine_resource_error_distribution: false, machine_resource_drift_age: false, fleet_resource_drift_age: false, machine_resource_recovery_rate: false, machine_resource_drift_velocity: false, fleet_resource_recovery_rate: false, machine_resource_convergence_efficiency: false, machine_resource_apply_frequency: false, fleet_resource_health_score: false, machine_resource_staleness_index: false, machine_resource_drift_recurrence: false, fleet_resource_drift_heatmap: false, machine_resource_convergence_trend_p90: false, machine_resource_drift_age_hours: false, fleet_resource_convergence_percentile: false, machine_resource_error_rate: false, machine_resource_convergence_gap: false, fleet_resource_error_distribution: false, machine_resource_convergence_stability: false, machine_resource_apply_latency_p95: false, fleet_resource_security_posture_score: false, fleet_apply_success_rate_trend: false, machine_resource_drift_flapping: false, fleet_resource_type_drift_heatmap: false, machine_ssh_connection_health: false, lock_file_staleness_report: false, fleet_transport_method_summary: false, fleet_state_churn_analysis: false, config_maturity_score: false, fleet_capacity_utilization: false,
            fleet_drift_velocity_trend: false,
            machine_convergence_window: false,
            fleet_resource_age_histogram: false,
        });
        match cmd {
            Commands::Status(StatusArgs { file, json, .. }) => {
                assert!(json);
                assert_eq!(file, Some(PathBuf::from("forjar.yaml")));
            }
            _ => panic!("expected Status"),
        }
    }


    #[test]
    fn test_fj292_status_json_enriched() {
        use crate::core::state;
        use crate::core::types::{ResourceLock, ResourceStatus, ResourceType, StateLock};
        use indexmap::IndexMap;
        use std::collections::HashMap;

        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create a machine lock
        let mut resources = IndexMap::new();
        resources.insert(
            "web-pkg".to_string(),
            ResourceLock {
                resource_type: ResourceType::Package,
                status: ResourceStatus::Converged,
                applied_at: Some("2026-02-26T00:00:00Z".to_string()),
                duration_seconds: Some(1.5),
                hash: "abc123def456".to_string(),
                details: HashMap::new(),
            },
        );
        let lock = StateLock {
            schema: "1".to_string(),
            machine: "local".to_string(),
            hostname: "local".to_string(),
            generated_at: "2026-02-26T00:00:00Z".to_string(),
            generator: "forjar-test".to_string(),
            blake3_version: "1.5.0".to_string(),
            resources,
        };

        // Save machine lock
        let machine_dir = state_dir.join("local");
        std::fs::create_dir_all(&machine_dir).unwrap();
        state::save_lock(&state_dir, &lock).unwrap();

        // Create a config file with resource_group and tags
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  web-pkg:
    type: package
    machine: local
    provider: apt
    packages: [curl]
    resource_group: web
    tags: [critical, frontend]
"#,
        )
        .unwrap();

        // Call status with config file — should enrich JSON
        let result = cmd_status(&state_dir, None, true, Some(config_path.as_path()), false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj292_status_no_file_still_works() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_status(&state_dir, None, false, None, false);
        assert!(result.is_ok());
    }

    // ── FJ-293: apply --dry-run --json ──

}
