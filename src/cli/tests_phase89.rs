//! Tests: Phase 89 — Dependency Visualization & Fleet Health Scoring (FJ-973→FJ-980).

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

    // ── FJ-973: validate --check-resource-env-consistency ──

    #[test]
    fn test_fj973_env_consistency_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_env_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj973_env_consistency_with_params() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nparams:\n  port:\n    default: \"8080\"\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: \"port={{port}}\"\n");
        assert!(cmd_validate_check_resource_env_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj973_env_consistency_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: \"val={{missing_var}}\"\n");
        assert!(cmd_validate_check_resource_env_consistency(f.path(), true).is_ok());
    }

    // ── FJ-974: status --machine-resource-apply-frequency ──

    #[test]
    fn test_fj974_apply_frequency_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_frequency(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj974_apply_frequency_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: converged\n    hash: \"blake3:def\"\n");
        assert!(cmd_status_machine_resource_apply_frequency(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj974_apply_frequency_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_frequency(dir.path(), None, true).is_ok());
    }

    // ── FJ-975: graph --resource-dependency-minimum-cut ──

    #[test]
    fn test_fj975_minimum_cut_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_minimum_cut(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj975_minimum_cut_with_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_minimum_cut(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj975_minimum_cut_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_minimum_cut(f.path(), true).is_ok());
    }

    // ── FJ-976: apply --notify-custom-dead-letter ──

    #[test]
    fn test_fj976_custom_dead_letter_notification() {
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
            custom_dead_letter: Some("https://hooks.example.com|my-dlq"),
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
    fn test_fj976_custom_dead_letter_none() {
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

    // ── FJ-977: validate --check-resource-secret-rotation ──

    #[test]
    fn test_fj977_secret_rotation_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_secret_rotation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj977_secret_rotation_with_secret() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  db_password:\n    type: file\n    machine: m\n    path: /tmp/secret\n    content: s3cret\n");
        assert!(cmd_validate_check_resource_secret_rotation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj977_secret_rotation_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  api_key:\n    type: file\n    machine: m\n    path: /tmp/key\n    content: mykey\n");
        assert!(cmd_validate_check_resource_secret_rotation(f.path(), true).is_ok());
    }

    // ── FJ-978: status --fleet-resource-health-score ──

    #[test]
    fn test_fj978_fleet_health_score_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_health_score(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj978_fleet_health_score_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: drifted\n    hash: \"blake3:def\"\n  h:\n    type: file\n    status: failed\n    hash: \"\"\n");
        assert!(cmd_status_fleet_resource_health_score(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj978_fleet_health_score_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_health_score(dir.path(), None, true).is_ok());
    }

    // ── FJ-979: graph --resource-dependency-dominator-tree ──

    #[test]
    fn test_fj979_dominator_tree_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_dominator_tree(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj979_dominator_tree_with_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_dominator_tree(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj979_dominator_tree_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_dominator_tree(f.path(), true).is_ok());
    }

    // ── FJ-980: status --machine-resource-staleness-index ──

    #[test]
    fn test_fj980_staleness_index_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_staleness_index(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj980_staleness_index_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n    duration_seconds: 7200.0\n  g:\n    type: file\n    status: converged\n    hash: \"blake3:def\"\n    duration_seconds: 300.0\n");
        assert!(cmd_status_machine_resource_staleness_index(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj980_staleness_index_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_staleness_index(dir.path(), None, true).is_ok());
    }
}
