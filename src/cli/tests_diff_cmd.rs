//! Tests: Diff and env commands.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::diff_cmd::*;
use super::apply_helpers::*;
use super::commands::*;
use super::dispatch::*;
use super::print_helpers::*;
use super::test_fixtures::*;
use std::sync::atomic::Ordering;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj131_cmd_diff_empty_state_dirs() {
        let from = tempfile::tempdir().unwrap();
        let to = tempfile::tempdir().unwrap();
        let err = cmd_diff(from.path(), to.path(), None, None, false).unwrap_err();
        assert!(err.contains("no machines found"));
    }


    #[test]
    fn test_fj131_cmd_diff_same_state() {
        let state = tempfile::tempdir().unwrap();
        // Create a machine state directory with a lock
        let machine_dir = state.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        let lock = types::StateLock {
            schema: "1.0".to_string(),
            machine: "web".to_string(),
            hostname: "web-box".to_string(),
            generated_at: "2026-02-25T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources: {
                let mut r = indexmap::IndexMap::new();
                r.insert(
                    "test-file".to_string(),
                    types::ResourceLock {
                        resource_type: types::ResourceType::File,
                        status: types::ResourceStatus::Converged,
                        applied_at: Some("2026-02-25T00:00:00Z".to_string()),
                        duration_seconds: Some(0.1),
                        hash: "blake3:abc123".to_string(),
                        details: HashMap::new(),
                    },
                );
                r
            },
        };
        state::save_lock(state.path(), &lock).unwrap();

        // Diff same directory against itself → no differences
        cmd_diff(state.path(), state.path(), None, None, false).unwrap();
    }


    #[test]
    fn test_fj131_cmd_diff_added_resource() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();

        // "from" has empty lock for web
        let from_machine = from_dir.path().join("web");
        std::fs::create_dir_all(&from_machine).unwrap();
        let from_lock = types::StateLock {
            schema: "1.0".to_string(),
            machine: "web".to_string(),
            hostname: "web-box".to_string(),
            generated_at: "2026-02-25T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources: indexmap::IndexMap::new(),
        };
        state::save_lock(from_dir.path(), &from_lock).unwrap();

        // "to" has one resource
        let to_machine = to_dir.path().join("web");
        std::fs::create_dir_all(&to_machine).unwrap();
        let mut to_lock = from_lock.clone();
        to_lock.resources.insert(
            "new-file".to_string(),
            types::ResourceLock {
                resource_type: types::ResourceType::File,
                status: types::ResourceStatus::Converged,
                applied_at: Some("2026-02-25T01:00:00Z".to_string()),
                duration_seconds: Some(0.2),
                hash: "blake3:def456".to_string(),
                details: HashMap::new(),
            },
        );
        state::save_lock(to_dir.path(), &to_lock).unwrap();

        cmd_diff(from_dir.path(), to_dir.path(), None, None, false).unwrap();
    }


    #[test]
    fn test_fj131_cmd_diff_machine_filter() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();

        // Create two machines
        for name in ["web", "db"] {
            let lock = types::StateLock {
                schema: "1.0".to_string(),
                machine: name.to_string(),
                hostname: format!("{}-box", name),
                generated_at: "2026-02-25T00:00:00Z".to_string(),
                generator: "forjar 0.1.0".to_string(),
                blake3_version: "1.8".to_string(),
                resources: indexmap::IndexMap::new(),
            };
            state::save_lock(from_dir.path(), &lock).unwrap();
            state::save_lock(to_dir.path(), &lock).unwrap();
        }

        // Filter to only "web" — should succeed
        cmd_diff(from_dir.path(), to_dir.path(), Some("web"), None, false).unwrap();
    }

    // ── FJ-131: cmd_anomaly tests ─────────────────────────────────


    #[test]
    fn test_fj211_env_file_overridden_by_param() {
        // Env file sets a value, then --param overrides it further
        let dir = tempfile::tempdir().unwrap();
        let file = write_env_config(dir.path());
        let env = dir.path().join("base.env.yaml");
        std::fs::write(&env, "log_level: debug\n").unwrap();

        let mut config = parse_and_validate(&file).unwrap();
        load_env_params(&mut config, &env).unwrap();
        apply_param_overrides(&mut config, &["log_level=trace".to_string()]).unwrap();

        // --param should win over env file
        assert_eq!(
            config.params.get("log_level").unwrap(),
            &serde_yaml_ng::Value::String("trace".to_string())
        );
    }

    // ================================================================
    // FJ-210: Workspace tests
    // ================================================================


    #[test]
    fn test_fj274_unified_diff_no_old_content() {
        // Update without old content — falls back to ~ prefix
        print_content_diff("new content", &types::PlanAction::Update, None);
    }


    #[test]
    fn test_fj263_no_color_env_respected() {
        // Verify NO_COLOR flag disables all color functions
        NO_COLOR.store(true, Ordering::Relaxed);
        assert_eq!(red("x"), "x");
        assert_eq!(green("x"), "x");
        assert_eq!(yellow("x"), "x");
        assert_eq!(dim("x"), "x");
        assert_eq!(bold("x"), "x");
        // Reset for other tests
        NO_COLOR.store(false, Ordering::Relaxed);
    }


    #[test]
    fn test_fj277_env_no_config() {
        let result = dispatch(
            Commands::Env(EnvArgs {
                file: PathBuf::from("/tmp/nonexistent-forjar.yaml"),
                json: false,
            }),
            false,
            true,
        );
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj277_env_with_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, "version: \"1.0\"\nname: my-project\nmachines:\n  a:\n    hostname: a\n    addr: 1.2.3.4\n  b:\n    hostname: b\n    addr: 5.6.7.8\nresources:\n  pkg:\n    type: file\n    machine: a\n    path: /tmp/x\n    content: x\n").unwrap();
        let result = dispatch(
            Commands::Env(EnvArgs {
                file: config_path,
                json: false,
            }),
            false,
            true,
        );
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj277_env_json() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, "version: \"1.0\"\nname: test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: local\n    path: /tmp/x\n    content: x\n").unwrap();
        let result = dispatch(
            Commands::Env(EnvArgs {
                file: config_path,
                json: true,
            }),
            false,
            true,
        );
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj277_env_command_parse() {
        let cmd = Commands::Env(EnvArgs {
            file: PathBuf::from("forjar.yaml"),
            json: true,
        });
        match cmd {
            Commands::Env(EnvArgs { json, .. }) => assert!(json),
            _ => panic!("expected Env"),
        }
    }

    // ========================================================================
    // FJ-273: forjar test
    // ========================================================================


    #[test]
    fn test_fj291_diff_resource_flag_parse() {
        let cmd = Commands::Diff(DiffArgs {
            from: PathBuf::from("state-a"),
            to: PathBuf::from("state-b"),
            machine: None,
            resource: Some("web-config".to_string()),
            json: false,
        });
        match cmd {
            Commands::Diff(DiffArgs { resource, .. }) => {
                assert_eq!(resource, Some("web-config".to_string()));
            }
            _ => panic!("expected Diff"),
        }
    }

    // ── FJ-292: status --json enriched output ──


    #[test]
    fn test_fj306_env_json_resolved_params() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: env-test
params:
  data_dir: /mnt/data
  log_level: info
machines:
  box1:
    hostname: box1
    addr: 127.0.0.1
resources:
  pkg:
    type: file
    machine: box1
    path: /tmp/test
    content: hello
"#,
        )
        .unwrap();
        let result = cmd_env(&config_path, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj306_env_json_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nonexistent.yaml");
        let result = cmd_env(&missing, true);
        assert!(result.is_ok());
    }

    // ── FJ-307: explain --json ──


    #[test]
    fn test_fj350_diff_only_flag() {
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
            resource_timeout: None,
            rollback_on_failure: false,
            max_parallel: None,
            notify: None,
            subset: None,
            confirm_destructive: false,
            backup: false,
            exclude: None,
            sequential: false,
            diff_only: true,
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
                notify_custom_webhook: None, notify_custom_headers: None, notify_custom_json: None, notify_custom_filter: None, notify_custom_retry: None, notify_custom_transform: None, notify_custom_batch: None, notify_custom_deduplicate: None, notify_custom_throttle: None, notify_custom_aggregate: None, notify_custom_priority: None, notify_custom_routing: None, notify_custom_dedup_window: None, notify_custom_rate_limit: None, notify_custom_backoff: None, notify_custom_circuit_breaker: None,
        });
        match cmd {
            Commands::Apply(ApplyArgs { diff_only, .. }) => assert!(diff_only),
            _ => panic!("expected Apply"),
        }
    }


    #[test]
    fn test_fj367_env_diff_parse() {
        let cmd = Commands::EnvDiff(EnvDiffArgs {
            env1: "staging".to_string(),
            env2: "production".to_string(),
            state_dir: PathBuf::from("state"),
            json: false,
        });
        match cmd {
            Commands::EnvDiff(EnvDiffArgs { env1, env2, .. }) => {
                assert_eq!(env1, "staging");
                assert_eq!(env2, "production");
            }
            _ => panic!("expected EnvDiff"),
        }
    }

}
