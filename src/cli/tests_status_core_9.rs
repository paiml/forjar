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
use super::status_alerts::*;
use super::status_compliance::*;
use super::status_convergence::*;
use super::status_drift::*;
use super::status_failures::*;
use super::status_fleet::*;
use super::status_observability::*;
use super::status_resource_detail::*;
use super::status_resources::*;
use super::status_trends::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj582_status_config_drift_flag() {
        let cmd = Commands::Status(StatusArgs {
            state_dir: PathBuf::from("state"),
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
            config_drift: true,
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
        });
        match cmd {
            Commands::Status(StatusArgs { config_drift, .. }) => assert!(config_drift),
            _ => panic!("expected Status"),
        }
    }


    #[test]
    fn test_fj587_status_convergence_time_flag() {
        let cmd = Commands::Status(StatusArgs {
            state_dir: PathBuf::from("state"),
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
            convergence_time: true,
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
        });
        match cmd {
            Commands::Status(StatusArgs {
                convergence_time, ..
            }) => assert!(convergence_time),
            _ => panic!("expected Status"),
        }
    }

    // ── Phase 45 Tests: FJ-590→FJ-597 Advanced Orchestration ──


    #[test]
    fn test_fj593_status_resource_timeline() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_resource_timeline(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj594_status_error_summary() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_error_summary(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj602_status_security_posture() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_security_posture(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj602_status_security_posture_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_security_posture(dir.path(), None, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj612_status_resource_cost() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_resource_cost(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj617_status_drift_forecast() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_drift_forecast(dir.path(), None, false);
        assert!(result.is_ok());
    }

}
