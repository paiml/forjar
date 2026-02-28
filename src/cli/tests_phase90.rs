//! Tests: Phase 90 — Resource Lifecycle & Dependency Resilience (FJ-981→FJ-988).

use super::validate_ordering::*;
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

    // ── FJ-981: validate --check-resource-lifecycle-completeness ──

    #[test]
    fn test_fj981_lifecycle_completeness_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_lifecycle_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj981_lifecycle_completeness_with_content() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_lifecycle_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj981_lifecycle_completeness_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_lifecycle_completeness(f.path(), true).is_ok());
    }

    // ── FJ-982: status --machine-resource-drift-recurrence ──

    #[test]
    fn test_fj982_drift_recurrence_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_recurrence(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj982_drift_recurrence_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_machine_resource_drift_recurrence(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj982_drift_recurrence_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_recurrence(dir.path(), None, true).is_ok());
    }

    // ── FJ-983: graph --resource-dependency-resilience-score ──

    #[test]
    fn test_fj983_resilience_score_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_resilience_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj983_resilience_score_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_resilience_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj983_resilience_score_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_resilience_score(f.path(), true).is_ok());
    }

    // ── FJ-984: apply --notify-custom-escalation ──

    #[test]
    fn test_fj984_custom_escalation_notification() {
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
            custom_circuit_breaker: None, custom_dead_letter: None,
            custom_escalation: Some("https://hooks.example.com|warning"),
            custom_correlation: None, custom_sampling: None,
        };
        let result: Result<(), String> = Ok(());
        super::super::dispatch_notify::send_apply_notifications(&opts, &result, std::path::Path::new("test.yaml"));
    }

    #[test]
    fn test_fj984_custom_escalation_none() {
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
            custom_circuit_breaker: None, custom_dead_letter: None, custom_escalation: None, custom_correlation: None, custom_sampling: None,
        };
        let result: Result<(), String> = Ok(());
        super::super::dispatch_notify::send_apply_notifications(&opts, &result, std::path::Path::new("test.yaml"));
    }

    // ── FJ-985: validate --check-resource-provider-compatibility ──

    #[test]
    fn test_fj985_provider_compatibility_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_provider_compatibility(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj985_provider_compatibility_valid() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_provider_compatibility(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj985_provider_compatibility_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_provider_compatibility(f.path(), true).is_ok());
    }

    // ── FJ-986: status --fleet-resource-drift-heatmap ──

    #[test]
    fn test_fj986_drift_heatmap_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_heatmap(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj986_drift_heatmap_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: converged\n    hash: \"blake3:def\"\n");
        write_yaml(dir.path(), "db/state.lock.yaml", "schema: \"1.0\"\nmachine: db\nhostname: db\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  h:\n    type: file\n    status: converged\n    hash: \"blake3:xyz\"\n");
        assert!(cmd_status_fleet_resource_drift_heatmap(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj986_drift_heatmap_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_heatmap(dir.path(), None, true).is_ok());
    }

    // ── FJ-987: graph --resource-dependency-pagerank ──

    #[test]
    fn test_fj987_pagerank_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_pagerank(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj987_pagerank_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a, b]\n");
        assert!(cmd_graph_resource_dependency_pagerank(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj987_pagerank_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_pagerank(f.path(), true).is_ok());
    }

    // ── FJ-988: status --machine-resource-convergence-trend-p90 ──

    #[test]
    fn test_fj988_convergence_trend_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_trend_p90(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj988_convergence_trend_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: converged\n    hash: \"blake3:def\"\n  h:\n    type: file\n    status: drifted\n    hash: \"blake3:ghi\"\n");
        assert!(cmd_status_machine_resource_convergence_trend_p90(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj988_convergence_trend_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_trend_p90(dir.path(), None, true).is_ok());
    }
}
