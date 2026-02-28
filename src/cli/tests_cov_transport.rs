//! Tests: Coverage for transport, notify (part 1 of 3).
//! Covers: exec_pepita, ensure_namespace, cleanup_namespace, send_apply_notifications.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::core::{parser, state, types};
use crate::transport;
use super::dispatch_notify::*;
use super::doctor::*;
use super::drift::*;
use super::observe::*;
use super::status_convergence::*;
use super::test_fixtures::*;
use super::helpers::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn write_yaml(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
        p
    }

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
        }
    }

    // ===================================================================
    // transport/pepita.rs — exec_pepita
    // ===================================================================

    #[test]
    fn test_exec_pepita_no_config_returns_error() {
        let machine = types::Machine {
            hostname: "test-ns".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
            pepita: None,
            cost: 0,
        };
        let result = transport::pepita::exec_pepita(&machine, "echo ok");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no pepita config"));
    }

    #[test]
    fn test_exec_pepita_missing_pidfile_error() {
        let machine = types::Machine {
            hostname: "cov-nonexistent-ns".to_string(),
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
        };
        let result = transport::pepita::exec_pepita(&machine, "echo hi");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("pidfile") || err.contains("cannot read"));
    }

    #[test]
    fn test_exec_pepita_isolated_network_missing_pidfile() {
        let machine = types::Machine {
            hostname: "cov-isolated-ns".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
            pepita: Some(types::PepitaTransportConfig {
                rootfs: "debootstrap:jammy".to_string(),
                memory_mb: Some(512),
                cpus: Some(2.0),
                network: "isolated".to_string(),
                filesystem: "overlay".to_string(),
                ephemeral: true,
            }),
            cost: 0,
        };
        let result = transport::pepita::exec_pepita(&machine, "echo hi");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("pidfile") || err.contains("cannot read"),
            "expected pidfile error, got: {}",
            err
        );
    }

    #[test]
    fn test_exec_pepita_empty_script() {
        let machine = types::Machine {
            hostname: "cov-empty-script".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
            pepita: Some(types::PepitaTransportConfig {
                rootfs: "noop".to_string(),
                memory_mb: None,
                cpus: None,
                network: "host".to_string(),
                filesystem: "bind".to_string(),
                ephemeral: false,
            }),
            cost: 0,
        };
        // Will fail reading the pidfile, but exercises the config extraction path
        let result = transport::pepita::exec_pepita(&machine, "");
        assert!(result.is_err());
    }

    // ===================================================================
    // transport/pepita.rs — ensure_namespace (error paths)
    // ===================================================================

    #[test]
    fn test_ensure_namespace_no_config_returns_error() {
        let machine = types::Machine {
            hostname: "test-ns-no-cfg".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
            pepita: None,
            cost: 0,
        };
        let result = transport::pepita::ensure_namespace(&machine);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no pepita config"));
    }

    #[test]
    fn test_ensure_namespace_host_network() {
        // This will either succeed (if root) or fail with permission error
        let machine = types::Machine {
            hostname: "cov-ensure-host".to_string(),
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
        };
        // In CI/non-root, this will fail at creating /run/forjar or unshare
        let result = transport::pepita::ensure_namespace(&machine);
        // We just exercise the code path; it's OK if it errors in non-root env
        let _ = result;
    }

    #[test]
    fn test_ensure_namespace_isolated_with_limits() {
        let machine = types::Machine {
            hostname: "cov-ensure-limits".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
            pepita: Some(types::PepitaTransportConfig {
                rootfs: "debootstrap:jammy".to_string(),
                memory_mb: Some(256),
                cpus: Some(1.5),
                network: "isolated".to_string(),
                filesystem: "overlay".to_string(),
                ephemeral: true,
            }),
            cost: 0,
        };
        let result = transport::pepita::ensure_namespace(&machine);
        // exercise code path — may fail without root
        let _ = result;
    }

    #[test]
    fn test_ensure_namespace_no_memory_no_cpus() {
        let machine = types::Machine {
            hostname: "cov-ensure-nolimits".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
            pepita: Some(types::PepitaTransportConfig {
                rootfs: "noop".to_string(),
                memory_mb: None,
                cpus: None,
                network: "host".to_string(),
                filesystem: "bind".to_string(),
                ephemeral: false,
            }),
            cost: 0,
        };
        let result = transport::pepita::ensure_namespace(&machine);
        let _ = result;
    }

    // ===================================================================
    // transport/pepita.rs — cleanup_namespace
    // ===================================================================

    #[test]
    fn test_cleanup_namespace_no_config() {
        let machine = types::Machine {
            hostname: "cleanup-no-cfg".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
            pepita: None,
            cost: 0,
        };
        let result = transport::pepita::cleanup_namespace(&machine);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no pepita config"));
    }

    #[test]
    fn test_cleanup_namespace_nonexistent_succeeds() {
        let machine = types::Machine {
            hostname: "cov-cleanup-nonexist".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
            pepita: Some(types::PepitaTransportConfig {
                rootfs: "noop".to_string(),
                memory_mb: None,
                cpus: None,
                network: "host".to_string(),
                filesystem: "bind".to_string(),
                ephemeral: false,
            }),
            cost: 0,
        };
        let result = transport::pepita::cleanup_namespace(&machine);
        assert!(result.is_ok());
    }

    // ===================================================================
    // dispatch_notify.rs — send_custom_retry_notification via send_apply_notifications
    // ===================================================================

    #[test]
    fn test_send_apply_notifications_all_none_noop() {
        let opts = empty_notify_opts();
        let result: Result<(), String> = Ok(());
        send_apply_notifications(&opts, &result, Path::new("/tmp/forjar.yaml"));
        // No panic = success; all None means no notifications sent
    }

    #[test]
    fn test_send_apply_notifications_custom_retry_success() {
        let mut opts = empty_notify_opts();
        // Set custom_retry with an unreachable URL — the retry code path runs
        opts.custom_retry = Some("http://127.0.0.1:1/noop|retries:1");
        let result: Result<(), String> = Ok(());
        send_apply_notifications(&opts, &result, Path::new("/tmp/test.yaml"));
    }

    #[test]
    fn test_send_apply_notifications_custom_retry_failure() {
        let mut opts = empty_notify_opts();
        opts.custom_retry = Some("http://127.0.0.1:1/noop|retries:2");
        let result: Result<(), String> = Err("apply failed".to_string());
        send_apply_notifications(&opts, &result, Path::new("/tmp/fail.yaml"));
    }

    #[test]
    fn test_send_apply_notifications_custom_retry_bad_retries() {
        let mut opts = empty_notify_opts();
        // Missing retries value — defaults to 3
        opts.custom_retry = Some("http://127.0.0.1:1/noop|retries:abc");
        let result: Result<(), String> = Ok(());
        send_apply_notifications(&opts, &result, Path::new("/tmp/bad.yaml"));
    }

    #[test]
    fn test_send_apply_notifications_custom_retry_no_pipe() {
        let mut opts = empty_notify_opts();
        // No pipe separator — parts.len() != 2, early return
        opts.custom_retry = Some("http://127.0.0.1:1/noop");
        let result: Result<(), String> = Ok(());
        send_apply_notifications(&opts, &result, Path::new("/tmp/nopipe.yaml"));
    }

    // ===================================================================
    // dispatch_notify.rs — send_custom_headers_notification via send_apply_notifications
    // ===================================================================

    #[test]
    fn test_send_apply_notifications_custom_headers_success() {
        let mut opts = empty_notify_opts();
        opts.custom_headers = Some("http://127.0.0.1:1/noop|X-Token:abc|X-Source:forjar");
        let result: Result<(), String> = Ok(());
        send_apply_notifications(&opts, &result, Path::new("/tmp/headers.yaml"));
    }

    #[test]
    fn test_send_apply_notifications_custom_headers_failure_result() {
        let mut opts = empty_notify_opts();
        opts.custom_headers = Some("http://127.0.0.1:1/noop|Authorization:Bearer tok123");
        let result: Result<(), String> = Err("deploy failed".to_string());
        send_apply_notifications(&opts, &result, Path::new("/tmp/hdr-fail.yaml"));
    }

    #[test]
    fn test_send_apply_notifications_custom_headers_no_extra_headers() {
        let mut opts = empty_notify_opts();
        // URL only, no extra headers after the pipe
        opts.custom_headers = Some("http://127.0.0.1:1/endpoint");
        let result: Result<(), String> = Ok(());
        send_apply_notifications(&opts, &result, Path::new("/tmp/no-hdr.yaml"));
    }

    #[test]
    fn test_send_apply_notifications_custom_headers_empty() {
        let mut opts = empty_notify_opts();
        opts.custom_headers = Some("");
        let result: Result<(), String> = Ok(());
        send_apply_notifications(&opts, &result, Path::new("/tmp/empty-hdr.yaml"));
    }
}
