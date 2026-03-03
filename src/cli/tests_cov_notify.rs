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
    fn test_all_channels_success() {
        send_apply_notifications(&all_opts(), &Ok(()), Path::new("/tmp/a.yaml"));
    }
    #[test]
    fn test_all_channels_failure() {
        send_apply_notifications(&all_opts(), &Err("fail".into()), Path::new("/tmp/a.yaml"));
    }

    // Individual channel isolation tests — webhook, monitoring, incident
    #[test]
    fn test_individual_webhook_channels() {
        let p = Path::new("/t.yaml");
        for (field, val) in &[
            ("slack", "http://127.0.0.1:1/s"),
            ("webhook", "http://127.0.0.1:1/w"),
            ("teams", "http://127.0.0.1:1/t"),
            ("discord", "http://127.0.0.1:1/d"),
            ("msteams_adaptive", "http://127.0.0.1:1/ms"),
            ("mattermost", "http://127.0.0.1:1/mm"),
            ("ntfy", "topic"),
            ("grafana", "http://127.0.0.1:1/g"),
            ("email", "t@e.com"),
        ] {
            let mut o = empty_opts();
            match *field {
                "slack" => o.slack = Some(val),
                "webhook" => o.webhook = Some(val),
                "teams" => o.teams = Some(val),
                "discord" => o.discord = Some(val),
                "msteams_adaptive" => o.msteams_adaptive = Some(val),
                "mattermost" => o.mattermost = Some(val),
                "ntfy" => o.ntfy = Some(val),
                "grafana" => o.grafana = Some(val),
                "email" => o.email = Some(val),
                _ => {}
            }
            send_apply_notifications(&o, &Ok(()), p);
        }
    }
    #[test]
    fn test_individual_monitoring_channels() {
        let p = Path::new("/t.yaml");
        let mut o = empty_opts();
        o.opsgenie = Some("k");
        send_apply_notifications(&o, &Ok(()), p);
        let mut o = empty_opts();
        o.datadog = Some("k");
        send_apply_notifications(&o, &Ok(()), p);
        let mut o = empty_opts();
        o.newrelic = Some("k");
        send_apply_notifications(&o, &Ok(()), p);
    }
    #[test]
    fn test_individual_incident_channels() {
        let p = Path::new("/t.yaml");
        let ok: Result<(), String> = Ok(());
        let err: Result<(), String> = Err("e".into());
        let mut o = empty_opts();
        o.victorops = Some("k");
        send_apply_notifications(&o, &ok, p);
        send_apply_notifications(&o, &err, p);
        let mut o = empty_opts();
        o.incident = Some("k");
        send_apply_notifications(&o, &ok, p);
        send_apply_notifications(&o, &err, p);
        let mut o = empty_opts();
        o.pagerduty = Some("k");
        send_apply_notifications(&o, &ok, p);
        send_apply_notifications(&o, &err, p);
        let mut o = empty_opts();
        o.discord_webhook = Some("http://127.0.0.1:1/dw");
        send_apply_notifications(&o, &ok, p);
        send_apply_notifications(&o, &err, p);
        let mut o = empty_opts();
        o.teams_webhook = Some("http://127.0.0.1:1/tw");
        send_apply_notifications(&o, &ok, p);
        send_apply_notifications(&o, &err, p);
        let mut o = empty_opts();
        o.slack_blocks = Some("http://127.0.0.1:1/sb");
        send_apply_notifications(&o, &ok, p);
        send_apply_notifications(&o, &err, p);
    }
    #[test]
    fn test_cloud_and_broker_channels() {
        let p = Path::new("/t.yaml");
        let mut o = empty_opts();
        o.sns = Some("arn:t");
        o.eventbridge = Some("b");
        o.sqs = Some("u");
        o.pubsub = Some("t");
        o.gcp_pubsub_v2 = Some("v");
        o.azure_servicebus = Some("c");
        o.kafka = Some("t");
        o.rabbitmq = Some("e");
        o.nats = Some("s");
        o.mqtt = Some("t");
        o.redis = Some("c");
        o.amqp = Some("e");
        o.stomp = Some("/q");
        o.zeromq = Some("tcp://x");
        o.grpc = Some("x:50051");
        send_apply_notifications(&o, &Ok(()), p);
    }
    #[test]
    fn test_custom_json_and_filter_no_pipe() {
        let p = Path::new("/t.yaml");
        let mut o = empty_opts();
        o.custom_json = Some("http://127.0.0.1:1/nopipe");
        send_apply_notifications(&o, &Ok(()), p);
        let mut o = empty_opts();
        o.custom_filter = Some("nopipe");
        send_apply_notifications(&o, &Ok(()), p);
    }

    // Direct dispatch_notify_custom tests
    #[test]
    fn test_custom_transform() {
        send_custom_transform_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_transform_notification(Some("nopipe"), &Ok(()), Path::new("/t.yaml"));
        send_custom_transform_notification(
            Some("http://127.0.0.1:1|{\"s\":\"{{status}}\",\"t\":\"{{timestamp}}\"}"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_transform_notification(
            Some("http://127.0.0.1:1|{\"s\":\"{{status}}\"}"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_batch() {
        send_custom_batch_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_batch_notification(Some("http://127.0.0.1:1"), &Ok(()), Path::new("/t.yaml"));
        send_custom_batch_notification(
            Some("http://127.0.0.1:1|20"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_batch_notification(
            Some("http://127.0.0.1:1|abc"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_batch_notification(
            Some("http://127.0.0.1:1|5"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_deduplicate() {
        send_custom_deduplicate_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_deduplicate_notification(
            Some("http://127.0.0.1:1|600"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_deduplicate_notification(
            Some("http://127.0.0.1:1|60"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_throttle() {
        send_custom_throttle_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_throttle_notification(
            Some("http://127.0.0.1:1"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_throttle_notification(
            Some("http://127.0.0.1:1|max_per_minute:20"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_throttle_notification(
            Some("http://127.0.0.1:1|max_per_minute:abc"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_throttle_notification(
            Some("http://127.0.0.1:1|max_per_minute:10"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_aggregate() {
        send_custom_aggregate_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_aggregate_notification(
            Some("http://127.0.0.1:1"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_aggregate_notification(
            Some("http://127.0.0.1:1|window_seconds:120"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_aggregate_notification(
            Some("http://127.0.0.1:1|window_seconds:xyz"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_aggregate_notification(
            Some("http://127.0.0.1:1|window_seconds:30"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
    #[test]
    fn test_custom_priority() {
        send_custom_priority_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_priority_notification(
            Some("http://127.0.0.1:1"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_priority_notification(
            Some("http://127.0.0.1:1|default:high"),
            &Ok(()),
            Path::new("/t.yaml"),
        );
        send_custom_priority_notification(
            Some("http://127.0.0.1:1|default:low"),
            &Err("e".into()),
            Path::new("/t.yaml"),
        );
    }
}
