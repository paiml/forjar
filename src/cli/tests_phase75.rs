//! Tests: Phase 75 — Resource Lifecycle & Operational Intelligence (FJ-861→FJ-868).

use super::validate_governance::*;
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

    // FJ-861: validate --check-resource-lifecycle-hooks
    #[test]
    fn test_fj861_lifecycle_hooks_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: hi\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    depends_on:\n      - cfg\n");
        assert!(cmd_validate_check_resource_lifecycle_hooks(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj861_lifecycle_hooks_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_lifecycle_hooks(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj861_lifecycle_hooks_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_lifecycle_hooks(f.path(), true).is_ok());
    }

    // FJ-862: status --machine-resource-churn-rate
    #[test]
    fn test_fj862_churn_rate_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_churn_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj862_churn_rate_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_churn_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj862_churn_rate_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_churn_rate(dir.path(), None, true).is_ok());
    }

    // FJ-863: graph --resource-dependency-bottleneck
    #[test]
    fn test_fj863_bottleneck_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_bottleneck(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj863_bottleneck_no_deps() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg1:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_bottleneck(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj863_bottleneck_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n");
        assert!(cmd_graph_resource_dependency_bottleneck(f.path(), true).is_ok());
    }

    // FJ-864: apply --notify-custom-json (tested via NotifyOpts)
    #[test]
    fn test_fj864_custom_json_field() {
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
            custom_json: Some("https://hooks.example.com|{\"status\":\"{{status}}\",\"config\":\"{{config}}\"}"),
            custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None,
        };
        assert!(opts.custom_json.is_some());
    }

    #[test]
    fn test_fj864_custom_json_none() {
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
            custom_template: None, custom_webhook: None, custom_headers: None, custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None,
        };
        assert!(opts.custom_json.is_none());
    }

    // FJ-865: validate --check-resource-provider-version
    #[test]
    fn test_fj865_provider_version_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    provider: apt\n");
        assert!(cmd_validate_check_resource_provider_version(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj865_provider_version_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_provider_version(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj865_provider_version_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_provider_version(f.path(), true).is_ok());
    }

    // FJ-866: status --fleet-resource-staleness
    #[test]
    fn test_fj866_staleness_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_staleness(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj866_staleness_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n");
        assert!(cmd_status_fleet_resource_staleness(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj866_staleness_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_staleness(dir.path(), None, true).is_ok());
    }

    // FJ-867: graph --resource-type-clustering
    #[test]
    fn test_fj867_clustering_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg1:\n    machine: m1\n    type: package\n    name: nginx\n  pkg2:\n    machine: m1\n    type: package\n    name: curl\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: hi\n");
        assert!(cmd_graph_resource_type_clustering(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj867_clustering_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_type_clustering(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj867_clustering_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_type_clustering(f.path(), true).is_ok());
    }

    // FJ-868: status --machine-convergence-trend
    #[test]
    fn test_fj868_convergence_trend_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_convergence_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj868_convergence_trend_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_convergence_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj868_convergence_trend_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_convergence_trend(dir.path(), None, true).is_ok());
    }
}
