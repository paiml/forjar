//! Tests: Phase 94 — Resource Profiling & Security Posture (FJ-1013→FJ-1020).

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

    // ── FJ-1013: status --machine-resource-apply-latency-p95 ──

    #[test]
    fn test_fj1013_apply_latency_p95_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_latency_p95(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1013_apply_latency_p95_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: converged\n    hash: \"blake3:def\"\n");
        assert!(cmd_status_machine_resource_apply_latency_p95(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1013_apply_latency_p95_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_latency_p95(dir.path(), None, true).is_ok());
    }

    // ── FJ-1014: validate --check-resource-gpu-backend-consistency ──

    #[test]
    fn test_fj1014_gpu_backend_consistency_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_gpu_backend_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1014_gpu_backend_consistency_single() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  gpu1:\n    type: gpu\n    machine: m\n    gpu_backend: nvidia\n");
        assert!(cmd_validate_check_resource_gpu_backend_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1014_gpu_backend_consistency_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  gpu1:\n    type: gpu\n    machine: m\n    gpu_backend: nvidia\n");
        assert!(cmd_validate_check_resource_gpu_backend_consistency(f.path(), true).is_ok());
    }

    // ── FJ-1015: graph --resource-dependency-bridge-criticality ──

    #[test]
    fn test_fj1015_bridge_criticality_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_bridge_criticality(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1015_bridge_criticality_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_bridge_criticality(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1015_bridge_criticality_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_bridge_criticality(f.path(), true).is_ok());
    }

    // ── FJ-1016: apply --notify-custom-digest ──

    #[test]
    fn test_fj1016_custom_digest_notification() {
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
            custom_digest: Some("https://hooks.example.com|1h"),
            custom_severity_filter: None,
        };
        let result: Result<(), String> = Ok(());
        super::super::dispatch_notify::send_apply_notifications(&opts, &result, std::path::Path::new("test.yaml"));
    }

    #[test]
    fn test_fj1016_custom_digest_none() {
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

    // ── FJ-1017: status --fleet-resource-security-posture-score ──

    #[test]
    fn test_fj1017_security_posture_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_security_posture_score(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1017_security_posture_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  s:\n    type: service\n    status: converged\n    hash: \"blake3:def\"\n  n:\n    type: network\n    status: converged\n    hash: \"blake3:ghi\"\n");
        assert!(cmd_status_fleet_resource_security_posture_score(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1017_security_posture_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_security_posture_score(dir.path(), None, true).is_ok());
    }

    // ── FJ-1018: validate --check-resource-when-condition-syntax ──

    #[test]
    fn test_fj1018_when_syntax_valid() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    when: '{{inputs.gpu_backend}} != \"cpu\"'\n");
        assert!(cmd_validate_check_resource_when_condition_syntax(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1018_when_syntax_no_when() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_validate_check_resource_when_condition_syntax(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1018_when_syntax_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_when_condition_syntax(f.path(), true).is_ok());
    }

    // ── FJ-1019: graph --resource-dependency-conditional-subgraph ──

    #[test]
    fn test_fj1019_conditional_subgraph_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_conditional_subgraph(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1019_conditional_subgraph_mixed() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    when: '{{inputs.env}} == \"production\"'\n");
        assert!(cmd_graph_resource_dependency_conditional_subgraph(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1019_conditional_subgraph_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    when: 'true'\n");
        assert!(cmd_graph_resource_dependency_conditional_subgraph(f.path(), true).is_ok());
    }

    // ── FJ-1020: apply --notify-custom-severity-filter ──

    #[test]
    fn test_fj1020_severity_filter_notification() {
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
            custom_digest: None,
            custom_severity_filter: Some("https://hooks.example.com|warn"),
        };
        let result: Result<(), String> = Err("apply failed".to_string());
        super::super::dispatch_notify::send_apply_notifications(&opts, &result, std::path::Path::new("test.yaml"));
    }

    #[test]
    fn test_fj1020_severity_filter_none() {
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
}
