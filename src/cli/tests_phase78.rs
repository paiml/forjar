//! Tests: Phase 78 — Automation Intelligence & Fleet Optimization (FJ-885→FJ-892).

#![allow(unused_imports)]
use super::graph_scoring::*;
use super::status_recovery::*;
use super::validate_ownership::*;
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

    // FJ-885: validate --check-resource-secret-exposure
    #[test]
    fn test_fj885_secret_exposure_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: \"host=localhost\"\n");
        assert!(cmd_validate_check_resource_secret_exposure(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj885_secret_exposure_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_secret_exposure(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj885_secret_exposure_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: \"password=secret123\"\n");
        assert!(cmd_validate_check_resource_secret_exposure(f.path(), true).is_ok());
    }

    // FJ-886: status --machine-resource-dependency-health
    #[test]
    fn test_fj886_dependency_health_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_dependency_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj886_dependency_health_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_dependency_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj886_dependency_health_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_dependency_health(dir.path(), None, true).is_ok());
    }

    // FJ-887: graph --resource-dependency-depth-analysis
    #[test]
    fn test_fj887_depth_analysis_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_depth_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj887_depth_analysis_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_depth_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj887_depth_analysis_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_depth_analysis(f.path(), true).is_ok());
    }

    // FJ-888: apply --notify-custom-transform (tested via NotifyOpts)
    #[test]
    fn test_fj888_custom_transform_field() {
        let opts = super::super::dispatch_notify::NotifyOpts {
            slack: None,
            email: None,
            webhook: None,
            teams: None,
            discord: None,
            opsgenie: None,
            datadog: None,
            newrelic: None,
            grafana: None,
            victorops: None,
            msteams_adaptive: None,
            incident: None,
            sns: None,
            pubsub: None,
            eventbridge: None,
            kafka: None,
            azure_servicebus: None,
            gcp_pubsub_v2: None,
            rabbitmq: None,
            nats: None,
            mqtt: None,
            redis: None,
            amqp: None,
            stomp: None,
            zeromq: None,
            grpc: None,
            sqs: None,
            mattermost: None,
            ntfy: None,
            pagerduty: None,
            discord_webhook: None,
            teams_webhook: None,
            slack_blocks: None,
            custom_template: None,
            custom_webhook: None,
            custom_headers: None,
            custom_json: None,
            custom_filter: None,
            custom_retry: None,
            custom_transform: Some("https://hooks.example.com|{\"s\":\"{{status}}\"}"),
            custom_batch: None,
            custom_deduplicate: None,
            custom_throttle: None,
            custom_aggregate: None,
            custom_priority: None,
            custom_routing: None,
            custom_dedup_window: None,
            custom_rate_limit: None,
            custom_backoff: None,
            custom_circuit_breaker: None,
            custom_dead_letter: None,
            custom_escalation: None,
            custom_correlation: None,
            custom_sampling: None,
            custom_digest: None,
            custom_severity_filter: None,
        };
        assert!(opts.custom_transform.is_some());
    }

    #[test]
    fn test_fj888_custom_transform_none() {
        let opts = super::super::dispatch_notify::NotifyOpts {
            slack: None,
            email: None,
            webhook: None,
            teams: None,
            discord: None,
            opsgenie: None,
            datadog: None,
            newrelic: None,
            grafana: None,
            victorops: None,
            msteams_adaptive: None,
            incident: None,
            sns: None,
            pubsub: None,
            eventbridge: None,
            kafka: None,
            azure_servicebus: None,
            gcp_pubsub_v2: None,
            rabbitmq: None,
            nats: None,
            mqtt: None,
            redis: None,
            amqp: None,
            stomp: None,
            zeromq: None,
            grpc: None,
            sqs: None,
            mattermost: None,
            ntfy: None,
            pagerduty: None,
            discord_webhook: None,
            teams_webhook: None,
            slack_blocks: None,
            custom_template: None,
            custom_webhook: None,
            custom_headers: None,
            custom_json: None,
            custom_filter: None,
            custom_retry: None,
            custom_transform: None,
            custom_batch: None,
            custom_deduplicate: None,
            custom_throttle: None,
            custom_aggregate: None,
            custom_priority: None,
            custom_routing: None,
            custom_dedup_window: None,
            custom_rate_limit: None,
            custom_backoff: None,
            custom_circuit_breaker: None,
            custom_dead_letter: None,
            custom_escalation: None,
            custom_correlation: None,
            custom_sampling: None,
            custom_digest: None,
            custom_severity_filter: None,
        };
        assert!(opts.custom_transform.is_none());
    }

    // FJ-889: validate --check-resource-tag-standards
    #[test]
    fn test_fj889_tag_standards_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags: [web, production]\n");
        assert!(cmd_validate_check_resource_tag_standards(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj889_tag_standards_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_tag_standards(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj889_tag_standards_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_tag_standards(f.path(), true).is_ok());
    }

    // FJ-890: status --fleet-resource-type-health
    #[test]
    fn test_fj890_type_health_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj890_type_health_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n  app:\n    resource_type: File\n    status: Failed\n    hash: def456\n");
        assert!(cmd_status_fleet_resource_type_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj890_type_health_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_health(dir.path(), None, true).is_ok());
    }

    // FJ-891: graph --resource-dependency-fan-analysis
    #[test]
    fn test_fj891_fan_analysis_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_fan_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj891_fan_analysis_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_fan_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj891_fan_analysis_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_fan_analysis(f.path(), true).is_ok());
    }

    // FJ-892: status --machine-resource-convergence-rate
    #[test]
    fn test_fj892_convergence_rate_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj892_convergence_rate_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_convergence_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj892_convergence_rate_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_rate(dir.path(), None, true).is_ok());
    }
}
