//! Tests: Phase 87 — Configuration Drift Analytics & Dependency Health (FJ-957→FJ-964).

use super::graph_intelligence_ext::*;
use super::status_intelligence_ext::*;
use super::validate_ordering::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn write_yaml(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
        p
    }

    // ── FJ-957: validate --check-resource-content-hash-consistency ──

    #[test]
    fn test_fj957_content_hash_consistency_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_content_hash_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj957_content_hash_consistency_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_content_hash_consistency(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj957_content_hash_consistency_with_resource() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n    checksum: invalid-hash\n");
        assert!(cmd_validate_check_resource_content_hash_consistency(f.path(), false).is_ok());
    }

    // ── FJ-958: status --machine-resource-drift-age ──

    #[test]
    fn test_fj958_drift_age_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_age(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj958_drift_age_with_drifted() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n    duration_seconds: 7200.0\n");
        assert!(cmd_status_machine_resource_drift_age(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj958_drift_age_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_age(dir.path(), None, true).is_ok());
    }

    // ── FJ-959: graph --resource-dependency-longest-path ──

    #[test]
    fn test_fj959_longest_path_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_longest_path(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj959_longest_path_with_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_longest_path(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj959_longest_path_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_longest_path(f.path(), true).is_ok());
    }

    // ── FJ-960: apply --notify-custom-backoff (integration via NotifyOpts) ──

    #[test]
    fn test_fj960_custom_backoff_notification() {
        let opts = super::super::dispatch_notify::NotifyOpts {
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
            custom_backoff: Some("https://hooks.example.com|exponential"),
            custom_circuit_breaker: None,
            custom_dead_letter: None,
            custom_escalation: None,
            custom_correlation: None,
            custom_sampling: None,
            custom_digest: None,
            custom_severity_filter: None,
        };
        let result: Result<(), String> = Ok(());
        super::super::dispatch_notify::send_apply_notifications(
            &opts,
            &result,
            std::path::Path::new("test.yaml"),
        );
    }

    #[test]
    fn test_fj960_custom_backoff_none() {
        let opts = super::super::dispatch_notify::NotifyOpts {
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
        };
        let result: Result<(), String> = Ok(());
        super::super::dispatch_notify::send_apply_notifications(
            &opts,
            &result,
            std::path::Path::new("test.yaml"),
        );
    }

    // ── FJ-961: validate --check-resource-dependency-refs ──

    #[test]
    fn test_fj961_dependency_refs_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj961_dependency_refs_valid() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_dependency_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj961_dependency_refs_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    depends_on: [nonexistent]\n");
        assert!(cmd_validate_check_resource_dependency_refs(f.path(), true).is_ok());
    }

    // ── FJ-962: status --fleet-resource-drift-age ──

    #[test]
    fn test_fj962_fleet_drift_age_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_age(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj962_fleet_drift_age_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_age(dir.path(), None, true).is_ok());
    }

    // ── FJ-963: graph --resource-dependency-strongly-connected ──

    #[test]
    fn test_fj963_strongly_connected_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_strongly_connected(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj963_strongly_connected_acyclic() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_strongly_connected(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj963_strongly_connected_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_strongly_connected(f.path(), true).is_ok());
    }

    // ── FJ-964: status --machine-resource-recovery-rate ──

    #[test]
    fn test_fj964_recovery_rate_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_recovery_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj964_recovery_rate_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: failed\n    hash: \"\"\n");
        assert!(cmd_status_machine_resource_recovery_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj964_recovery_rate_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_recovery_rate(dir.path(), None, true).is_ok());
    }
}
