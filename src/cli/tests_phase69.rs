//! Tests: Phase 69 — Operational Insights & Governance (FJ-813→FJ-820).

#![allow(unused_imports)]
use super::graph_advanced::*;
use super::status_operational::*;
use super::validate_advanced::*;
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

    // FJ-813: validate --check-resource-tags
    #[test]
    fn test_fj813_check_resource_tags_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg1:\n    machine: m1\n    type: package\n    name: nginx\n    tags: [web, server]\n");
        assert!(cmd_validate_check_resource_tags(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj813_check_resource_tags_empty() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg1:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_tags(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj813_check_resource_tags_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg1:\n    machine: m1\n    type: package\n    name: nginx\n    tags: [web]\n");
        assert!(cmd_validate_check_resource_tags(f.path(), true).is_ok());
    }

    // FJ-814: status --machine-last-apply
    #[test]
    fn test_fj814_machine_last_apply_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_last_apply(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj814_machine_last_apply_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_last_apply(dir.path(), Some("missing"), false).is_ok());
    }

    #[test]
    fn test_fj814_machine_last_apply_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_last_apply(dir.path(), None, true).is_ok());
    }

    // FJ-815: graph --resource-fanin
    #[test]
    fn test_fj815_resource_fanin_basic() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_resource_fanin(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj815_resource_fanin_no_deps() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg1:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_graph_resource_fanin(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj815_resource_fanin_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n");
        assert!(cmd_graph_resource_fanin(f.path(), true).is_ok());
    }

    // FJ-816: apply --notify-discord-webhook (tested via NotifyOpts struct)
    #[test]
    fn test_fj816_discord_webhook_field() {
        // Verify the field exists on NotifyOpts by constructing it
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
            discord_webhook: Some("https://discord.com/api/webhooks/test"),
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
        assert!(opts.discord_webhook.is_some());
    }

    #[test]
    fn test_fj816_discord_webhook_none() {
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
        assert!(opts.discord_webhook.is_none());
    }

    // FJ-817: validate --check-resource-state-consistency
    #[test]
    fn test_fj817_state_consistency_clean() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  svc1:\n    machine: m1\n    type: service\n    name: nginx\n    state: running\n");
        assert!(cmd_validate_check_resource_state_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj817_state_consistency_no_state() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg1:\n    machine: m1\n    type: package\n    name: nginx\n");
        assert!(cmd_validate_check_resource_state_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj817_state_consistency_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg1:\n    machine: m1\n    type: package\n    name: nginx\n    state: present\n");
        assert!(cmd_validate_check_resource_state_consistency(f.path(), true).is_ok());
    }

    // FJ-818: status --fleet-drift-summary
    #[test]
    fn test_fj818_fleet_drift_summary_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_drift_summary(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj818_fleet_drift_summary_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_drift_summary(dir.path(), Some("missing"), false).is_ok());
    }

    #[test]
    fn test_fj818_fleet_drift_summary_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_drift_summary(dir.path(), None, true).is_ok());
    }

    // FJ-819: graph --isolated-subgraphs
    #[test]
    fn test_fj819_isolated_subgraphs_connected() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  base:\n    machine: m1\n    type: package\n    name: base\n  app:\n    machine: m1\n    type: package\n    name: app\n    depends_on: [base]\n");
        assert!(cmd_graph_isolated_subgraphs(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj819_isolated_subgraphs_disconnected() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  a:\n    machine: m1\n    type: package\n    name: a\n  b:\n    machine: m1\n    type: package\n    name: b\n");
        assert!(cmd_graph_isolated_subgraphs(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj819_isolated_subgraphs_json() {
        let f = write_temp_config("version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  a:\n    machine: m1\n    type: package\n    name: a\n");
        assert!(cmd_graph_isolated_subgraphs(f.path(), true).is_ok());
    }

    // FJ-820: status --resource-apply-duration
    #[test]
    fn test_fj820_resource_apply_duration_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_apply_duration(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj820_resource_apply_duration_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_apply_duration(dir.path(), Some("missing"), false).is_ok());
    }

    #[test]
    fn test_fj820_resource_apply_duration_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_apply_duration(dir.path(), None, true).is_ok());
    }
}
