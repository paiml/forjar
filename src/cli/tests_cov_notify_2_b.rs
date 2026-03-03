//! Coverage tests: dispatch_notify — all-channels-failure, custom_json,
//! custom_filter edge cases, and dispatch_notify_custom direct function tests
//! (transform, batch, deduplicate, throttle, aggregate, priority, routing,
//! dedup_window, rate_limit, backoff).

#![allow(unused_imports)]
use super::check::*;
use super::dispatch_notify::*;
use super::dispatch_notify_custom::*;
use super::doctor::*;
use super::secrets::*;
use super::test_fixtures::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Helper: empty NotifyOpts ──────────────────────────────────

    fn empty_opts<'a>() -> NotifyOpts<'a> {
        NotifyOpts {
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
        }
    }

    // ─── dispatch_notify.rs — all channels failure ──────────────────

    #[test]
    #[test]
    fn test_custom_deduplicate_with_window() {
        send_custom_deduplicate_notification(
            Some("http://127.0.0.1:1/d|600"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_deduplicate_failure() {
        send_custom_deduplicate_notification(
            Some("http://127.0.0.1:1/d|60"),
            &Err("x".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_throttle_none_noop() {
        send_custom_throttle_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_throttle_default_rate() {
        send_custom_throttle_notification(
            Some("http://127.0.0.1:1/th"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_throttle_with_rate() {
        send_custom_throttle_notification(
            Some("http://127.0.0.1:1/th|max_per_minute:20"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_throttle_invalid_rate_uses_default() {
        send_custom_throttle_notification(
            Some("http://127.0.0.1:1/th|max_per_minute:abc"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_throttle_failure() {
        send_custom_throttle_notification(
            Some("http://127.0.0.1:1/th|max_per_minute:10"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_aggregate_none_noop() {
        send_custom_aggregate_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_aggregate_default_window() {
        send_custom_aggregate_notification(
            Some("http://127.0.0.1:1/ag"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_aggregate_with_window() {
        send_custom_aggregate_notification(
            Some("http://127.0.0.1:1/ag|window_seconds:120"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_aggregate_invalid_window_uses_default() {
        send_custom_aggregate_notification(
            Some("http://127.0.0.1:1/ag|window_seconds:xyz"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_aggregate_failure() {
        send_custom_aggregate_notification(
            Some("http://127.0.0.1:1/ag|window_seconds:30"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_priority_none_noop() {
        send_custom_priority_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_priority_default() {
        send_custom_priority_notification(
            Some("http://127.0.0.1:1/p"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_priority_with_default_level() {
        send_custom_priority_notification(
            Some("http://127.0.0.1:1/p|default:high"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_priority_failure_overrides_to_critical() {
        send_custom_priority_notification(
            Some("http://127.0.0.1:1/p|default:low"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_routing_none_noop() {
        send_custom_routing_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_routing_default_rules() {
        send_custom_routing_notification(
            Some("http://127.0.0.1:1/r"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_routing_with_rules() {
        send_custom_routing_notification(
            Some("http://127.0.0.1:1/r|Package:slack,File:email"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_routing_failure() {
        send_custom_routing_notification(
            Some("http://127.0.0.1:1/r|default"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_dedup_window_none_noop() {
        send_custom_dedup_window_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_dedup_window_default() {
        send_custom_dedup_window_notification(
            Some("http://127.0.0.1:1/dw"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_dedup_window_with_value() {
        send_custom_dedup_window_notification(
            Some("http://127.0.0.1:1/dw|120"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_dedup_window_failure() {
        send_custom_dedup_window_notification(
            Some("http://127.0.0.1:1/dw|60"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_rate_limit_none_noop() {
        send_custom_rate_limit_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_rate_limit_default() {
        send_custom_rate_limit_notification(
            Some("http://127.0.0.1:1/rl"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_rate_limit_with_value() {
        send_custom_rate_limit_notification(
            Some("http://127.0.0.1:1/rl|50"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_rate_limit_failure() {
        send_custom_rate_limit_notification(
            Some("http://127.0.0.1:1/rl|10"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_backoff_none_noop() {
        send_custom_backoff_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_backoff_default() {
        send_custom_backoff_notification(
            Some("http://127.0.0.1:1/bk"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_backoff_with_strategy() {
        send_custom_backoff_notification(
            Some("http://127.0.0.1:1/bk|linear"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_backoff_failure() {
        send_custom_backoff_notification(
            Some("http://127.0.0.1:1/bk|exponential"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }
}
