//! Coverage tests: dispatch_notify channels, secrets, check, doctor.
use super::check::*;
use super::dispatch_notify::*;
use super::dispatch_notify_custom::*;
use super::doctor::*;
use super::secrets::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

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

    fn all_opts<'a>() -> NotifyOpts<'a> {
        NotifyOpts {
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
            custom_template: Some("echo '{{status}}'"),
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
            custom_severity_filter: Some("http://127.0.0.1:1/csf|info"),
        }
    }

    // All channels — success and failure paths
    #[test]
    fn test_custom_routing() {
        send_custom_routing_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_routing_notification(Some("http://127.0.0.1:1"), &Ok(()), Path::new("/t.yaml"));
        send_custom_routing_notification(
            Some("http://127.0.0.1:1|P:slack"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_dedup_window() {
        send_custom_dedup_window_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_dedup_window_notification(
            Some("http://127.0.0.1:1|120"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_dedup_window_notification(
            Some("http://127.0.0.1:1|60"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_rate_limit() {
        send_custom_rate_limit_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_rate_limit_notification(
            Some("http://127.0.0.1:1|50"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_rate_limit_notification(
            Some("http://127.0.0.1:1|10"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_backoff() {
        send_custom_backoff_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_backoff_notification(
            Some("http://127.0.0.1:1|linear"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_backoff_notification(
            Some("http://127.0.0.1:1|exp"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_circuit_breaker() {
        send_custom_circuit_breaker_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_circuit_breaker_notification(
            Some("http://127.0.0.1:1|10"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_circuit_breaker_notification(
            Some("http://127.0.0.1:1|3"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_dead_letter() {
        send_custom_dead_letter_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_dead_letter_notification(
            Some("http://127.0.0.1:1|dlq"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_dead_letter_notification(
            Some("http://127.0.0.1:1|dlq"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_escalation() {
        send_custom_escalation_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_escalation_notification(
            Some("http://127.0.0.1:1|warn"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_escalation_notification(
            Some("http://127.0.0.1:1|info"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
        send_custom_escalation_notification(
            Some("http://127.0.0.1:1"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_correlation() {
        send_custom_correlation_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_correlation_notification(
            Some("http://127.0.0.1:1|120s"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_correlation_notification(
            Some("http://127.0.0.1:1|30s"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_sampling() {
        send_custom_sampling_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_sampling_notification(
            Some("http://127.0.0.1:1|50"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_sampling_notification(
            Some("http://127.0.0.1:1|25"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_digest() {
        send_custom_digest_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_digest_notification(
            Some("http://127.0.0.1:1|4h"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_digest_notification(
            Some("http://127.0.0.1:1|1h"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_severity_filter() {
        send_custom_severity_filter_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1|info"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1|warn"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1|critical"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1|warn"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1|info"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
}
