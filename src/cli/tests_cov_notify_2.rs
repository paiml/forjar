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
    fn test_notify_all_channels_failure() {
        let opts = NotifyOpts {
            slack: Some("http://127.0.0.1:1/s"),
            email: Some("t@e.com"),
            webhook: Some("http://127.0.0.1:1/w"),
            teams: Some("http://127.0.0.1:1/t"),
            discord: Some("http://127.0.0.1:1/d"),
            opsgenie: Some("k1"),
            datadog: Some("k2"),
            newrelic: Some("k3"),
            grafana: Some("http://127.0.0.1:1/g"),
            victorops: Some("k4"),
            msteams_adaptive: Some("http://127.0.0.1:1/ms"),
            incident: Some("k5"),
            sns: Some("arn:test"),
            pubsub: Some("t/topic"),
            eventbridge: Some("bus"),
            kafka: Some("topic"),
            azure_servicebus: Some("conn"),
            gcp_pubsub_v2: Some("t/v2"),
            rabbitmq: Some("ex"),
            nats: Some("subj"),
            mqtt: Some("t/m"),
            redis: Some("ch"),
            amqp: Some("amqp-ex"),
            stomp: Some("/q/s"),
            zeromq: Some("tcp://x"),
            grpc: Some("x:50051"),
            sqs: Some("https://sqs/q"),
            mattermost: Some("http://127.0.0.1:1/mm"),
            ntfy: Some("topic"),
            pagerduty: Some("k6"),
            discord_webhook: Some("http://127.0.0.1:1/dw"),
            teams_webhook: Some("http://127.0.0.1:1/tw"),
            slack_blocks: Some("http://127.0.0.1:1/sb"),
            custom_template: Some("echo 'done {{status}}'"),
            custom_webhook: Some("http://127.0.0.1:1/cw"),
            custom_headers: Some("http://127.0.0.1:1/ch|X-Key:val"),
            custom_json: Some("http://127.0.0.1:1/cj|{\"s\":\"{{status}}\"}"),
            custom_filter: Some("http://127.0.0.1:1/cf|type:File"),
            custom_retry: Some("http://127.0.0.1:1/cr|retries:1"),
            custom_transform: Some("http://127.0.0.1:1/ct|{\"x\":\"{{status}}\"}"),
            custom_batch: Some("http://127.0.0.1:1/cb|5"),
            custom_deduplicate: Some("http://127.0.0.1:1/cd|120"),
            custom_throttle: Some("http://127.0.0.1:1/cth|max_per_minute:3"),
            custom_aggregate: Some("http://127.0.0.1:1/ca|window_seconds:30"),
            custom_priority: Some("http://127.0.0.1:1/cp|default:low"),
            custom_routing: Some("http://127.0.0.1:1/crt|File:slack"),
            custom_dedup_window: Some("http://127.0.0.1:1/cdw|120"),
            custom_rate_limit: Some("http://127.0.0.1:1/crl|20"),
            custom_backoff: Some("http://127.0.0.1:1/cbk|linear"),
            custom_circuit_breaker: Some("http://127.0.0.1:1/ccb|3"),
            custom_dead_letter: Some("http://127.0.0.1:1/cdl|my-dlq"),
            custom_escalation: Some("http://127.0.0.1:1/ce|warn"),
            custom_correlation: Some("http://127.0.0.1:1/cc|90s"),
            custom_sampling: Some("http://127.0.0.1:1/cs|25"),
            custom_digest: Some("http://127.0.0.1:1/cdi|2h"),
            custom_severity_filter: Some("http://127.0.0.1:1/csf|critical"),
        };
        send_apply_notifications(
            &opts,
            &Err("apply failed".into()),
            Path::new("/tmp/all.yaml"),
        );
    }

    // ─── dispatch_notify.rs — custom_json edge cases ──────────────

    #[test]
    fn test_notify_custom_json_no_pipe_noop() {
        let mut opts = empty_opts();
        // No pipe separator means parts.len() != 2, so it's a no-op
        opts.custom_json = Some("http://127.0.0.1:1/nopipe");
        send_apply_notifications(&opts, &Ok(()), Path::new("/tmp/cov.yaml"));
    }

    #[test]
    fn test_notify_custom_json_failure_result() {
        let mut opts = empty_opts();
        opts.custom_json =
            Some("http://127.0.0.1:1/cj|{\"s\":\"{{status}}\",\"c\":\"{{config}}\"}");
        send_apply_notifications(&opts, &Err("oops".into()), Path::new("/tmp/cov.yaml"));
    }

    // ─── dispatch_notify.rs — custom_filter edge cases ────────────

    #[test]
    fn test_notify_custom_filter_no_pipe_noop() {
        let mut opts = empty_opts();
        opts.custom_filter = Some("nopipe");
        send_apply_notifications(&opts, &Ok(()), Path::new("/tmp/cov.yaml"));
    }

    #[test]
    fn test_notify_custom_filter_failure_result() {
        let mut opts = empty_opts();
        opts.custom_filter = Some("http://127.0.0.1:1/cf|type:File");
        send_apply_notifications(&opts, &Err("oops".into()), Path::new("/tmp/cov.yaml"));
    }

    // ─── dispatch_notify_custom.rs — direct function tests ────────

    #[test]
    fn test_custom_transform_none_noop() {
        send_custom_transform_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_transform_no_pipe_noop() {
        send_custom_transform_notification(Some("nopipe"), &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_transform_success() {
        send_custom_transform_notification(
            Some("http://127.0.0.1:1/ct|{\"s\":\"{{status}}\",\"c\":\"{{config}}\",\"t\":\"{{timestamp}}\"}"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_transform_failure() {
        send_custom_transform_notification(
            Some("http://127.0.0.1:1/ct|{\"s\":\"{{status}}\"}"),
            &Err("err".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_batch_none_noop() {
        send_custom_batch_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_batch_default_size() {
        send_custom_batch_notification(
            Some("http://127.0.0.1:1/b"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_batch_with_size() {
        send_custom_batch_notification(
            Some("http://127.0.0.1:1/b|20"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_batch_failure() {
        send_custom_batch_notification(
            Some("http://127.0.0.1:1/b|5"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_batch_invalid_size_uses_default() {
        send_custom_batch_notification(
            Some("http://127.0.0.1:1/b|abc"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_deduplicate_none_noop() {
        send_custom_deduplicate_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_deduplicate_default_window() {
        send_custom_deduplicate_notification(
            Some("http://127.0.0.1:1/d"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }
}
