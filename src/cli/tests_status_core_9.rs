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
        let cmd = Commands::Status {
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
        };
        match cmd {
            Commands::Status { config_drift, .. } => assert!(config_drift),
            _ => panic!("expected Status"),
        }
    }


    #[test]
    fn test_fj587_status_convergence_time_flag() {
        let cmd = Commands::Status {
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
        };
        match cmd {
            Commands::Status {
                convergence_time, ..
            } => assert!(convergence_time),
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

    // ── Phase 48 Tests: FJ-620→FJ-627 Workflow Automation & Pipelines ──


    #[test]
    fn test_fj622_status_pipeline_status() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_pipeline_status(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj627_status_resource_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_resource_dependencies(dir.path(), None, false);
        assert!(result.is_ok());
    }

    // ── Phase 49 Tests: FJ-630→FJ-637 Advanced Diagnostics & Debugging ──


    #[test]
    fn test_fj632_status_diagnostic() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_diagnostic(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj632_status_diagnostic_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_diagnostic(dir.path(), None, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj642_status_uptime() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_uptime(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj642_status_uptime_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_uptime(dir.path(), None, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj647_status_recommendations() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_recommendations(dir.path(), None, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj652_status_convergence_rate() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_convergence_rate(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj652_status_convergence_rate_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_convergence_rate(dir.path(), None, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj657_status_machine_summary() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_machine_summary(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj657_status_machine_summary_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_machine_summary(dir.path(), None, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj662_status_change_frequency() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_change_frequency(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj662_status_change_frequency_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_change_frequency(dir.path(), None, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj667_status_lock_age() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_lock_age(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj667_status_lock_age_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_lock_age(dir.path(), None, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj672_status_failed_since() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_failed_since(dir.path(), None, "2024-01-01", false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj677_status_hash_verify() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_hash_verify(dir.path(), None, true);
        assert!(result.is_ok());
    }

    // ── Phase 54 tests (FJ-680 → FJ-687) ──────────────────────────


    #[test]
    fn test_fj682_status_resource_size() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_resource_size(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj687_status_drift_details_all() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_drift_details_all(dir.path(), false);
        assert!(result.is_ok());
    }

    // ── Phase 55 tests (FJ-690 → FJ-697) ──────────────────────────


    #[test]
    fn test_fj692_status_last_apply_duration() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_last_apply_duration(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj697_status_config_hash() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_config_hash(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj697_status_config_hash_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_config_hash(dir.path(), None, true);
        assert!(result.is_ok());
    }

    // ── Phase 56 tests (FJ-700 → FJ-707) ──────────────────────────


    #[test]
    fn test_fj707_status_convergence_history() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_convergence_history(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj707_status_convergence_history_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_convergence_history(dir.path(), None, true);
        assert!(result.is_ok());
    }

    // ── Phase 57 tests (FJ-710 → FJ-717) ──────────────────────────


    #[test]
    fn test_fj712_status_resource_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_resource_inputs(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj717_status_drift_trend() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_drift_trend(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj717_status_drift_trend_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_drift_trend(dir.path(), None, true);
        assert!(result.is_ok());
    }

    // ── Phase 58 tests (FJ-720 → FJ-727) ──────────────────────────


    #[test]
    fn test_fj722_status_failed_resources() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_failed_resources(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj727_status_resource_types_summary() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_resource_types_summary(dir.path(), None, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj727_status_resource_types_summary_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_resource_types_summary(dir.path(), None, true);
        assert!(result.is_ok());

}
}
