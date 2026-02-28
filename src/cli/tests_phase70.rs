//! Tests: Phase 70 — Advanced Governance & Analytics (FJ-821→FJ-828).

use super::validate_advanced::*;
use super::graph_advanced::*;
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

    // FJ-821: validate --check-resource-dependencies-complete
    #[test]
    fn test_fj821_deps_complete_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_validate_check_resource_dependencies_complete(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj821_deps_complete_missing() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [missing-dep]\n");
        assert!(cmd_validate_check_resource_dependencies_complete(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj821_deps_complete_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n");
        assert!(cmd_validate_check_resource_dependencies_complete(f.path(), true).is_ok());
    }

    // FJ-822: status --machine-resource-health
    #[test]
    fn test_fj822_machine_resource_health_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj822_machine_resource_health_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_health(dir.path(), Some("x"), false).is_ok());
    }

    #[test]
    fn test_fj822_machine_resource_health_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_health(dir.path(), None, true).is_ok());
    }

    // FJ-823: graph --resource-dependency-chain
    #[test]
    fn test_fj823_dep_chain_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_chain(f.path(), "app", false).is_ok());
    }

    #[test]
    fn test_fj823_dep_chain_missing() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n");
        assert!(cmd_graph_resource_dependency_chain(f.path(), "missing", false).is_err());
    }

    #[test]
    fn test_fj823_dep_chain_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n");
        assert!(cmd_graph_resource_dependency_chain(f.path(), "base", true).is_ok());
    }

    // FJ-824: apply --notify-teams-webhook (tested via NotifyOpts)
    #[test]
    fn test_fj824_teams_webhook_field() {
        let opts = super::super::dispatch_notify::NotifyOpts {
            slack: None, email: None, webhook: None, teams: None,
            discord: None, opsgenie: None, datadog: None, newrelic: None,
            grafana: None, victorops: None, msteams_adaptive: None,
            incident: None, sns: None, pubsub: None, eventbridge: None,
            kafka: None, azure_servicebus: None, gcp_pubsub_v2: None,
            rabbitmq: None, nats: None, mqtt: None, redis: None,
            amqp: None, stomp: None, zeromq: None, grpc: None,
            sqs: None, mattermost: None, ntfy: None, pagerduty: None,
            discord_webhook: None,
            teams_webhook: Some("https://teams.webhook.office.com/test"), slack_blocks: None, custom_template: None, custom_webhook: None, custom_headers: None, custom_json: None, custom_filter: None, custom_retry: None,
        };
        assert!(opts.teams_webhook.is_some());
    }

    #[test]
    fn test_fj824_teams_webhook_none() {
        let opts = super::super::dispatch_notify::NotifyOpts {
            slack: None, email: None, webhook: None, teams: None,
            discord: None, opsgenie: None, datadog: None, newrelic: None,
            grafana: None, victorops: None, msteams_adaptive: None,
            incident: None, sns: None, pubsub: None, eventbridge: None,
            kafka: None, azure_servicebus: None, gcp_pubsub_v2: None,
            rabbitmq: None, nats: None, mqtt: None, redis: None,
            amqp: None, stomp: None, zeromq: None, grpc: None,
            sqs: None, mattermost: None, ntfy: None, pagerduty: None,
            discord_webhook: None, teams_webhook: None, slack_blocks: None, custom_template: None, custom_webhook: None, custom_headers: None, custom_json: None, custom_filter: None, custom_retry: None,
        };
        assert!(opts.teams_webhook.is_none());
    }

    // FJ-825: validate --check-machine-connectivity
    #[test]
    fn test_fj825_machine_connectivity_valid() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 10.0.0.1\n  m2:\n    hostname: m2\n    addr: example.com\nresources: {}\n");
        assert!(cmd_validate_check_machine_connectivity(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj825_machine_connectivity_sentinel() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: localhost\n  m2:\n    hostname: m2\n    addr: container\nresources: {}\n");
        assert!(cmd_validate_check_machine_connectivity(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj825_machine_connectivity_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 10.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_machine_connectivity(f.path(), true).is_ok());
    }

    // FJ-826: status --fleet-convergence-trend
    #[test]
    fn test_fj826_convergence_trend_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_convergence_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj826_convergence_trend_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_convergence_trend(dir.path(), Some("x"), false).is_ok());
    }

    #[test]
    fn test_fj826_convergence_trend_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_convergence_trend(dir.path(), None, true).is_ok());
    }

    // FJ-827: graph --bottleneck-resources
    #[test]
    fn test_fj827_bottleneck_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  mid:\n    machine: m1\n    type: package\n    name: mid\n    depends_on: [base]\n  top:\n    machine: m1\n    type: package\n    name: top\n    depends_on: [mid]\n");
        assert!(cmd_graph_bottleneck_resources(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj827_bottleneck_no_deps() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg1:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_bottleneck_resources(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj827_bottleneck_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n");
        assert!(cmd_graph_bottleneck_resources(f.path(), true).is_ok());
    }

    // FJ-828: status --resource-state-distribution
    #[test]
    fn test_fj828_state_distribution_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_state_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj828_state_distribution_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_state_distribution(dir.path(), Some("x"), false).is_ok());
    }

    #[test]
    fn test_fj828_state_distribution_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_state_distribution(dir.path(), None, true).is_ok());
    }
}
