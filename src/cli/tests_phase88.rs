//! Tests: Phase 88 — Drift Velocity, Trigger Refs & Topological Depth (FJ-965→FJ-972).

use super::validate_ordering::*;
use super::graph_intelligence_ext::*;
use super::status_intelligence_ext::*;
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

    // ── FJ-965: validate --check-resource-trigger-refs ──

    #[test]
    fn test_fj965_trigger_refs_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_trigger_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj965_trigger_refs_valid_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_trigger_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj965_trigger_refs_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_validate_check_resource_trigger_refs(f.path(), true).is_ok());
    }

    // ── FJ-966: status --machine-resource-drift-velocity ──

    #[test]
    fn test_fj966_drift_velocity_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj966_drift_velocity_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n    duration_seconds: 3600.0\n  g:\n    type: file\n    status: converged\n    hash: \"blake3:def\"\n    duration_seconds: 1800.0\n");
        assert!(cmd_status_machine_resource_drift_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj966_drift_velocity_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_velocity(dir.path(), None, true).is_ok());
    }

    // ── FJ-967: graph --resource-dependency-topological-depth ──

    #[test]
    fn test_fj967_topological_depth_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_topological_depth(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj967_topological_depth_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_topological_depth(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj967_topological_depth_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_topological_depth(f.path(), true).is_ok());
    }

    // ── FJ-968: apply --notify-custom-circuit-breaker ──

    #[test]
    fn test_fj968_custom_circuit_breaker_notification() {
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
            custom_circuit_breaker: Some("https://hooks.example.com|5"),
            custom_dead_letter: None,
        };
        let result: Result<(), String> = Ok(());
        super::super::dispatch_notify::send_apply_notifications(&opts, &result, std::path::Path::new("test.yaml"));
    }

    #[test]
    fn test_fj968_custom_circuit_breaker_none() {
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
        };
        let result: Result<(), String> = Ok(());
        super::super::dispatch_notify::send_apply_notifications(&opts, &result, std::path::Path::new("test.yaml"));
    }

    // ── FJ-969: validate --check-resource-param-type-safety ──

    #[test]
    fn test_fj969_param_type_safety_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_param_type_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj969_param_type_safety_with_params() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nparams:\n  port:\n    default: \"8080\"\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: \"port={{port}}\"\n");
        assert!(cmd_validate_check_resource_param_type_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj969_param_type_safety_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_param_type_safety(f.path(), true).is_ok());
    }

    // ── FJ-970: status --fleet-resource-recovery-rate ──

    #[test]
    fn test_fj970_fleet_recovery_rate_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_recovery_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj970_fleet_recovery_rate_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: failed\n    hash: \"\"\n");
        write_yaml(dir.path(), "db/state.lock.yaml", "schema: \"1.0\"\nmachine: db\nhostname: db\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  h:\n    type: file\n    status: converged\n    hash: \"blake3:xyz\"\n");
        assert!(cmd_status_fleet_resource_recovery_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj970_fleet_recovery_rate_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_recovery_rate(dir.path(), None, true).is_ok());
    }

    // ── FJ-971: graph --resource-dependency-weak-links ──

    #[test]
    fn test_fj971_weak_links_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_weak_links(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj971_weak_links_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_weak_links(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj971_weak_links_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_weak_links(f.path(), true).is_ok());
    }

    // ── FJ-972: status --machine-resource-convergence-efficiency ──

    #[test]
    fn test_fj972_convergence_efficiency_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_efficiency(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj972_convergence_efficiency_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n    duration_seconds: 2.5\n  g:\n    type: file\n    status: drifted\n    hash: \"blake3:def\"\n    duration_seconds: 10.0\n");
        assert!(cmd_status_machine_resource_convergence_efficiency(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj972_convergence_efficiency_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_efficiency(dir.path(), None, true).is_ok());
    }
}
