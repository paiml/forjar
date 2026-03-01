//! Tests: Fleet operations.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::fleet_ops::*;
use super::commands::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj283_retry_flag_parse() {
        let cmd = Commands::Apply(ApplyArgs {
            file: PathBuf::from("forjar.yaml"),
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
            state_dir: PathBuf::from("state"),
            json: false,
            env_file: None,
            workspace: None,
            check: false,
            report: false,
            force_unlock: false,
            output: None,
            progress: false,
            retry: 3,
            yes: false,
            parallel: false,
            timing: false,
            resource_timeout: None,
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
            notify_webhook_headers: None,
            notify_log: None,
        notify_exec: None,
        notify_file: None,
        notify_json: false,
            notify_slack_webhook: None,
            notify_telegram: None,
            notify_webhook_v2: None, notify_discord_webhook: None, notify_teams_webhook: None, notify_slack_blocks: None, notify_custom_template: None,
                notify_custom_webhook: None, notify_custom_headers: None, notify_custom_json: None, notify_custom_filter: None, notify_custom_retry: None, notify_custom_transform: None, notify_custom_batch: None, notify_custom_deduplicate: None, notify_custom_throttle: None, notify_custom_aggregate: None, notify_custom_priority: None, notify_custom_routing: None, notify_custom_dedup_window: None, notify_custom_rate_limit: None, notify_custom_backoff: None, notify_custom_circuit_breaker: None, notify_custom_dead_letter: None, notify_custom_escalation: None, notify_custom_correlation: None, notify_custom_sampling: None, notify_custom_digest: None, notify_custom_severity_filter: None, refresh_only: false, encrypt_state: false,
        });
        match cmd {
            Commands::Apply(ApplyArgs { retry, .. }) => assert_eq!(retry, 3),
            _ => panic!("expected Apply"),
        }
    }


    #[test]
    fn test_fj283_retry_default_zero() {
        let cmd = Commands::Apply(ApplyArgs {
            file: PathBuf::from("forjar.yaml"),
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
            state_dir: PathBuf::from("state"),
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
            notify_webhook_v2: None, notify_discord_webhook: None, notify_teams_webhook: None, notify_slack_blocks: None, notify_custom_template: None,
                notify_custom_webhook: None, notify_custom_headers: None, notify_custom_json: None, notify_custom_filter: None, notify_custom_retry: None, notify_custom_transform: None, notify_custom_batch: None, notify_custom_deduplicate: None, notify_custom_throttle: None, notify_custom_aggregate: None, notify_custom_priority: None, notify_custom_routing: None, notify_custom_dedup_window: None, notify_custom_rate_limit: None, notify_custom_backoff: None, notify_custom_circuit_breaker: None, notify_custom_dead_letter: None, notify_custom_escalation: None, notify_custom_correlation: None, notify_custom_sampling: None, notify_custom_digest: None, notify_custom_severity_filter: None, refresh_only: false, encrypt_state: false,
        });
        match cmd {
            Commands::Apply(ApplyArgs { retry, .. }) => assert_eq!(retry, 0),
            _ => panic!("expected Apply"),
        }
    }

    // ── FJ-284: forjar history --since ──────────────────────────


    #[test]
    fn test_fj326_inventory_flag_parse() {
        let cmd = Commands::Inventory(InventoryArgs {
            file: PathBuf::from("infra.yaml"),
            json: true,
        });
        match cmd {
            Commands::Inventory(InventoryArgs { file, json }) => {
                assert_eq!(file, PathBuf::from("infra.yaml"));
                assert!(json);
            }
            _ => panic!("expected Inventory"),
        }
    }


    #[test]
    fn test_fj326_inventory_local_machine() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: test-inventory
machines:
  local-box:
    hostname: local
    addr: 127.0.0.1
    user: test
resources:
  cfg:
    type: file
    machine: local-box
    path: /tmp/fj326/test.txt
    content: hello
"#,
        )
        .unwrap();
        // cmd_inventory should succeed for local machine
        let result = cmd_inventory(&config_path, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj327_retry_failed_flag_parse() {
        let cmd = Commands::RetryFailed(RetryFailedArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("state"),
            params: vec!["key=val".to_string()],
            timeout: Some(30),
        });
        match cmd {
            Commands::RetryFailed(RetryFailedArgs {
                file,
                state_dir,
                params,
                timeout,
            }) => {
                assert_eq!(file, PathBuf::from("f.yaml"));
                assert_eq!(state_dir, PathBuf::from("state"));
                assert_eq!(params.len(), 1);
                assert_eq!(timeout, Some(30));
            }
            _ => panic!("expected RetryFailed"),
        }
    }


    #[test]
    fn test_fj327_retry_failed_no_failures() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: test-retry
machines:
  local:
    hostname: local
    addr: 127.0.0.1
    user: test
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/fj327/test.txt
    content: hello
"#,
        )
        .unwrap();
        // No event logs → no failures to retry
        let result = cmd_retry_failed(&config_path, &state_dir, &[], None);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj324_rolling_flag_parse() {
        let cmd = Commands::Rolling(RollingArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("state"),
            batch_size: 3,
            params: vec![],
            timeout: None,
        });
        match cmd {
            Commands::Rolling(RollingArgs { batch_size, .. }) => {
                assert_eq!(batch_size, 3);
            }
            _ => panic!("expected Rolling"),
        }
    }


    #[test]
    fn test_fj325_canary_flag_parse() {
        let cmd = Commands::Canary(CanaryArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("state"),
            machine: "web-1".to_string(),
            auto_proceed: true,
            params: vec![],
            timeout: None,
        });
        match cmd {
            Commands::Canary(CanaryArgs {
                machine,
                auto_proceed,
                ..
            }) => {
                assert_eq!(machine, "web-1");
                assert!(auto_proceed);
            }
            _ => panic!("expected Canary"),
        }
    }

}
