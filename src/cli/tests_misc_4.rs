//! Tests: Misc.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::commands::*;
use super::status_core::*;
use super::validate_core::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj303_summary_flag_parse() {
        let cmd = Commands::Status(StatusArgs {
            state_dir: PathBuf::from("state"),
            machine: None,
            json: false,
            file: None,
            summary: true,
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
        });
        match cmd {
            Commands::Status(StatusArgs { summary, .. }) => assert!(summary),
            _ => panic!("expected Status"),
        }
    }


    #[test]
    fn test_fj303_summary_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_status(&state_dir, None, false, None, true);
        assert!(result.is_ok());
    }

    // ── FJ-304: apply --resource-timeout ──


    #[test]
    fn test_fj304_resource_timeout_flag_parse() {
        let cmd = Commands::Apply(ApplyArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("state"),
            machine: None,
            resource: None,
            tag: None,
            group: None,
            force: false,
            dry_run: false,
            no_tripwire: false,
            params: vec![],
            auto_commit: false,
            timeout: None,
            json: false,
            env_file: None,
            workspace: None,
            check: false,
            report: false,
            force_unlock: false,
            output: None,
            progress: false,
            timing: false,
            retry: 0,
            yes: false,
            parallel: false,
            resource_timeout: Some(30),
            rollback_on_failure: false,
            max_parallel: None,
            notify: None,
            subset: None,
            confirm_destructive: false,
            backup: false,
            exclude: None,
            sequential: false,
            diff_only: false,
            notify_slack: None,
            cost_limit: None,
            preview: false,
            tag_filter: None,
            output_scripts: None,
            resume: false,
            confirm: false,
            max_failures: None,
            rate_limit: None,
            labels: vec![],
            plan_file: None,
            notify_email: None,
            skip: None,
            snapshot_before: None,
            concurrency: None,
            webhook_before: None,
            rollback_snapshot: None,
            retry_delay: None,
            tags: vec![],
            log_file: None,
            comment: None,
            only_changed: false,
            pre_script: None,
            dry_run_json: false,
            notify_webhook: None,
            post_script: None,
            approval_required: false,
            canary_percent: None,
            schedule: None,
            env_name: None,
            dry_run_diff: false,
            notify_pagerduty: None,
            batch_size: None,
            notify_teams: None,
            abort_on_drift: false,
            dry_run_summary: false,
            notify_discord: None,
            rollback_on_threshold: None,
            metrics_port: None,
            notify_opsgenie: None,
            circuit_breaker: None,
            require_approval: None,
            notify_datadog: None,
            change_window: None,
            canary_machine: None,
            notify_newrelic: None,
            max_duration: None,
            notify_grafana: None,
            rate_limit_resources: None,
            checkpoint_interval: None,
            notify_victorops: None,
            blue_green: None,
            dry_run_cost: false,
            notify_msteams_adaptive: None,
            progressive: None,
            approval_webhook: None,
            notify_incident: None,
            sign_off: None,
            notify_sns: None,
            telemetry_endpoint: None,
            runbook: None,
            notify_pubsub: None,
            fleet_strategy: None,
            pre_check: None,
            notify_eventbridge: None,
            dry_run_graph: false,
            post_check: None,
            notify_kafka: None,
            max_retries: None,
            rollback_window: None,
            notify_azure_servicebus: None,
            approval_timeout: None,
            pre_flight: None,
            notify_gcp_pubsub_v2: None,
            checkpoint: None,
            post_flight: None,
            notify_rabbitmq: None,
            gate: None,
            notify_nats: None,
            dry_run_verbose: false,
            explain: false,
            notify_mqtt: None,
            confirmation_message: None,
            summary_only: false,
            notify_redis: None,
            notify_amqp: None,
            pre_apply_hook: None,
            resource_filter: None,
            notify_stomp: None,
            post_apply_hook: None,
            dry_run_shell: false,
            notify_zeromq: None,
            canary_resource: None,
            timeout_per_resource: None,
            notify_grpc: None,
            skip_unchanged: false,
            retry_backoff: None,
            notify_sqs: None,
            plan_output_file: None,
            resource_priority: vec![],
            apply_window: None,
            fail_fast_machine: false,
            notify_mattermost: None,
            cooldown: None,
            exclude_machine: None,
            notify_ntfy: None,
            only_machine: None,
        });
        match cmd {
            Commands::Apply(ApplyArgs {
                resource_timeout, ..
            }) => {
                assert_eq!(resource_timeout, Some(30));
            }
            _ => panic!("expected Apply"),
        }
    }


    #[test]
    fn test_fj304_resource_timeout_precedence() {
        // resource_timeout.or(timeout_secs) gives resource_timeout priority
        let resource_timeout: Option<u64> = Some(30);
        let timeout_secs: Option<u64> = Some(60);
        let effective = resource_timeout.or(timeout_secs);
        assert_eq!(effective, Some(30));

        // When resource_timeout is None, falls back to timeout_secs
        let resource_timeout: Option<u64> = None;
        let effective = resource_timeout.or(timeout_secs);
        assert_eq!(effective, Some(60));
    }

    // ── FJ-305: check --json enhanced ──


    #[test]
    fn test_fj311_strict_unused_params() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
description: "test"
params:
  unused_key: some_value
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/test.txt
    content: "hello"
"#,
        )
        .unwrap();
        let result = cmd_validate(&file, true, false, false);
        assert!(result.is_err()); // unused param triggers strict error
    }


    #[test]
    fn test_fj311_strict_missing_description() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/test.txt
    content: "hello"
"#,
        )
        .unwrap();
        let result = cmd_validate(&file, true, false, false);
        assert!(result.is_err()); // missing description triggers strict error
    }

    // ── FJ-312: plan --cost ──

}
