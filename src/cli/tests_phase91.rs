//! Tests: Phase 91 — Advanced Governance & Operational Depth (FJ-989→FJ-996).

use super::graph_intelligence_ext2::*;
use super::status_intelligence_ext2::*;
use super::validate_ordering_ext::*;
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

    // ── FJ-989: validate --check-resource-naming-convention-strict ──

    #[test]
    fn test_fj989_naming_convention_strict_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_naming_convention_strict(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj989_naming_convention_strict_valid() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  my_resource:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_naming_convention_strict(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj989_naming_convention_strict_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  my-resource:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_naming_convention_strict(f.path(), true).is_ok());
    }

    // ── FJ-990: status --machine-resource-drift-age-hours ──

    #[test]
    fn test_fj990_drift_age_hours_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_age_hours(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj990_drift_age_hours_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n    duration_seconds: 7200.0\n");
        assert!(cmd_status_machine_resource_drift_age_hours(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj990_drift_age_hours_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_age_hours(dir.path(), None, true).is_ok());
    }

    // ── FJ-991: graph --resource-dependency-betweenness-centrality ──

    #[test]
    fn test_fj991_betweenness_centrality_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_betweenness_centrality(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj991_betweenness_centrality_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a, b]\n");
        assert!(cmd_graph_resource_dependency_betweenness_centrality(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj991_betweenness_centrality_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_betweenness_centrality(f.path(), true).is_ok());
    }

    // ── FJ-992: apply --notify-custom-correlation ──

    #[test]
    fn test_fj992_custom_correlation_notification() {
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
            custom_correlation: Some("https://hooks.example.com|30s"),
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
    fn test_fj992_custom_correlation_none() {
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

    // ── FJ-993: validate --check-resource-idempotency-annotations ──

    #[test]
    fn test_fj993_idempotency_annotations_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_idempotency_annotations(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj993_idempotency_annotations_with_resources() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_idempotency_annotations(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj993_idempotency_annotations_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_idempotency_annotations(f.path(), true).is_ok());
    }

    // ── FJ-994: status --fleet-resource-convergence-percentile ──

    #[test]
    fn test_fj994_convergence_percentile_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_percentile(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj994_convergence_percentile_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: drifted\n    hash: \"blake3:def\"\n");
        write_yaml(dir.path(), "db/state.lock.yaml", "schema: \"1.0\"\nmachine: db\nhostname: db\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  h:\n    type: file\n    status: converged\n    hash: \"blake3:xyz\"\n");
        assert!(cmd_status_fleet_resource_convergence_percentile(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj994_convergence_percentile_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_percentile(dir.path(), None, true).is_ok());
    }

    // ── FJ-995: graph --resource-dependency-closure-size ──

    #[test]
    fn test_fj995_closure_size_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_closure_size(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj995_closure_size_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a, b]\n");
        assert!(cmd_graph_resource_dependency_closure_size(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj995_closure_size_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_closure_size(f.path(), true).is_ok());
    }

    // ── FJ-996: status --machine-resource-error-rate ──

    #[test]
    fn test_fj996_error_rate_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj996_error_rate_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: failed\n    hash: \"\"\n  h:\n    type: file\n    status: converged\n    hash: \"blake3:def\"\n");
        assert!(cmd_status_machine_resource_error_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj996_error_rate_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_rate(dir.path(), None, true).is_ok());
    }
}
