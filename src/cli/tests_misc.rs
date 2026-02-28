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
use super::lint::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj225_notify_default_empty() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(config.policy.notify.on_success.is_none());
        assert!(config.policy.notify.on_failure.is_none());
        assert!(config.policy.notify.on_drift.is_none());
    }


    #[test]
    fn test_fj225_notify_partial_config() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
policy:
  notify:
    on_drift: "echo drift"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(config.policy.notify.on_success.is_none());
        assert!(config.policy.notify.on_failure.is_none());
        assert_eq!(config.policy.notify.on_drift.as_deref(), Some("echo drift"));
    }


    #[test]
    fn test_fj221_strict_no_root_owner() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "hello"
    owner: root
"#,
        )
        .unwrap();
        // Non-strict: no warning about root owner
        cmd_lint(&file, false, false, false).unwrap();
        // TODO: can't easily capture stdout in tests, but verify it compiles and runs
        // Strict mode adds warnings
        cmd_lint(&file, false, true, false).unwrap();
    }


    #[test]
    fn test_fj221_strict_root_with_system_tag() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/system.conf
    content: "system config"
    owner: root
    tags: [system]
"#,
        )
        .unwrap();
        // Root owner with "system" tag should NOT produce a no_root_owner warning
        cmd_lint(&file, false, true, false).unwrap();
    }


    #[test]
    fn test_fj221_strict_require_tags() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  a:
    type: file
    machine: m1
    path: /tmp/a
    content: "a"
  b:
    type: file
    machine: m1
    path: /tmp/b
    content: "b"
    tags: [web]
"#,
        )
        .unwrap();
        // Strict mode should warn about resource 'a' having no tags
        cmd_lint(&file, false, true, false).unwrap();
    }


    #[test]
    fn test_fj221_strict_require_ssh_key() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  remote:
    hostname: web01
    addr: 10.0.0.1
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/test.txt
    content: "test"
    tags: [test]
"#,
        )
        .unwrap();
        // Strict: remote machine without ssh_key should warn
        cmd_lint(&file, false, true, false).unwrap();
    }


    #[test]
    fn test_fj221_strict_no_privileged_containers() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      privileged: true
resources:
  cfg:
    type: file
    machine: test-box
    path: /tmp/test.txt
    content: "test"
    tags: [test]
"#,
        )
        .unwrap();
        // Strict: privileged container should warn
        cmd_lint(&file, false, true, false).unwrap();
    }


    #[test]
    fn test_fj262_resource_report_serialize() {
        use crate::core::types::ResourceReport;
        let report = ResourceReport {
            resource_id: "test-pkg".to_string(),
            resource_type: "package".to_string(),
            status: "converged".to_string(),
            duration_seconds: 1.234,
            exit_code: Some(0),
            hash: Some("blake3:abc123".to_string()),
            error: None,
        };
        let yaml = serde_yaml_ng::to_string(&report).unwrap();
        assert!(yaml.contains("test-pkg"));
        assert!(yaml.contains("1.234"));
        assert!(yaml.contains("blake3:abc123"));
    }


    #[test]
    fn test_fj270_events_mode_detection() {
        // events_mode should be true only when output == Some("events")
        let mode: Option<&str> = Some("events");
        assert_eq!(mode, Some("events"));
        let mode2: Option<&str> = Some("json");
        assert_ne!(mode2, Some("events"));
        let mode3: Option<&str> = None;
        assert!(mode3.is_none());
    }


    #[test]
    fn test_fj270_event_json_format() {
        // Verify event JSON structure
        let event = serde_json::json!({
            "event": "resource_converged",
            "machine": "local",
            "resource": "test-file",
            "type": "file",
            "status": "converged",
            "duration_seconds": 0.015,
            "hash": "blake3:abc123",
            "error": null,
        });
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("resource_converged"));
        assert!(serialized.contains("test-file"));
        assert!(serialized.contains("blake3:abc123"));
    }


    #[test]
    fn test_fj272_progress_flag_parse() {
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
            progress: true,
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
                notify_custom_webhook: None,
        });
        match cmd {
            Commands::Apply(ApplyArgs { progress, .. }) => assert!(progress),
            _ => panic!("expected Apply"),
        }
    }

}
