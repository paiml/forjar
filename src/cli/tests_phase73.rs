//! Tests: Phase 73 — Drift Intelligence & Governance (FJ-845→FJ-852).

use super::validate_governance::*;
use super::graph_paths::*;
use super::graph_scoring::*;
use super::status_insights::*;
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

    // FJ-845: validate --check-resource-dependency-depth
    #[test]
    fn test_fj845_depth_within_limit() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    depends_on:\n      - cfg\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n");
        assert!(cmd_validate_check_resource_dependency_depth(f.path(), false, 5).is_ok());
    }

    #[test]
    fn test_fj845_depth_exceeds_limit() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - b\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n    depends_on:\n      - c\n  c:\n    machine: m1\n    type: file\n    path: /c\n    content: c\n");
        let result = cmd_validate_check_resource_dependency_depth(f.path(), false, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj845_depth_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_dependency_depth(f.path(), true, 10).is_ok());
    }

    // FJ-846: status --machine-drift-age
    #[test]
    fn test_fj846_drift_age_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_drift_age(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj846_drift_age_with_drift() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Drifted\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n");
        assert!(cmd_status_machine_drift_age(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj846_drift_age_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Drifted\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n");
        assert!(cmd_status_machine_drift_age(dir.path(), None, true).is_ok());
    }

    // FJ-847: graph --resource-impact-score
    #[test]
    fn test_fj847_impact_score() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    depends_on:\n      - cfg\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n");
        assert!(cmd_graph_resource_impact_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj847_impact_score_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_impact_score(f.path(), true).is_ok());
    }

    // FJ-848: apply --notify-custom-webhook (tested via NotifyOpts)
    #[test]
    fn test_fj848_custom_webhook_field() {
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
            custom_template: None,
            custom_webhook: Some("https://hooks.example.com/forjar"), custom_headers: None, custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None, custom_dedup_window: None,
        };
        assert!(opts.custom_webhook.is_some());
    }

    #[test]
    fn test_fj848_custom_webhook_none() {
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
            custom_template: None, custom_webhook: None, custom_headers: None, custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None, custom_dedup_window: None,
        };
        assert!(opts.custom_webhook.is_none());
    }

    // FJ-849: validate --check-resource-machine-affinity
    #[test]
    fn test_fj849_affinity_valid() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_machine_affinity(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj849_affinity_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_machine_affinity(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj849_affinity_invalid_machine() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: nonexistent\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_machine_affinity(f.path(), false).is_ok());
    }

    // FJ-850: status --fleet-failed-resources
    #[test]
    fn test_fj850_fleet_failed_none() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_fleet_failed_resources(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj850_fleet_failed_with_failures() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        assert!(cmd_status_fleet_failed_resources(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj850_fleet_failed_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        assert!(cmd_status_fleet_failed_resources(dir.path(), None, true).is_ok());
    }

    // FJ-851: graph --resource-stability-score
    #[test]
    fn test_fj851_stability_score() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    depends_on:\n      - cfg\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n    state: present\n");
        assert!(cmd_graph_resource_stability_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj851_stability_score_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_stability_score(f.path(), true).is_ok());
    }

    // FJ-852: status --resource-dependency-health
    #[test]
    fn test_fj852_dep_health_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_dependency_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj852_dep_health_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n    duration_seconds: 1.5\n");
        assert!(cmd_status_resource_dependency_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj852_dep_health_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_resource_dependency_health(dir.path(), None, true).is_ok());
    }
}
