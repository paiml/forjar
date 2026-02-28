//! Tests: Phase 80 — Operational Resilience & Configuration Intelligence (FJ-901→FJ-908).

use super::validate_ownership::*;
use super::graph_advanced::*;
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

    // FJ-901: validate --check-resource-cross-machine-consistency
    #[test]
    fn test_fj901_cross_machine_consistency_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_cross_machine_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj901_cross_machine_consistency_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_cross_machine_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj901_cross_machine_consistency_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_cross_machine_consistency(f.path(), true).is_ok());
    }

    // FJ-902: status --machine-resource-health-trend
    #[test]
    fn test_fj902_health_trend_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_health_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj902_health_trend_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_health_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj902_health_trend_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_health_trend(dir.path(), None, true).is_ok());
    }

    // FJ-903: graph --resource-dependency-critical-path-length
    #[test]
    fn test_fj903_critical_path_length_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_critical_path_length(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj903_critical_path_length_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_critical_path_length(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj903_critical_path_length_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_critical_path_length(f.path(), true).is_ok());
    }

    // FJ-904: apply --notify-custom-deduplicate (tested via NotifyOpts)
    #[test]
    fn test_fj904_custom_deduplicate_field() {
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
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None,
            custom_deduplicate: Some("https://hooks.example.com|60"),
            custom_throttle: None,
        };
        assert!(opts.custom_deduplicate.is_some());
    }

    #[test]
    fn test_fj904_custom_deduplicate_none() {
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
        assert!(opts.custom_deduplicate.is_none());
    }

    // FJ-905: validate --check-resource-version-pinning
    #[test]
    fn test_fj905_version_pinning_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    version: '1.24'\n");
        assert!(cmd_validate_check_resource_version_pinning(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj905_version_pinning_unpinned() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_version_pinning(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj905_version_pinning_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_version_pinning(f.path(), true).is_ok());
    }

    // FJ-906: status --fleet-resource-drift-velocity
    #[test]
    fn test_fj906_drift_velocity_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj906_drift_velocity_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Drifted\n    hash: abc123\n");
        assert!(cmd_status_fleet_resource_drift_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj906_drift_velocity_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_velocity(dir.path(), None, true).is_ok());
    }

    // FJ-907: graph --resource-dependency-redundancy-score
    #[test]
    fn test_fj907_redundancy_score_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_redundancy_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj907_redundancy_score_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_redundancy_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj907_redundancy_score_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_redundancy_score(f.path(), true).is_ok());
    }

    // FJ-908: status --machine-resource-apply-success-trend
    #[test]
    fn test_fj908_apply_success_trend_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_success_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj908_apply_success_trend_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_apply_success_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj908_apply_success_trend_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_success_trend(dir.path(), None, true).is_ok());
    }
}
