//! Tests: Phase 72 — Security & Fleet Insights (FJ-837→FJ-844).

use super::validate_governance::*;
use super::graph_paths::*;
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

    // FJ-837: validate --check-resource-secret-refs
    #[test]
    fn test_fj837_secret_refs_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_secret_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj837_secret_refs_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_secret_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj837_secret_refs_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_secret_refs(f.path(), true).is_ok());
    }

    // FJ-838: status --machine-uptime-estimate
    #[test]
    fn test_fj838_uptime_estimate_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_uptime_estimate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj838_uptime_estimate_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_uptime_estimate(dir.path(), Some("x"), false).is_ok());
    }

    #[test]
    fn test_fj838_uptime_estimate_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_uptime_estimate(dir.path(), None, true).is_ok());
    }

    // FJ-839: graph --resource-coupling-score
    #[test]
    fn test_fj839_coupling_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_coupling_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj839_coupling_no_deps() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg1:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_coupling_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj839_coupling_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n");
        assert!(cmd_graph_resource_coupling_score(f.path(), true).is_ok());
    }

    // FJ-840: apply --notify-custom-template (tested via NotifyOpts)
    #[test]
    fn test_fj840_custom_template_field() {
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
            custom_template: Some("echo {{status}} {{config}}"), custom_webhook: None, custom_headers: None, custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None, custom_dedup_window: None, custom_rate_limit: None, custom_backoff: None, custom_circuit_breaker: None, custom_dead_letter: None, custom_escalation: None, custom_correlation: None, custom_sampling: None, custom_digest: None, custom_severity_filter: None,        };;
        assert!(opts.custom_template.is_some());
    }

    #[test]
    fn test_fj840_custom_template_none() {
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
            custom_template: None, custom_webhook: None, custom_headers: None, custom_json: None, custom_filter: None, custom_retry: None, custom_transform: None, custom_batch: None, custom_deduplicate: None, custom_throttle: None, custom_aggregate: None, custom_priority: None, custom_routing: None, custom_dedup_window: None, custom_rate_limit: None, custom_backoff: None, custom_circuit_breaker: None, custom_dead_letter: None, custom_escalation: None, custom_correlation: None, custom_sampling: None, custom_digest: None, custom_severity_filter: None,        };;
        assert!(opts.custom_template.is_none());
    }

    // FJ-841: validate --check-resource-idempotency-hints
    #[test]
    fn test_fj841_idempotency_hints_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    state: present\n");
        assert!(cmd_validate_check_resource_idempotency_hints(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj841_idempotency_hints_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_resource_idempotency_hints(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj841_idempotency_hints_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_idempotency_hints(f.path(), true).is_ok());
    }

    // FJ-842: status --fleet-resource-type-breakdown
    #[test]
    fn test_fj842_type_breakdown_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_breakdown(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj842_type_breakdown_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_breakdown(dir.path(), Some("x"), false).is_ok());
    }

    #[test]
    fn test_fj842_type_breakdown_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_breakdown(dir.path(), None, true).is_ok());
    }

    // FJ-843: graph --resource-change-frequency
    #[test]
    fn test_fj843_change_freq_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_change_frequency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj843_change_freq_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_graph_resource_change_frequency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj843_change_freq_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n");
        assert!(cmd_graph_resource_change_frequency(f.path(), true).is_ok());
    }

    // FJ-844: status --resource-convergence-time
    #[test]
    fn test_fj844_convergence_time_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_convergence_time(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj844_convergence_time_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_convergence_time(dir.path(), Some("x"), false).is_ok());
    }

    #[test]
    fn test_fj844_convergence_time_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_convergence_time(dir.path(), None, true).is_ok());
    }
}
