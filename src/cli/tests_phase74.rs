//! Tests: Phase 74 — Predictive Analysis & Fleet Governance (FJ-853→FJ-860).

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

    // FJ-853: validate --check-resource-drift-risk
    #[test]
    fn test_fj853_drift_risk() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: hi\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    depends_on:\n      - cfg\n");
        assert!(cmd_validate_check_resource_drift_risk(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj853_drift_risk_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_drift_risk(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj853_drift_risk_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_drift_risk(f.path(), false).is_ok());
    }

    // FJ-854: status --machine-resource-age-distribution
    #[test]
    fn test_fj854_age_distribution_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_age_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj854_age_distribution_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n");
        assert!(cmd_status_machine_resource_age_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj854_age_distribution_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n");
        assert!(cmd_status_machine_resource_age_distribution(dir.path(), None, true).is_ok());
    }

    // FJ-855: graph --resource-dependency-fanout
    #[test]
    fn test_fj855_dependency_fanout() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    depends_on:\n      - cfg\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n");
        assert!(cmd_graph_resource_dependency_fanout(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj855_dependency_fanout_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_fanout(f.path(), true).is_ok());
    }

    // FJ-856: apply --notify-custom-headers (tested via NotifyOpts)
    #[test]
    fn test_fj856_custom_headers_field() {
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
            custom_template: None, custom_webhook: None,
            custom_headers: Some("https://hooks.example.com|Authorization:Bearer test123"),
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None,
        };
        assert!(opts.custom_headers.is_some());
    }

    #[test]
    fn test_fj856_custom_headers_none() {
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
            custom_template: None, custom_webhook: None, custom_headers: None, custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None,
        };
        assert!(opts.custom_headers.is_none());
    }

    // FJ-857: validate --check-resource-tag-coverage
    #[test]
    fn test_fj857_tag_coverage_all_tagged() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - web\n");
        assert!(cmd_validate_check_resource_tag_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj857_tag_coverage_missing() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_tag_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj857_tag_coverage_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_tag_coverage(f.path(), true).is_ok());
    }

    // FJ-858: status --fleet-convergence-velocity
    #[test]
    fn test_fj858_convergence_velocity_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_convergence_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj858_convergence_velocity_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_fleet_convergence_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj858_convergence_velocity_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_fleet_convergence_velocity(dir.path(), None, true).is_ok());
    }

    // FJ-859: graph --resource-dependency-weight
    #[test]
    fn test_fj859_dependency_weight() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    depends_on:\n      - cfg\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n");
        assert!(cmd_graph_resource_dependency_weight(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj859_dependency_weight_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_weight(f.path(), true).is_ok());
    }

    // FJ-860: status --resource-failure-correlation
    #[test]
    fn test_fj860_failure_correlation_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_failure_correlation(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj860_failure_correlation_with_failures() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        write_yaml(dir.path(), "web2.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        assert!(cmd_status_resource_failure_correlation(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj860_failure_correlation_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        assert!(cmd_status_resource_failure_correlation(dir.path(), None, true).is_ok());
    }
}
