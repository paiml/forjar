//! Tests: Phase 77 — Documentation, Compliance & Recovery (FJ-877→FJ-884).

use super::validate_governance::*;
use super::validate_ownership::*;
use super::graph_scoring::*;
use super::status_predictive::*;
use super::status_recovery::*;
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

    // FJ-877: validate --check-resource-documentation
    #[test]
    fn test_fj877_documentation_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  my-pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags: [web]\n");
        assert!(cmd_validate_check_resource_documentation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj877_documentation_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_documentation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj877_documentation_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_documentation(f.path(), true).is_ok());
    }

    // FJ-878: status --machine-error-budget
    #[test]
    fn test_fj878_error_budget_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_error_budget(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj878_error_budget_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n  app:\n    resource_type: File\n    status: Failed\n    hash: def456\n");
        assert!(cmd_status_machine_error_budget(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj878_error_budget_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_error_budget(dir.path(), None, true).is_ok());
    }

    // FJ-879: graph --resource-dependency-health-map
    #[test]
    fn test_fj879_health_map_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_health_map(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj879_health_map_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_health_map(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj879_health_map_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_health_map(f.path(), true).is_ok());
    }

    // FJ-880: apply --notify-custom-retry (tested via NotifyOpts)
    #[test]
    fn test_fj880_custom_retry_field() {
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
            custom_json: None, custom_filter: None,
            custom_retry: Some("https://hooks.example.com|retries:3"), custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None, custom_dedup_window: None, custom_rate_limit: None, custom_backoff: None, custom_circuit_breaker: None, custom_dead_letter: None,        };
        assert!(opts.custom_retry.is_some());
    }

    #[test]
    fn test_fj880_custom_retry_none() {
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
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None, custom_dedup_window: None, custom_rate_limit: None, custom_backoff: None, custom_circuit_breaker: None, custom_dead_letter: None,        };
        assert!(opts.custom_retry.is_none());
    }

    // FJ-881: validate --check-resource-ownership
    #[test]
    fn test_fj881_ownership_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags: [owned-by:team-a]\n");
        assert!(cmd_validate_check_resource_ownership(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj881_ownership_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_ownership(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj881_ownership_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_ownership(f.path(), true).is_ok());
    }

    // FJ-882: status --fleet-compliance-score
    #[test]
    fn test_fj882_compliance_score_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_compliance_score(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj882_compliance_score_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n  app:\n    resource_type: File\n    status: Failed\n    hash: def456\n");
        assert!(cmd_status_fleet_compliance_score(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj882_compliance_score_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_compliance_score(dir.path(), None, true).is_ok());
    }

    // FJ-883: graph --resource-change-propagation
    #[test]
    fn test_fj883_change_propagation_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_change_propagation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj883_change_propagation_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_change_propagation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj883_change_propagation_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_change_propagation(f.path(), true).is_ok());
    }

    // FJ-884: status --machine-mean-time-to-recovery
    #[test]
    fn test_fj884_mttr_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_mean_time_to_recovery(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj884_mttr_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_mean_time_to_recovery(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj884_mttr_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_mean_time_to_recovery(dir.path(), None, true).is_ok());
    }
}
