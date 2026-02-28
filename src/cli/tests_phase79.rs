//! Tests: Phase 79 — Security Hardening & Operational Insights (FJ-893→FJ-900).

use super::validate_ownership::*;
use super::graph_scoring::*;
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

    // FJ-893: validate --check-resource-privilege-escalation
    #[test]
    fn test_fj893_privilege_escalation_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: \"host=localhost\"\n");
        assert!(cmd_validate_check_resource_privilege_escalation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj893_privilege_escalation_risk() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/sudoers.d/app\n    content: \"app ALL=(ALL) NOPASSWD: ALL\"\n");
        assert!(cmd_validate_check_resource_privilege_escalation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj893_privilege_escalation_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_privilege_escalation(f.path(), true).is_ok());
    }

    // FJ-894: status --machine-resource-failure-correlation
    #[test]
    fn test_fj894_failure_correlation_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_failure_correlation(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj894_failure_correlation_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_failure_correlation(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj894_failure_correlation_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_failure_correlation(dir.path(), None, true).is_ok());
    }

    // FJ-895: graph --resource-dependency-isolation-score
    #[test]
    fn test_fj895_isolation_score_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_isolation_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj895_isolation_score_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_isolation_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj895_isolation_score_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_isolation_score(f.path(), true).is_ok());
    }

    // FJ-896: apply --notify-custom-batch (tested via NotifyOpts)
    #[test]
    fn test_fj896_custom_batch_field() {
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
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None,
            custom_batch: Some("https://hooks.example.com|5"), custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None,
        };
        assert!(opts.custom_batch.is_some());
    }

    #[test]
    fn test_fj896_custom_batch_none() {
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
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None,
        };
        assert!(opts.custom_batch.is_none());
    }

    // FJ-897: validate --check-resource-update-safety
    #[test]
    fn test_fj897_update_safety_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_update_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj897_update_safety_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_update_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj897_update_safety_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  mnt:\n    machine: m1\n    type: mount\n    path: /data\n    source: /dev/sda1\n");
        assert!(cmd_validate_check_resource_update_safety(f.path(), true).is_ok());
    }

    // FJ-898: status --fleet-resource-age-distribution
    #[test]
    fn test_fj898_age_distribution_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_age_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj898_age_distribution_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_fleet_resource_age_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj898_age_distribution_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_age_distribution(dir.path(), None, true).is_ok());
    }

    // FJ-899: graph --resource-dependency-stability-score
    #[test]
    fn test_fj899_stability_score_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_stability_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj899_stability_score_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_stability_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj899_stability_score_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_stability_score(f.path(), true).is_ok());
    }

    // FJ-900: status --machine-resource-rollback-readiness
    #[test]
    fn test_fj900_rollback_readiness_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_rollback_readiness(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj900_rollback_readiness_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_rollback_readiness(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj900_rollback_readiness_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_rollback_readiness(dir.path(), None, true).is_ok());
    }
}
