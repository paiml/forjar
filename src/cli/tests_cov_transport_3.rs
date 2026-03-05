//! Tests: Coverage for parser/model, exec_script_retry, drift, notify extras (part 3 of 3).
//! Covers: validate_model, exec_script_retry, cmd_drift_dry_run, additional notify/doctor.

#![allow(unused_imports)]
use std::io::Write;
use std::path::{Path, PathBuf};

use super::dispatch_notify::*;
use super::doctor::*;
use super::drift::*;
use super::helpers::*;
use super::observe::*;
use super::status_convergence::*;
use super::test_fixtures::*;
use crate::core::{parser, state, types};
use crate::transport;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn minimal_config_yaml() -> &'static str {
        r#"version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: /tmp/forjar-cov-transport-test.txt
    content: "hello"
"#
    }

    fn empty_notify_opts<'a>() -> NotifyOpts<'a> {
        NotifyOpts {
            slack: None,
            email: None,
            webhook: None,
            teams: None,
            discord: None,
            opsgenie: None,
            datadog: None,
            newrelic: None,
            grafana: None,
            victorops: None,
            msteams_adaptive: None,
            incident: None,
            sns: None,
            pubsub: None,
            eventbridge: None,
            kafka: None,
            azure_servicebus: None,
            gcp_pubsub_v2: None,
            rabbitmq: None,
            nats: None,
            mqtt: None,
            redis: None,
            amqp: None,
            stomp: None,
            zeromq: None,
            grpc: None,
            sqs: None,
            mattermost: None,
            ntfy: None,
            pagerduty: None,
            discord_webhook: None,
            teams_webhook: None,
            slack_blocks: None,
            custom_template: None,
            custom_webhook: None,
            custom_headers: None,
            custom_json: None,
            custom_filter: None,
            custom_retry: None,
            custom_transform: None,
            custom_batch: None,
            custom_deduplicate: None,
            custom_throttle: None,
            custom_aggregate: None,
            custom_priority: None,
            custom_routing: None,
            custom_dedup_window: None,
            custom_rate_limit: None,
            custom_backoff: None,
            custom_circuit_breaker: None,
            custom_dead_letter: None,
            custom_escalation: None,
            custom_correlation: None,
            custom_sampling: None,
            custom_digest: None,
            custom_severity_filter: None,
        }
    }

    fn make_local_machine() -> types::Machine {
        types::Machine {
            hostname: "localhost".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        }
    }

    fn make_ssh_machine() -> types::Machine {
        types::Machine {
            hostname: "remote".to_string(),
            addr: "10.99.99.99".to_string(),
            user: "deploy".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        }
    }

    // ===================================================================
    // core/parser/resource_types.rs — validate_model
    // ===================================================================

    #[test]
    fn test_validate_model_no_name_error() {
        let yaml = r#"
version: "1.0"
name: model-test
machines:
  ml:
    hostname: ml
    addr: 127.0.0.1
resources:
  m1:
    type: model
    machine: ml
"#;
        let config = parser::parse_config(yaml).unwrap();
        let errors = parser::validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("model") && e.message.contains("no name")),
            "expected model no-name error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_model_valid_state_present() {
        let yaml = r#"
version: "1.0"
name: model-test
machines:
  ml:
    hostname: ml
    addr: 127.0.0.1
resources:
  m1:
    type: model
    machine: ml
    name: llama-3
    state: present
"#;
        let config = parser::parse_config(yaml).unwrap();
        let errors = parser::validate_config(&config);
        let model_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("model") && e.message.contains("invalid state"))
            .collect();
        assert!(model_errors.is_empty(), "unexpected: {model_errors:?}");
    }

    #[test]
    fn test_validate_model_invalid_state() {
        let yaml = r#"
version: "1.0"
name: model-test
machines:
  ml:
    hostname: ml
    addr: 127.0.0.1
resources:
  m1:
    type: model
    machine: ml
    name: llama-3
    state: running
"#;
        let config = parser::parse_config(yaml).unwrap();
        let errors = parser::validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("model") && e.message.contains("invalid state")),
            "expected invalid state error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_model_valid_state_absent() {
        let yaml = r#"
version: "1.0"
name: model-test
machines:
  ml:
    hostname: ml
    addr: 127.0.0.1
resources:
  m1:
    type: model
    machine: ml
    name: llama-3
    state: absent
"#;
        let config = parser::parse_config(yaml).unwrap();
        let errors = parser::validate_config(&config);
        let model_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("model") && e.message.contains("invalid state"))
            .collect();
        assert!(model_errors.is_empty(), "unexpected: {model_errors:?}");
    }

    // ===================================================================
    // transport/mod.rs — exec_script_retry
    // ===================================================================

    #[test]
    fn test_exec_script_retry_local_no_retry() {
        let machine = make_local_machine();
        let result = transport::exec_script_retry(&machine, "echo hello", None, 3);
        assert!(result.is_ok());
        let out = result.unwrap();
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("hello"));
    }

    #[test]
    fn test_exec_script_retry_local_with_timeout() {
        let machine = make_local_machine();
        let result = transport::exec_script_retry(&machine, "echo timed", Some(5), 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_exec_script_retry_local_ignores_retries() {
        // Local transport should NOT retry even if ssh_retries > 1
        let machine = make_local_machine();
        let result = transport::exec_script_retry(&machine, "exit 1", None, 3);
        assert!(result.is_ok());
        let out = result.unwrap();
        assert_ne!(out.exit_code, 0);
    }

    #[test]
    fn test_exec_script_retry_ssh_fails_immediately_non_transient() {
        // SSH to an unreachable host — non-transient error, should not retry
        let machine = make_ssh_machine();
        let result = transport::exec_script_retry(&machine, "echo hi", Some(2), 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_exec_script_retry_clamps_max_retries() {
        // retries clamped to max 4
        let machine = make_local_machine();
        let result = transport::exec_script_retry(&machine, "echo clamped", None, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_exec_script_retry_zero_retries_means_one_attempt() {
        let machine = make_local_machine();
        let result = transport::exec_script_retry(&machine, "echo zero", None, 0);
        assert!(result.is_ok());
    }

    // ===================================================================
    // drift.rs — cmd_drift_dry_run
    // ===================================================================

    #[test]
    fn test_cmd_drift_dry_run_empty_state_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = cmd_drift_dry_run(tmp.path(), None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_drift_dry_run_empty_state_dir_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = cmd_drift_dry_run(tmp.path(), None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_drift_dry_run_nonexistent_dir_errors() {
        let result = cmd_drift_dry_run(Path::new("/nonexistent/drift/dir/cov"), None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_drift_dry_run_with_lock_data() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_dir = tmp.path();

        make_state_dir_with_lock(
            state_dir,
            "drift-host",
            vec![
                ("res1", "blake3:aaa111", types::ResourceStatus::Converged),
                ("res2", "blake3:bbb222", types::ResourceStatus::Drifted),
            ],
        );

        let result = cmd_drift_dry_run(state_dir, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_drift_dry_run_with_lock_data_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_dir = tmp.path();

        make_state_dir_with_lock(
            state_dir,
            "drift-host-json",
            vec![
                ("svc1", "blake3:ccc333", types::ResourceStatus::Converged),
                ("svc2", "blake3:ddd444", types::ResourceStatus::Failed),
                ("svc3", "blake3:eee555", types::ResourceStatus::Unknown),
            ],
        );

        let result = cmd_drift_dry_run(state_dir, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_drift_dry_run_with_machine_filter() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_dir = tmp.path();

        make_state_dir_with_lock(
            state_dir,
            "host-a",
            vec![("r1", "blake3:111", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            state_dir,
            "host-b",
            vec![("r2", "blake3:222", types::ResourceStatus::Converged)],
        );

        // Filter to only host-a
        let result = cmd_drift_dry_run(state_dir, Some("host-a"), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_drift_dry_run_filter_no_match() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_dir = tmp.path();

        make_state_dir_with_lock(
            state_dir,
            "host-x",
            vec![("r1", "blake3:xxx", types::ResourceStatus::Converged)],
        );

        let result = cmd_drift_dry_run(state_dir, Some("nonexistent"), false);
        assert!(result.is_ok());
    }

    // ===================================================================
    // Additional notify coverage — send multiple channels at once
    // ===================================================================

    #[test]
    fn test_send_apply_notifications_custom_filter_success() {
        let mut opts = empty_notify_opts();
        opts.custom_filter = Some("http://127.0.0.1:1/filter|status==success");
        let result: Result<(), String> = Ok(());
        send_apply_notifications(&opts, &result, Path::new("/tmp/filter.yaml"));
    }

    #[test]
    fn test_send_apply_notifications_custom_json_success() {
        let mut opts = empty_notify_opts();
        opts.custom_json = Some(r#"http://127.0.0.1:1/json|{"s":"{{status}}","c":"{{config}}"}"#);
        let result: Result<(), String> = Ok(());
        send_apply_notifications(&opts, &result, Path::new("/tmp/json.yaml"));
    }

    #[test]
    fn test_send_apply_notifications_custom_template() {
        let mut opts = empty_notify_opts();
        opts.custom_template = Some("echo '{{status}} {{config}}'");
        let result: Result<(), String> = Err("error".to_string());
        send_apply_notifications(&opts, &result, Path::new("/tmp/tmpl.yaml"));
    }

    #[test]
    fn test_send_apply_notifications_custom_webhook_failure() {
        let mut opts = empty_notify_opts();
        opts.custom_webhook = Some("http://127.0.0.1:1/webhook");
        let result: Result<(), String> = Err("failed".to_string());
        send_apply_notifications(&opts, &result, Path::new("/tmp/webhook.yaml"));
    }

    // ===================================================================
    // Additional transport coverage
    // ===================================================================

    #[test]
    fn test_exec_script_retry_pepita_machine_no_pidfile() {
        let machine = types::Machine {
            hostname: "pepita-retry".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
            pepita: Some(types::PepitaTransportConfig {
                rootfs: "debootstrap:jammy".to_string(),
                memory_mb: None,
                cpus: None,
                network: "host".to_string(),
                filesystem: "bind".to_string(),
                ephemeral: false,
            }),
            cost: 0,
            allowed_operators: vec![],
        };
        // Pepita transport — should NOT retry, just fail once
        let result = transport::exec_script_retry(&machine, "echo hi", None, 3);
        assert!(result.is_err());
    }

    // ===================================================================
    // Additional convergence coverage
    // ===================================================================

    #[test]
    fn test_cmd_status_since_json_with_results() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_dir = tmp.path();

        make_state_dir_with_lock(
            state_dir,
            "json-host",
            vec![
                ("pkg-a", "blake3:aa11", types::ResourceStatus::Converged),
                ("pkg-b", "blake3:bb22", types::ResourceStatus::Failed),
            ],
        );

        let result = cmd_status_since(state_dir, None, "48h", true);
        assert!(result.is_ok());
    }

    // ===================================================================
    // Additional doctor coverage — network check
    // ===================================================================

    #[test]
    fn test_cmd_doctor_network_valid_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("forjar.yaml");
        std::fs::write(&config_path, minimal_config_yaml()).unwrap();
        let result = cmd_doctor_network(Some(&config_path), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_doctor_network_json_output() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("forjar.yaml");
        std::fs::write(&config_path, minimal_config_yaml()).unwrap();
        let result = cmd_doctor_network(Some(&config_path), true);
        assert!(result.is_ok());
    }
}
