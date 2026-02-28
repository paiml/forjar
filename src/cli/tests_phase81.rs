//! Tests: Phase 81 — Predictive Infrastructure Intelligence (FJ-909→FJ-916).

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

    // FJ-909: validate --check-resource-dependency-completeness
    #[test]
    fn test_fj909_dependency_completeness_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_validate_check_resource_dependency_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj909_dependency_completeness_missing() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [nonexistent]\n");
        assert!(cmd_validate_check_resource_dependency_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj909_dependency_completeness_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_completeness(f.path(), true).is_ok());
    }

    // FJ-910: status --machine-resource-mttr-estimate
    #[test]
    fn test_fj910_mttr_estimate_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_mttr_estimate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj910_mttr_estimate_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_mttr_estimate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj910_mttr_estimate_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_mttr_estimate(dir.path(), None, true).is_ok());
    }

    // FJ-911: graph --resource-dependency-centrality-score
    #[test]
    fn test_fj911_centrality_score_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_centrality_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj911_centrality_score_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_centrality_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj911_centrality_score_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_centrality_score(f.path(), true).is_ok());
    }

    // FJ-912: apply --notify-custom-throttle (tested via NotifyOpts)
    #[test]
    fn test_fj912_custom_throttle_field() {
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
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None,
            custom_throttle: Some("https://hooks.example.com|max_per_minute:10"),
            custom_aggregate: None, custom_priority: None, custom_routing: None, custom_dedup_window: None, custom_rate_limit: None, custom_backoff: None, custom_circuit_breaker: None, custom_dead_letter: None, custom_escalation: None,        };
        assert!(opts.custom_throttle.is_some());
    }

    #[test]
    fn test_fj912_custom_throttle_none() {
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
        assert!(opts.custom_throttle.is_none());
    }

    // FJ-913: validate --check-resource-state-coverage
    #[test]
    fn test_fj913_state_coverage_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    state: present\n");
        assert!(cmd_validate_check_resource_state_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj913_state_coverage_missing() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_state_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj913_state_coverage_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_state_coverage(f.path(), true).is_ok());
    }

    // FJ-914: status --fleet-resource-convergence-forecast
    #[test]
    fn test_fj914_convergence_forecast_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_forecast(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj914_convergence_forecast_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_fleet_resource_convergence_forecast(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj914_convergence_forecast_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_forecast(dir.path(), None, true).is_ok());
    }

    // FJ-915: graph --resource-dependency-bridge-detection
    #[test]
    fn test_fj915_bridge_detection_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_bridge_detection(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj915_bridge_detection_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_bridge_detection(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj915_bridge_detection_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_bridge_detection(f.path(), true).is_ok());
    }

    // FJ-916: status --machine-resource-error-budget-forecast
    #[test]
    fn test_fj916_error_budget_forecast_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_budget_forecast(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj916_error_budget_forecast_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_error_budget_forecast(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj916_error_budget_forecast_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_budget_forecast(dir.path(), None, true).is_ok());
    }
}
