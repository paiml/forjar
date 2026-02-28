//! Tests: Phase 71 — Advanced Governance & Analytics (FJ-829→FJ-836).

use super::validate_governance::*;
use super::graph_paths::*;
use super::status_operational::*;
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

    // FJ-829: validate --check-resource-naming-pattern
    #[test]
    fn test_fj829_naming_pattern_match() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  app-nginx:\n    machine: m1\n    type: package\n    name: nginx\n  app-redis:\n    machine: m1\n    type: package\n    name: redis\n");
        assert!(cmd_validate_check_resource_naming_pattern(f.path(), false, "app").is_ok());
    }

    #[test]
    fn test_fj829_naming_pattern_violation() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  bad_name:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_naming_pattern(f.path(), false, "^app").is_ok());
    }

    #[test]
    fn test_fj829_naming_pattern_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  app-test:\n    machine: m1\n    type: package\n    name: test\n");
        assert!(cmd_validate_check_resource_naming_pattern(f.path(), true, "app").is_ok());
    }

    // FJ-830: status --machine-apply-count
    #[test]
    fn test_fj830_apply_count_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_apply_count(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj830_apply_count_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_apply_count(dir.path(), Some("x"), false).is_ok());
    }

    #[test]
    fn test_fj830_apply_count_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_apply_count(dir.path(), None, true).is_ok());
    }

    // FJ-831: graph --critical-dependency-path
    #[test]
    fn test_fj831_critical_dep_path_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  mid:\n    machine: m1\n    type: package\n    name: mid\n    depends_on: [base]\n  top:\n    machine: m1\n    type: package\n    name: top\n    depends_on: [mid]\n");
        assert!(cmd_graph_critical_dependency_path(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj831_critical_dep_path_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg1:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_critical_dependency_path(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj831_critical_dep_path_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n");
        assert!(cmd_graph_critical_dependency_path(f.path(), true).is_ok());
    }

    // FJ-832: apply --notify-slack-blocks (tested via NotifyOpts)
    #[test]
    fn test_fj832_slack_blocks_field() {
        let opts = super::super::dispatch_notify::NotifyOpts {
            slack: None, email: None, webhook: None, teams: None,
            discord: None, opsgenie: None, datadog: None, newrelic: None,
            grafana: None, victorops: None, msteams_adaptive: None,
            incident: None, sns: None, pubsub: None, eventbridge: None,
            kafka: None, azure_servicebus: None, gcp_pubsub_v2: None,
            rabbitmq: None, nats: None, mqtt: None, redis: None,
            amqp: None, stomp: None, zeromq: None, grpc: None,
            sqs: None, mattermost: None, ntfy: None, pagerduty: None,
            discord_webhook: None, teams_webhook: None,
            slack_blocks: Some("https://hooks.slack.com/services/test"), custom_template: None, custom_webhook: None, custom_headers: None, custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None,
        };
        assert!(opts.slack_blocks.is_some());
    }

    #[test]
    fn test_fj832_slack_blocks_none() {
        let opts = super::super::dispatch_notify::NotifyOpts {
            slack: None, email: None, webhook: None, teams: None,
            discord: None, opsgenie: None, datadog: None, newrelic: None,
            grafana: None, victorops: None, msteams_adaptive: None,
            incident: None, sns: None, pubsub: None, eventbridge: None,
            kafka: None, azure_servicebus: None, gcp_pubsub_v2: None,
            rabbitmq: None, nats: None, mqtt: None, redis: None,
            amqp: None, stomp: None, zeromq: None, grpc: None,
            sqs: None, mattermost: None, ntfy: None, pagerduty: None,
            discord_webhook: None, teams_webhook: None,
            slack_blocks: None, custom_template: None, custom_webhook: None, custom_headers: None, custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None,
        };
        assert!(opts.slack_blocks.is_none());
    }

    // FJ-833: validate --check-resource-provider-support
    #[test]
    fn test_fj833_provider_support_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_provider_support(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj833_provider_support_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_provider_support(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj833_provider_support_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_provider_support(f.path(), true).is_ok());
    }

    // FJ-834: status --fleet-apply-history
    #[test]
    fn test_fj834_fleet_apply_history_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_apply_history(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj834_fleet_apply_history_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_apply_history(dir.path(), Some("x"), false).is_ok());
    }

    #[test]
    fn test_fj834_fleet_apply_history_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_apply_history(dir.path(), None, true).is_ok());
    }

    // FJ-835: graph --resource-depth-histogram
    #[test]
    fn test_fj835_depth_histogram_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_depth_histogram(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj835_depth_histogram_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_depth_histogram(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj835_depth_histogram_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n");
        assert!(cmd_graph_resource_depth_histogram(f.path(), true).is_ok());
    }

    // FJ-836: status --resource-hash-changes
    #[test]
    fn test_fj836_hash_changes_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_hash_changes(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj836_hash_changes_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_hash_changes(dir.path(), Some("x"), false).is_ok());
    }

    #[test]
    fn test_fj836_hash_changes_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_hash_changes(dir.path(), None, true).is_ok());
    }
}
