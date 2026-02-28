//! Tests: Phase 84 — Compliance Analytics & Infrastructure Forecasting (FJ-933→FJ-940).

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

    // FJ-933: validate --check-resource-naming-pattern
    #[test]
    fn test_fj933_naming_pattern_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  nginx-pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_naming_standards(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj933_naming_pattern_violation() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  Nginx__pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_naming_standards(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj933_naming_pattern_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_naming_standards(f.path(), true).is_ok());
    }

    // FJ-934: status --machine-resource-convergence-velocity
    #[test]
    fn test_fj934_convergence_velocity_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj934_convergence_velocity_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_convergence_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj934_convergence_velocity_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_velocity(dir.path(), None, true).is_ok());
    }

    // FJ-935: graph --resource-dependency-density
    #[test]
    fn test_fj935_density_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_density(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj935_density_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_density(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj935_density_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_density(f.path(), true).is_ok());
    }

    // FJ-936: apply --notify-custom-routing (tested via NotifyOpts)
    #[test]
    fn test_fj936_custom_routing_field() {
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
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None,
            custom_routing: Some("https://hooks.example.com|Package:slack,File:email"),
            custom_dedup_window: None, custom_rate_limit: None, custom_backoff: None,        };
        assert!(opts.custom_routing.is_some());
    }

    #[test]
    fn test_fj936_custom_routing_none() {
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
            custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None, custom_dedup_window: None, custom_rate_limit: None, custom_backoff: None,        };
        assert!(opts.custom_routing.is_none());
    }

    // FJ-937: validate --check-resource-dependency-symmetry
    #[test]
    fn test_fj937_dependency_symmetry_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_validate_check_resource_dependency_symmetry(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj937_dependency_symmetry_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_symmetry(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj937_dependency_symmetry_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_dependency_symmetry(f.path(), true).is_ok());
    }

    // FJ-938: status --fleet-resource-convergence-velocity
    #[test]
    fn test_fj938_fleet_convergence_velocity_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj938_fleet_convergence_velocity_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n");
        assert!(cmd_status_fleet_resource_convergence_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj938_fleet_convergence_velocity_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_velocity(dir.path(), None, true).is_ok());
    }

    // FJ-939: graph --resource-dependency-transitivity
    #[test]
    fn test_fj939_transitivity_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_dependency_transitivity(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj939_transitivity_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_transitivity(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj939_transitivity_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_dependency_transitivity(f.path(), true).is_ok());
    }

    // FJ-940: status --machine-resource-failure-recurrence
    #[test]
    fn test_fj940_failure_recurrence_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_failure_recurrence(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj940_failure_recurrence_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n");
        assert!(cmd_status_machine_resource_failure_recurrence(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj940_failure_recurrence_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_failure_recurrence(dir.path(), None, true).is_ok());
    }
}
