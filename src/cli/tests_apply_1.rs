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
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj017_apply_with_results_summary() {
        // Tests the full apply path with real local execution, covering the
        // results iteration and summary output lines
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target = dir.path().join("apply-summary.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  summary-file:
    type: file
    machine: local
    path: {}
    content: "summary test"
"#,
                target.display()
            ),
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
        assert!(target.exists());

        // Second apply — should be unchanged (NoOp)
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
    }


    #[test]
    fn test_fj017_apply_with_param_override() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: param-test
params:
  env: dev
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  conf:
    type: file
    machine: local
    path: /tmp/forjar-param-test.txt
    content: "env={{params.env}}"
"#,
        )
        .unwrap();
        // Apply with param override in dry-run
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            true, // dry-run
            false,
            &["env=prod".to_string()],
            false,
            None,
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

    // ── Lint edge cases ────────────────────────────────────────


    #[test]
    fn test_fj205_apply_json_dry_run() {
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
    path: /tmp/forjar-fj205-test.txt
    content: "x"
"#,
        )
        .unwrap();
        // Dry-run with json=true should succeed (dry run exits before JSON output)
        let result = cmd_apply(
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
            true,
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
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj205_apply_result_serialize() {
        // Verify ApplyResult serializes correctly (duration as f64 seconds)
        use crate::core::types::ApplyResult;
        let result = ApplyResult {
            machine: "web".to_string(),
            resources_converged: 3,
            resources_unchanged: 1,
            resources_failed: 0,
            total_duration: std::time::Duration::from_millis(1500),
            resource_reports: Vec::new(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"machine\":\"web\""));
        assert!(json.contains("\"resources_converged\":3"));
        assert!(json.contains("\"total_duration\":1.5"));
    }


    #[test]
    fn test_fj205_dispatch_apply_json() {
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
    path: /tmp/forjar-fj205-dispatch.txt
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
                json: true,
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

    // ================================================================
    // FJ-214: state-list tests
    // ================================================================

    fn make_test_lock(
        machine: &str,
        resources: indexmap::IndexMap<String, types::ResourceLock>,
    ) -> types::StateLock {
        types::StateLock {
            schema: "1.0".to_string(),
            machine: machine.to_string(),
            hostname: machine.to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        }
    }

    fn make_test_resource_lock(rtype: types::ResourceType) -> types::ResourceLock {
        types::ResourceLock {
            resource_type: rtype,
            status: types::ResourceStatus::Converged,
            applied_at: Some("2026-01-15T10:30:00Z".to_string()),
            duration_seconds: Some(0.5),
            hash: "blake3:abcdef123456".to_string(),
            details: HashMap::new(),
        }
    }

}
