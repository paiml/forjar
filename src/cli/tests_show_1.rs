//! Tests: Show, explain, compare, template.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::show::*;
use super::commands::*;
use super::dispatch::*;
use super::lint::*;
use super::print_helpers::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj221_strict_json_output() {
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
    path: /tmp/test.txt
    content: "test"
    owner: root
"#,
        )
        .unwrap();
        // JSON + strict should not crash
        cmd_lint(&file, true, true, false).unwrap();
    }

    // FJ-251: Doctor tests


    #[test]
    fn test_fj274_unified_diff_shows_changes() {
        // Update with old content — should show unified diff
        print_content_diff(
            "new line\nkept",
            &types::PlanAction::Update,
            Some("old line\nkept"),
        );
    }


    #[test]
    fn test_fj262_report_flag_with_json_output() {
        use crate::core::types::{ApplyResult, ResourceReport};
        // Verify that resource_reports are included in JSON output
        let result = ApplyResult {
            machine: "gpu-box".to_string(),
            resources_converged: 1,
            resources_unchanged: 0,
            resources_failed: 0,
            total_duration: std::time::Duration::from_secs(2),
            resource_reports: vec![ResourceReport {
                resource_id: "cuda-driver".to_string(),
                resource_type: "gpu".to_string(),
                status: "converged".to_string(),
                duration_seconds: 2.0,
                exit_code: Some(0),
                hash: Some("blake3:deadbeef".to_string()),
                error: None,
            }],
        };
        let output = serde_json::json!({
            "machines": [&result],
            "summary": {
                "total_converged": 1,
                "total_unchanged": 0,
                "total_failed": 0,
            }
        });
        let json_str = serde_json::to_string_pretty(&output).unwrap();
        assert!(json_str.contains("cuda-driver"));
        assert!(json_str.contains("resource_reports"));
    }

    // ── FJ-263: Colored CLI output ──


    #[test]
    fn test_fj270_output_flag_parse() {
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
            output: Some("events".to_string()),
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
        });
        match cmd {
            Commands::Apply(ApplyArgs { output, .. }) => {
                assert_eq!(output.as_deref(), Some("events"));
            }
            _ => panic!("expected Apply"),
        }
    }

    // ========================================================================
    // FJ-271: forjar explain
    // ========================================================================


    #[test]
    fn test_fj271_explain_resource_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test:\n    type: file\n    machine: local\n    path: /tmp/test.txt\n    content: test\n").unwrap();
        let result = dispatch(
            Commands::Explain(ExplainArgs {
                file: config_path,
                resource: "nonexistent".to_string(),
                json: false,
            }),
            false,
            true,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }


    #[test]
    fn test_fj271_explain_valid_resource() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  test:\n    type: file\n    machine: local\n    path: /tmp/explain-test.txt\n    content: hello\n").unwrap();
        let result = dispatch(
            Commands::Explain(ExplainArgs {
                file: config_path,
                resource: "test".to_string(),
                json: false,
            }),
            false,
            true,
        );
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj271_explain_with_templates() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, "version: \"1.0\"\nname: test\nparams:\n  dir: /opt/app\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: local\n    path: \"{{params.dir}}/config.txt\"\n    content: test\n").unwrap();
        let result = dispatch(
            Commands::Explain(ExplainArgs {
                file: config_path,
                resource: "cfg".to_string(),
                json: false,
            }),
            false,
            true,
        );
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj271_explain_command_parse() {
        let cmd = Commands::Explain(ExplainArgs {
            file: PathBuf::from("forjar.yaml"),
            resource: "my-resource".to_string(),
            json: false,
        });
        match cmd {
            Commands::Explain(ExplainArgs { resource, .. }) => assert_eq!(resource, "my-resource"),
            _ => panic!("expected Explain"),
        }
    }

    // ========================================================================
    // FJ-272: Apply progress indicator
    // ========================================================================


    #[test]
    fn test_fj307_explain_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: explain-test
params:
  dir: /opt/app
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: "{{params.dir}}/config.txt"
    content: test
    tags: [web]
    depends_on: []
"#,
        )
        .unwrap();
        let result = cmd_explain(&config_path, "cfg", true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj307_explain_json_flag_parse() {
        let cmd = Commands::Explain(ExplainArgs {
            file: PathBuf::from("forjar.yaml"),
            resource: "my-resource".to_string(),
            json: true,
        });
        match cmd {
            Commands::Explain(ExplainArgs { json, .. }) => assert!(json),
            _ => panic!("expected Explain"),
        }
    }

    // ── FJ-310: apply --rollback-on-failure ──


    #[test]
    fn test_fj363_compare_parse() {
        let cmd = Commands::Compare(CompareArgs {
            file1: PathBuf::from("a.yaml"),
            file2: PathBuf::from("b.yaml"),
            json: false,
        });
        match cmd {
            Commands::Compare(CompareArgs { file1, file2, .. }) => {
                assert_eq!(file1, PathBuf::from("a.yaml"));
                assert_eq!(file2, PathBuf::from("b.yaml"));
            }
            _ => panic!("expected Compare"),
        }
    }

}
