//! Tests: Phase 92 — Fleet Observability & Dependency Topology (FJ-997→FJ-1004).

use super::validate_ordering_ext::*;
use super::graph_intelligence_ext2::*;
use super::status_intelligence_ext2::*;
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
        if let Some(parent) = p.parent() { std::fs::create_dir_all(parent).unwrap(); }
        std::fs::write(&p, content).unwrap();
        p
    }

    // ── FJ-997: validate --check-resource-content-size-limit ──

    #[test]
    fn test_fj997_content_size_limit_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_content_size_limit(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj997_content_size_limit_small() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_content_size_limit(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj997_content_size_limit_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_content_size_limit(f.path(), true).is_ok());
    }

    // ── FJ-998: status --machine-resource-convergence-gap ──

    #[test]
    fn test_fj998_convergence_gap_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_gap(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj998_convergence_gap_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: drifted\n    hash: \"blake3:def\"\n");
        assert!(cmd_status_machine_resource_convergence_gap(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj998_convergence_gap_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_gap(dir.path(), None, true).is_ok());
    }

    // ── FJ-999: graph --resource-dependency-eccentricity-map ──

    #[test]
    fn test_fj999_eccentricity_map_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_eccentricity_map(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj999_eccentricity_map_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_eccentricity_map(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj999_eccentricity_map_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_eccentricity_map(f.path(), true).is_ok());
    }

    // ── FJ-1000: apply --notify-custom-sampling ──

    #[test]
    fn test_fj1000_custom_sampling_notification() {
        let opts = super::super::dispatch_notify::NotifyOpts {
            slack: None, email: None, webhook: None, teams: None, discord: None,
            opsgenie: None, datadog: None, newrelic: None, grafana: None, victorops: None,
            msteams_adaptive: None, incident: None, sns: None, pubsub: None, eventbridge: None,
            kafka: None, azure_servicebus: None, gcp_pubsub_v2: None, rabbitmq: None, nats: None,
            mqtt: None, redis: None, amqp: None, stomp: None, zeromq: None, grpc: None, sqs: None,
            mattermost: None, ntfy: None, pagerduty: None,
            discord_webhook: None, teams_webhook: None, slack_blocks: None, custom_template: None,
            custom_webhook: None, custom_headers: None, custom_json: None, custom_filter: None,
            custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None,
            custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None,
            custom_dedup_window: None, custom_rate_limit: None, custom_backoff: None,
            custom_circuit_breaker: None, custom_dead_letter: None, custom_escalation: None,
            custom_correlation: None,
            custom_sampling: Some("https://hooks.example.com|10"),
            custom_digest: None, custom_severity_filter: None,
        };
        let result: Result<(), String> = Ok(());
        super::super::dispatch_notify::send_apply_notifications(&opts, &result, std::path::Path::new("test.yaml"));
    }

    #[test]
    fn test_fj1000_custom_sampling_none() {
        let opts = super::super::dispatch_notify::NotifyOpts {
            slack: None, email: None, webhook: None, teams: None, discord: None,
            opsgenie: None, datadog: None, newrelic: None, grafana: None, victorops: None,
            msteams_adaptive: None, incident: None, sns: None, pubsub: None, eventbridge: None,
            kafka: None, azure_servicebus: None, gcp_pubsub_v2: None, rabbitmq: None, nats: None,
            mqtt: None, redis: None, amqp: None, stomp: None, zeromq: None, grpc: None, sqs: None,
            mattermost: None, ntfy: None, pagerduty: None,
            discord_webhook: None, teams_webhook: None, slack_blocks: None, custom_template: None,
            custom_webhook: None, custom_headers: None, custom_json: None, custom_filter: None,
            custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None,
            custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None,
            custom_dedup_window: None, custom_rate_limit: None, custom_backoff: None,
            custom_circuit_breaker: None, custom_dead_letter: None, custom_escalation: None,
            custom_correlation: None, custom_sampling: None,
            custom_digest: None, custom_severity_filter: None,
        };
        let result: Result<(), String> = Ok(());
        super::super::dispatch_notify::send_apply_notifications(&opts, &result, std::path::Path::new("test.yaml"));
    }

    // ── FJ-1001: validate --check-resource-dependency-fan-limit ──

    #[test]
    fn test_fj1001_fan_limit_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_fan_limit(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1001_fan_limit_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_dependency_fan_limit(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1001_fan_limit_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_validate_check_resource_dependency_fan_limit(f.path(), true).is_ok());
    }

    // ── FJ-1002: status --fleet-resource-error-distribution ──

    #[test]
    fn test_fj1002_error_distribution_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_error_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1002_error_distribution_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: failed\n    hash: \"\"\n");
        write_yaml(dir.path(), "db/state.lock.yaml", "schema: \"1.0\"\nmachine: db\nhostname: db\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  h:\n    type: file\n    status: converged\n    hash: \"blake3:xyz\"\n");
        assert!(cmd_status_fleet_resource_error_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1002_error_distribution_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_error_distribution(dir.path(), None, true).is_ok());
    }

    // ── FJ-1003: graph --resource-dependency-diameter-path ──

    #[test]
    fn test_fj1003_diameter_path_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_diameter_path(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1003_diameter_path_with_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_diameter_path(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1003_diameter_path_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_diameter_path(f.path(), true).is_ok());
    }

    // ── FJ-1004: status --machine-resource-convergence-stability ──

    #[test]
    fn test_fj1004_convergence_stability_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_stability(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1004_convergence_stability_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: converged\n    hash: \"blake3:def\"\n");
        write_yaml(dir.path(), "db/state.lock.yaml", "schema: \"1.0\"\nmachine: db\nhostname: db\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  h:\n    type: file\n    status: converged\n    hash: \"blake3:xyz\"\n  i:\n    type: file\n    status: drifted\n    hash: \"blake3:uvw\"\n");
        assert!(cmd_status_machine_resource_convergence_stability(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1004_convergence_stability_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_stability(dir.path(), None, true).is_ok());
    }
}
