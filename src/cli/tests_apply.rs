//! Tests: Apply command.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::apply::*;
use super::commands::*;
use super::dispatch::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj017_apply_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-cli-dry-run.txt
    content: "test"
"#,
        )
        .unwrap();
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            true,
            false,
            &[],
            false,
            None, // no timeout
            false,
            false,
            None,  // no env_file
            None,  // no workspace
            false, // no report
            false, // no force_unlock
            None,  // no output mode
            false, // no progress
            false, // no timing
            0,     // no retry
            true,  // yes (skip prompt)
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
    }


    #[test]
    fn test_fj017_apply_real() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-cli-apply-test.txt
    content: "hello from cli test"
policy:
  tripwire: true
  lock_file: true
"#,
        )
        .unwrap();
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
            None, // no timeout
            false,
            false,
            None,  // no env_file
            None,  // no workspace
            false, // no report
            false, // no force_unlock
            None,  // no output mode
            false, // no progress
            false, // no timing
            0,     // no retry
            true,  // yes (skip prompt)
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

        // Verify file was created
        assert!(std::path::Path::new("/tmp/forjar-cli-apply-test.txt").exists());

        // Verify lock was saved
        let lock = crate::core::state::load_lock(&state, "local").unwrap();
        assert!(lock.is_some());

        let _ = std::fs::remove_file("/tmp/forjar-cli-apply-test.txt");
    }


    #[test]
    fn test_fj017_apply_validation_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::write(
            &config,
            r#"
version: "2.0"
name: ""
machines: {}
resources: {}
"#,
        )
        .unwrap();
        let result = cmd_apply(
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
            None, // no timeout
            false,
            false,
            None,  // no env_file
            None,  // no workspace
            false, // no report
            false, // no force_unlock
            None,  // no output mode
            false, // no progress
            false, // no timing
            0,     // no retry
            true,  // yes (skip prompt)
            false,
            None,
            false,
            None,
            None,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("validation"));
    }


    #[test]
    fn test_fj017_dispatch_apply_dry() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
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
    path: /tmp/forjar-dispatch-dry.txt
    content: "x"
"#,
        )
        .unwrap();
        dispatch(
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
            }),
            false,
            true,
        )
        .unwrap();
    }

}
