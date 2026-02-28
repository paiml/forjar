//! Tests: Phase 83 — Advanced Graph Analytics & Fleet Observability (FJ-925→FJ-932).

use super::validate_ordering::*;
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

    // FJ-925: validate --check-resource-dependency-ordering
    #[test]
    fn test_fj925_dependency_ordering_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_validate_check_resource_dependency_ordering(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj925_dependency_ordering_missing() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [nonexistent]\n");
        assert!(cmd_validate_check_resource_dependency_ordering(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj925_dependency_ordering_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_ordering(f.path(), true).is_ok());
    }

    // FJ-926: status --machine-resource-convergence-lag
    #[test]
    fn test_fj926_convergence_lag_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj926_convergence_lag_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_convergence_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj926_convergence_lag_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_lag(dir.path(), None, true).is_ok());
    }

    // FJ-927: graph --resource-dependency-diameter
    #[test]
    fn test_fj927_diameter_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_diameter(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj927_diameter_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_diameter(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj927_diameter_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_diameter(f.path(), true).is_ok());
    }

    // FJ-928: apply --notify-custom-priority (tested via NotifyOpts)
    #[test]
    fn test_fj928_custom_priority_field() {
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
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None,
            custom_priority: Some("https://hooks.example.com|default:high"), custom_routing: None, custom_dedup_window: None,
        };
        assert!(opts.custom_priority.is_some());
    }

    #[test]
    fn test_fj928_custom_priority_none() {
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
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None, custom_dedup_window: None,
        };
        assert!(opts.custom_priority.is_none());
    }

    // FJ-929: validate --check-resource-tag-completeness
    #[test]
    fn test_fj929_tag_completeness_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags: [web]\n");
        assert!(cmd_validate_check_resource_tag_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj929_tag_completeness_missing() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_tag_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj929_tag_completeness_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_tag_completeness(f.path(), true).is_ok());
    }

    // FJ-930: status --fleet-resource-convergence-lag
    #[test]
    fn test_fj930_fleet_convergence_lag_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj930_fleet_convergence_lag_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        assert!(cmd_status_fleet_resource_convergence_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj930_fleet_convergence_lag_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_lag(dir.path(), None, true).is_ok());
    }

    // FJ-931: graph --resource-dependency-eccentricity
    #[test]
    fn test_fj931_eccentricity_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_eccentricity(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj931_eccentricity_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_eccentricity(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj931_eccentricity_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_eccentricity(f.path(), true).is_ok());
    }

    // FJ-932: status --machine-resource-dependency-depth
    #[test]
    fn test_fj932_dependency_depth_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_dependency_depth(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj932_dependency_depth_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_dependency_depth(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj932_dependency_depth_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_dependency_depth(dir.path(), None, true).is_ok());
    }
}
