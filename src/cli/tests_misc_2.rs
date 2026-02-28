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
use super::validate_core::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj281_group_field_parse() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  web-pkg:
    type: file
    machine: local
    path: /tmp/a.txt
    content: a
    resource_group: network
  db-pkg:
    type: file
    machine: local
    path: /tmp/b.txt
    content: b
    resource_group: database
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(
            config.resources["web-pkg"].resource_group,
            Some("network".to_string())
        );
        assert_eq!(
            config.resources["db-pkg"].resource_group,
            Some("database".to_string())
        );
    }


    #[test]
    fn test_fj281_group_default_none() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  pkg:
    type: file
    machine: local
    path: /tmp/a.txt
    content: a
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.resources["pkg"].resource_group, None);
    }


    #[test]
    fn test_fj282_strict_catches_relative_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
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
    path: relative/path.txt
    content: "hello"
"#;
        std::fs::write(&file, yaml).unwrap();
        let result = cmd_validate(&file, true, false, false);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("strict validation failed"));
    }


    #[test]
    fn test_fj282_strict_flag_parse() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
            strict: true,
            json: false,
            dry_expand: false,
            schema_version: None,
            exhaustive: false,
            policy_file: None,
            check_connectivity: false,
            check_templates: false,
            strict_deps: false,
            check_secrets: false,
            check_idempotency: false,
            check_drift_coverage: false,
            check_cycles_deep: false,
            check_naming: false,
            check_overlaps: false,
            check_limits: false,
            check_complexity: false,
            check_security: false,
            check_deprecation: false,
            check_drift_risk: false,
            check_compliance: None,
            check_portability: false,
            check_resource_limits: false,
            check_unused: false,
            check_dependencies: false,
            check_permissions: false,
            check_idempotency_deep: false,
            check_machine_reachability: false,
            check_circular_refs: false,
            check_naming_conventions: false,
            check_owner_consistency: false,
            check_path_conflicts: false,
            check_service_deps: false,
            check_template_vars: false,
            check_mode_consistency: false,
            check_group_consistency: false,
            check_mount_points: false,
            check_cron_syntax: false,
            check_env_refs: false,
            check_resource_names: None,
            check_resource_count: None,
            check_duplicate_paths: false,
        check_circular_deps: false,
        check_machine_refs: false,
        check_provider_consistency: false,
        check_state_values: false,
        check_unused_machines: false,
        check_tag_consistency: false,
            check_dependency_exists: false,
            check_path_conflicts_strict: false,
            check_duplicate_names: false,
            check_resource_groups: false,
            check_orphan_resources: false,
            check_machine_arch: false, check_resource_health_conflicts: false, check_resource_overlap: false, check_resource_tags: false, check_resource_state_consistency: false, check_resource_dependencies_complete: false, check_machine_connectivity: false, check_resource_naming_pattern: None, check_resource_provider_support: false, check_resource_secret_refs: false, check_resource_idempotency_hints: false,
                check_resource_dependency_depth: None,
                check_resource_machine_affinity: false,
                check_resource_drift_risk: false, check_resource_tag_coverage: false, check_resource_lifecycle_hooks: false, check_resource_provider_version: false, check_resource_naming_convention: false, check_resource_idempotency: false,
        });
        match cmd {
            Commands::Validate(ValidateArgs { strict, .. }) => assert!(strict),
            _ => panic!("expected Validate"),
        }
    }


    #[test]
    fn test_fj282_strict_passes_clean_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
description: "test project"
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
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_validate(&file, true, false, false).unwrap();
    }


    #[test]
    fn test_fj284_parse_duration_invalid() {
        assert!(parse_duration_secs("abc").is_err());
        assert!(parse_duration_secs("10x").is_err());
        assert!(parse_duration_secs("").is_err());
    }


    #[test]
    fn test_fj284_since_flag_parse() {
        let cmd = Commands::History(HistoryArgs {
            state_dir: PathBuf::from("state"),
            machine: None,
            limit: 10,
            json: false,
            since: Some("24h".to_string()),
            resource: None,
        });
        match cmd {
            Commands::History(HistoryArgs { since, .. }) => {
                assert_eq!(since, Some("24h".to_string()));
            }
            _ => panic!("expected History"),
        }
    }


    #[test]
    fn test_fj285_target_flag_parse() {
        let cmd = Commands::Plan(PlanArgs {
            file: PathBuf::from("forjar.yaml"),
            machine: None,
            resource: None,
            tag: None,
            group: None,
            state_dir: PathBuf::from("state"),
            json: false,
            output_dir: None,
            env_file: None,
            workspace: None,
            no_diff: false,
            target: Some("web-config".to_string()),
            cost: false,
            what_if: vec![],
        });
        match cmd {
            Commands::Plan(PlanArgs { target, .. }) => {
                assert_eq!(target, Some("web-config".to_string()));
            }
            _ => panic!("expected Plan"),
        }
    }


    #[test]
    fn test_fj286_yes_flag_parse() {
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
            yes: true,
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
            notify_webhook_v2: None, notify_discord_webhook: None, notify_teams_webhook: None, notify_slack_blocks: None, notify_custom_template: None, notify_custom_webhook: None, notify_custom_headers: None, notify_custom_json: None, notify_custom_filter: None,
        });
        match cmd {
            Commands::Apply(ApplyArgs { yes, .. }) => assert!(yes),
            _ => panic!("expected Apply"),
        }
    }

}
