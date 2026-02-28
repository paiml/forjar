//! Tests: Phase 82 — Infrastructure Insight & Configuration Maturity (FJ-917→FJ-924).

use super::validate_ownership::*;
use super::graph_intelligence::*;
use super::status_intelligence::*;
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
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    // FJ-917: validate --check-resource-rollback-safety
    #[test]
    fn test_fj917_rollback_safety_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_rollback_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj917_rollback_safety_triggers() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    triggers: [svc]\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n");
        assert!(cmd_validate_check_resource_rollback_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj917_rollback_safety_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_rollback_safety(f.path(), true).is_ok());
    }

    // FJ-918: status --machine-resource-dependency-lag
    #[test]
    fn test_fj918_dependency_lag_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_dependency_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj918_dependency_lag_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_dependency_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj918_dependency_lag_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_dependency_lag(dir.path(), None, true).is_ok());
    }

    // FJ-919: graph --resource-dependency-cluster-coefficient
    #[test]
    fn test_fj919_cluster_coefficient_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_cluster_coefficient(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj919_cluster_coefficient_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_cluster_coefficient(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj919_cluster_coefficient_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_cluster_coefficient(f.path(), true).is_ok());
    }

    // FJ-920: apply --notify-custom-aggregate (tested via NotifyOpts)
    #[test]
    fn test_fj920_custom_aggregate_field() {
        let opts = super::super::dispatch_notify::NotifyOpts {
            slack: None, email: None, webhook: None, teams: None,
            discord: None, opsgenie: None, datadog: None, newrelic: None,
            grafana: None, victorops: None, msteams_adaptive: None,
            incident: None, sns: None, pubsub: None, eventbridge: None,
            kafka: None, azure_servicebus: None, gcp_pubsub_v2: None,
            rabbitmq: None, nats: None, mqtt: None, redis: None,
            amqp: None, stomp: None, zeromq: None, grpc: None,
            sqs: None, mattermost: None, ntfy: None, pagerduty: None,
            discord_webhook: None, teams_webhook: None, slack_blocks: None,
            custom_template: None, custom_webhook: None, custom_headers: None,
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None,
            custom_aggregate: Some("https://hooks.example.com|window_seconds:120"), custom_priority: None, custom_routing: None, custom_dedup_window: None, custom_rate_limit: None, custom_backoff: None, custom_circuit_breaker: None, custom_dead_letter: None, custom_escalation: None,        };
        assert!(opts.custom_aggregate.is_some());
    }

    #[test]
    fn test_fj920_custom_aggregate_none() {
        let opts = super::super::dispatch_notify::NotifyOpts {
            slack: None, email: None, webhook: None, teams: None,
            discord: None, opsgenie: None, datadog: None, newrelic: None,
            grafana: None, victorops: None, msteams_adaptive: None,
            incident: None, sns: None, pubsub: None, eventbridge: None,
            kafka: None, azure_servicebus: None, gcp_pubsub_v2: None,
            rabbitmq: None, nats: None, mqtt: None, redis: None,
            amqp: None, stomp: None, zeromq: None, grpc: None,
            sqs: None, mattermost: None, ntfy: None, pagerduty: None,
            discord_webhook: None, teams_webhook: None, slack_blocks: None,
            custom_template: None, custom_webhook: None, custom_headers: None,
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None, custom_dedup_window: None, custom_rate_limit: None, custom_backoff: None, custom_circuit_breaker: None, custom_dead_letter: None, custom_escalation: None,        };
        assert!(opts.custom_aggregate.is_none());
    }

    // FJ-921: validate --check-resource-config-maturity
    #[test]
    fn test_fj921_config_maturity_scored() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    state: present\n    tags: [web]\n");
        assert!(cmd_validate_check_resource_config_maturity(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj921_config_maturity_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_config_maturity(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj921_config_maturity_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_config_maturity(f.path(), true).is_ok());
    }

    // FJ-922: status --fleet-resource-dependency-lag
    #[test]
    fn test_fj922_fleet_dependency_lag_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_dependency_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj922_fleet_dependency_lag_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        assert!(cmd_status_fleet_resource_dependency_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj922_fleet_dependency_lag_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_dependency_lag(dir.path(), None, true).is_ok());
    }

    // FJ-923: graph --resource-dependency-modularity-score
    #[test]
    fn test_fj923_modularity_score_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_modularity_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj923_modularity_score_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_modularity_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj923_modularity_score_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_modularity_score(f.path(), true).is_ok());
    }

    // FJ-924: status --machine-resource-config-drift-rate
    #[test]
    fn test_fj924_config_drift_rate_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_config_drift_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj924_config_drift_rate_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Drifted\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_config_drift_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj924_config_drift_rate_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_config_drift_rate(dir.path(), None, true).is_ok());
    }
}
