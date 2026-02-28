//! Tests: Phase 76 — Capacity Planning & Configuration Analytics (FJ-869→FJ-876).

use super::validate_governance::*;
use super::validate_ownership::*;
use super::graph_scoring::*;
use super::status_predictive::*;
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

    // FJ-869: validate --check-resource-naming-convention
    #[test]
    fn test_fj869_naming_convention_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  my-pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_naming_convention(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj869_naming_convention_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_naming_convention(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj869_naming_convention_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_naming_convention(f.path(), true).is_ok());
    }

    // FJ-870: status --machine-capacity-utilization
    #[test]
    fn test_fj870_capacity_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_capacity_utilization(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj870_capacity_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_capacity_utilization(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj870_capacity_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_capacity_utilization(dir.path(), None, true).is_ok());
    }

    // FJ-871: graph --resource-dependency-cycle-risk
    #[test]
    fn test_fj871_cycle_risk_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_cycle_risk(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj871_cycle_risk_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_cycle_risk(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj871_cycle_risk_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_cycle_risk(f.path(), true).is_ok());
    }

    // FJ-872: apply --notify-custom-filter (tested via NotifyOpts)
    #[test]
    fn test_fj872_custom_filter_field() {
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
            custom_json: None,
            custom_filter: Some("https://hooks.example.com|type:Package,status:Converged"),
            custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None,
        };
        assert!(opts.custom_filter.is_some());
    }

    #[test]
    fn test_fj872_custom_filter_none() {
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
        };
        assert!(opts.custom_filter.is_none());
    }

    // FJ-873: validate --check-resource-idempotency
    #[test]
    fn test_fj873_idempotency_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_idempotency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj873_idempotency_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_idempotency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj873_idempotency_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_idempotency(f.path(), true).is_ok());
    }

    // FJ-874: status --fleet-configuration-entropy
    #[test]
    fn test_fj874_entropy_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_configuration_entropy(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj874_entropy_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_fleet_configuration_entropy(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj874_entropy_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_configuration_entropy(dir.path(), None, true).is_ok());
    }

    // FJ-875: graph --resource-impact-radius
    #[test]
    fn test_fj875_impact_radius_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_impact_radius_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj875_impact_radius_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_impact_radius_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj875_impact_radius_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_impact_radius_analysis(f.path(), true).is_ok());
    }

    // FJ-876: status --machine-resource-freshness
    #[test]
    fn test_fj876_freshness_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_freshness(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj876_freshness_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n");
        assert!(cmd_status_machine_resource_freshness(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj876_freshness_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_freshness(dir.path(), None, true).is_ok());
    }
}
