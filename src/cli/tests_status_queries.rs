//! Tests: Status query variants.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::status_queries::*;
use super::commands::*;
use super::dispatch::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj205_dispatch_status_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        dispatch(
            Commands::Status(StatusArgs {
                state_dir: state,
                machine: None,
                json: true,
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
            }),
            false,
            true,
        )
        .unwrap();
    }

}
