//! Tests: Apply command.

#![allow(unused_imports)]
use super::apply::*;
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj226_apply_check_false_runs_normally() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: /tmp/fj226-normal.txt
    content: "hello"
"#;
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(&config, yaml).unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        // check: false runs normal apply
        let result = dispatch(
            Commands::Apply(ApplyArgs {
                file: config,
                machine: None,
                resource: None,
                tag: None,
                group: None,
                force: false,
                dry_run: true,
                no_tripwire: false,
                params: vec![],
                auto_commit: false,
                state_dir: state,
                timeout: None,
                json: false,
                env_file: None,
                workspace: None,
                check: false,
                report: false,
                force_unlock: false,
                output: None,
                progress: false,
                retry: 0,
                yes: false,
                parallel: false,
                timing: false,
                resource_timeout: None,
                rollback_on_failure: false,
                max_parallel: None,
            trace: false,
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
                notify_webhook_headers: None,
                notify_log: None,
                notify_exec: None,
                notify_file: None,
                notify_json: false,
                notify_slack_webhook: None,
                notify_telegram: None,
                notify_webhook_v2: None,
                notify_discord_webhook: None,
                notify_teams_webhook: None,
                notify_slack_blocks: None,
                notify_custom_template: None,
                notify_custom_webhook: None,
                notify_custom_headers: None,
                notify_custom_json: None,
                notify_custom_filter: None,
                notify_custom_retry: None,
                notify_custom_transform: None,
                notify_custom_batch: None,
                notify_custom_deduplicate: None,
                notify_custom_throttle: None,
                notify_custom_aggregate: None,
                notify_custom_priority: None,
                notify_custom_routing: None,
                notify_custom_dedup_window: None,
                notify_custom_rate_limit: None,
                notify_custom_backoff: None,
                notify_custom_circuit_breaker: None,
                notify_custom_dead_letter: None,
                notify_custom_escalation: None,
                notify_custom_correlation: None,
                notify_custom_sampling: None,
                notify_custom_digest: None,
                notify_custom_severity_filter: None,
                refresh_only: false,
                encrypt_state: false,
            operator: None,
            }),
            0,
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj262_apply_writes_last_apply_report() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: report-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  rpt-file:
    type: file
    machine: local
    path: /tmp/forjar-report-test.txt
    content: "report test"
"#,
        )
        .unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            false,
            false,
            &[],
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true,
            false,
            None,
            false,
            None,
            None,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        )
        .unwrap();
        // last-apply.yaml should be written
        let report_path = state.join("local").join("last-apply.yaml");
        assert!(report_path.exists(), "last-apply.yaml should exist");
        let content = std::fs::read_to_string(&report_path).unwrap();
        assert!(
            content.contains("rpt-file"),
            "report should contain resource id"
        );
        assert!(
            content.contains("duration_seconds"),
            "report should contain timing"
        );
        let _ = std::fs::remove_file("/tmp/forjar-report-test.txt");
    }

    #[test]
    fn test_fj262_apply_report_contains_all_resources() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: report-multi
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  rpt-a:
    type: file
    machine: local
    path: /tmp/forjar-rpt-a.txt
    content: "a"
  rpt-b:
    type: file
    machine: local
    path: /tmp/forjar-rpt-b.txt
    content: "b"
"#,
        )
        .unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            false,
            false,
            &[],
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true,
            false,
            None,
            false,
            None,
            None,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        )
        .unwrap();
        let content = std::fs::read_to_string(state.join("local").join("last-apply.yaml")).unwrap();
        assert!(content.contains("rpt-a"));
        assert!(content.contains("rpt-b"));
        let _ = std::fs::remove_file("/tmp/forjar-rpt-a.txt");
        let _ = std::fs::remove_file("/tmp/forjar-rpt-b.txt");
    }

    #[test]
    fn test_fj262_apply_result_includes_reports() {
        use crate::core::types::{ApplyResult, ResourceReport};
        let result = ApplyResult {
            machine: "web".to_string(),
            resources_converged: 1,
            resources_unchanged: 0,
            resources_failed: 0,
            total_duration: std::time::Duration::from_millis(500),
            resource_reports: vec![ResourceReport {
                resource_id: "pkg".to_string(),
                resource_type: "package".to_string(),
                status: "converged".to_string(),
                duration_seconds: 0.5,
                exit_code: Some(0),
                hash: None,
                error: None,
            }],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("resource_reports"));
        assert!(json.contains("\"pkg\""));
        assert!(json.contains("0.5"));
    }

    #[test]
    fn test_fj262_save_and_load_apply_report() {
        use crate::core::state::{load_apply_report, save_apply_report};
        use crate::core::types::ApplyResult;
        let dir = tempfile::tempdir().unwrap();
        let result = ApplyResult {
            machine: "test-m".to_string(),
            resources_converged: 2,
            resources_unchanged: 1,
            resources_failed: 0,
            total_duration: std::time::Duration::from_millis(750),
            resource_reports: Vec::new(),
        };
        save_apply_report(dir.path(), &result).unwrap();
        let loaded = load_apply_report(dir.path(), "test-m").unwrap();
        assert!(loaded.is_some());
        let content = loaded.unwrap();
        assert!(content.contains("test-m"));
        assert!(content.contains("resources_converged: 2"));
    }

    #[test]
    fn test_fj262_load_apply_report_missing() {
        use crate::core::state::load_apply_report;
        let dir = tempfile::tempdir().unwrap();
        let loaded = load_apply_report(dir.path(), "nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_fj270_apply_complete_event() {
        let event = serde_json::json!({
            "event": "apply_complete",
            "machine": "local",
            "converged": 3,
            "unchanged": 1,
            "failed": 0,
            "duration_seconds": 1.5,
        });
        let s = serde_json::to_string(&event).unwrap();
        assert!(s.contains("apply_complete"));
        assert!(s.contains("\"converged\":3"));
        assert!(s.contains("\"failed\":0"));
    }
}
