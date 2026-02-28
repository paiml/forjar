//! Coverage tests: dispatch_notify channels, secrets, check, doctor.
use std::path::Path;
use super::dispatch_notify::*;
use super::dispatch_notify_custom::*;
use super::secrets::*;
use super::check::*;
use super::doctor::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_opts<'a>() -> NotifyOpts<'a> {
        NotifyOpts {
            slack: None, email: None, webhook: None, teams: None, discord: None,
            opsgenie: None, datadog: None, newrelic: None, grafana: None, victorops: None,
            msteams_adaptive: None, incident: None, sns: None, pubsub: None,
            eventbridge: None, kafka: None, azure_servicebus: None, gcp_pubsub_v2: None,
            rabbitmq: None, nats: None, mqtt: None, redis: None, amqp: None, stomp: None,
            zeromq: None, grpc: None, sqs: None, mattermost: None, ntfy: None,
            pagerduty: None, discord_webhook: None, teams_webhook: None, slack_blocks: None,
            custom_template: None, custom_webhook: None, custom_headers: None,
            custom_json: None, custom_filter: None, custom_retry: None,
            custom_transform: None, custom_batch: None, custom_deduplicate: None,
            custom_throttle: None, custom_aggregate: None, custom_priority: None,
            custom_routing: None, custom_dedup_window: None, custom_rate_limit: None,
            custom_backoff: None, custom_circuit_breaker: None, custom_dead_letter: None,
            custom_escalation: None, custom_correlation: None, custom_sampling: None,
            custom_digest: None, custom_severity_filter: None,
        }
    }

    fn all_opts<'a>() -> NotifyOpts<'a> {
        NotifyOpts {
            slack: Some("http://127.0.0.1:1/s"), email: Some("t@e.com"),
            webhook: Some("http://127.0.0.1:1/w"), teams: Some("http://127.0.0.1:1/t"),
            discord: Some("http://127.0.0.1:1/d"), opsgenie: Some("k1"),
            datadog: Some("k2"), newrelic: Some("k3"),
            grafana: Some("http://127.0.0.1:1/g"), victorops: Some("k4"),
            msteams_adaptive: Some("http://127.0.0.1:1/ms"), incident: Some("k5"),
            sns: Some("arn:test"), pubsub: Some("t/topic"), eventbridge: Some("bus"),
            kafka: Some("topic"), azure_servicebus: Some("conn"),
            gcp_pubsub_v2: Some("t/v2"), rabbitmq: Some("ex"), nats: Some("subj"),
            mqtt: Some("t/m"), redis: Some("ch"), amqp: Some("amqp-ex"),
            stomp: Some("/q/s"), zeromq: Some("tcp://x"), grpc: Some("x:50051"),
            sqs: Some("https://sqs/q"), mattermost: Some("http://127.0.0.1:1/mm"),
            ntfy: Some("topic"), pagerduty: Some("k6"),
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
        for (field, val) in &[("slack", "http://127.0.0.1:1/s"), ("webhook", "http://127.0.0.1:1/w"),
            ("teams", "http://127.0.0.1:1/t"), ("discord", "http://127.0.0.1:1/d"),
            ("msteams_adaptive", "http://127.0.0.1:1/ms"), ("mattermost", "http://127.0.0.1:1/mm"),
            ("ntfy", "topic"), ("grafana", "http://127.0.0.1:1/g"), ("email", "t@e.com")] {
            let mut o = empty_opts();
            match *field {
                "slack" => o.slack = Some(val), "webhook" => o.webhook = Some(val),
                "teams" => o.teams = Some(val), "discord" => o.discord = Some(val),
                "msteams_adaptive" => o.msteams_adaptive = Some(val),
                "mattermost" => o.mattermost = Some(val), "ntfy" => o.ntfy = Some(val),
                "grafana" => o.grafana = Some(val), "email" => o.email = Some(val),
                _ => {}
            }
            send_apply_notifications(&o, &Ok(()), p);
        }
    }
    #[test]
    fn test_individual_monitoring_channels() {
        let p = Path::new("/t.yaml");
        let mut o = empty_opts(); o.opsgenie = Some("k");
        send_apply_notifications(&o, &Ok(()), p);
        let mut o = empty_opts(); o.datadog = Some("k");
        send_apply_notifications(&o, &Ok(()), p);
        let mut o = empty_opts(); o.newrelic = Some("k");
        send_apply_notifications(&o, &Ok(()), p);
    }
    #[test]
    fn test_individual_incident_channels() {
        let p = Path::new("/t.yaml");
        let ok: Result<(), String> = Ok(()); let err: Result<(), String> = Err("e".into());
        let mut o = empty_opts(); o.victorops = Some("k");
        send_apply_notifications(&o, &ok, p); send_apply_notifications(&o, &err, p);
        let mut o = empty_opts(); o.incident = Some("k");
        send_apply_notifications(&o, &ok, p); send_apply_notifications(&o, &err, p);
        let mut o = empty_opts(); o.pagerduty = Some("k");
        send_apply_notifications(&o, &ok, p); send_apply_notifications(&o, &err, p);
        let mut o = empty_opts(); o.discord_webhook = Some("http://127.0.0.1:1/dw");
        send_apply_notifications(&o, &ok, p); send_apply_notifications(&o, &err, p);
        let mut o = empty_opts(); o.teams_webhook = Some("http://127.0.0.1:1/tw");
        send_apply_notifications(&o, &ok, p); send_apply_notifications(&o, &err, p);
        let mut o = empty_opts(); o.slack_blocks = Some("http://127.0.0.1:1/sb");
        send_apply_notifications(&o, &ok, p); send_apply_notifications(&o, &err, p);
    }
    #[test]
    fn test_cloud_and_broker_channels() {
        let p = Path::new("/t.yaml");
        let mut o = empty_opts();
        o.sns = Some("arn:t"); o.eventbridge = Some("b"); o.sqs = Some("u");
        o.pubsub = Some("t"); o.gcp_pubsub_v2 = Some("v"); o.azure_servicebus = Some("c");
        o.kafka = Some("t"); o.rabbitmq = Some("e"); o.nats = Some("s");
        o.mqtt = Some("t"); o.redis = Some("c"); o.amqp = Some("e");
        o.stomp = Some("/q"); o.zeromq = Some("tcp://x"); o.grpc = Some("x:50051");
        send_apply_notifications(&o, &Ok(()), p);
    }
    #[test]
    fn test_custom_json_and_filter_no_pipe() {
        let p = Path::new("/t.yaml");
        let mut o = empty_opts(); o.custom_json = Some("http://127.0.0.1:1/nopipe");
        send_apply_notifications(&o, &Ok(()), p);
        let mut o = empty_opts(); o.custom_filter = Some("nopipe");
        send_apply_notifications(&o, &Ok(()), p);
    }

    // Direct dispatch_notify_custom tests
    #[test]
    fn test_custom_transform() {
        send_custom_transform_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_transform_notification(Some("nopipe"), &Ok(()), Path::new("/t.yaml"));
        send_custom_transform_notification(Some("http://127.0.0.1:1|{\"s\":\"{{status}}\",\"t\":\"{{timestamp}}\"}"), &Ok(()), Path::new("/t.yaml"));
        send_custom_transform_notification(Some("http://127.0.0.1:1|{\"s\":\"{{status}}\"}"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_batch() {
        send_custom_batch_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_batch_notification(Some("http://127.0.0.1:1"), &Ok(()), Path::new("/t.yaml"));
        send_custom_batch_notification(Some("http://127.0.0.1:1|20"), &Ok(()), Path::new("/t.yaml"));
        send_custom_batch_notification(Some("http://127.0.0.1:1|abc"), &Ok(()), Path::new("/t.yaml"));
        send_custom_batch_notification(Some("http://127.0.0.1:1|5"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_deduplicate() {
        send_custom_deduplicate_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_deduplicate_notification(Some("http://127.0.0.1:1|600"), &Ok(()), Path::new("/t.yaml"));
        send_custom_deduplicate_notification(Some("http://127.0.0.1:1|60"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_throttle() {
        send_custom_throttle_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_throttle_notification(Some("http://127.0.0.1:1"), &Ok(()), Path::new("/t.yaml"));
        send_custom_throttle_notification(Some("http://127.0.0.1:1|max_per_minute:20"), &Ok(()), Path::new("/t.yaml"));
        send_custom_throttle_notification(Some("http://127.0.0.1:1|max_per_minute:abc"), &Ok(()), Path::new("/t.yaml"));
        send_custom_throttle_notification(Some("http://127.0.0.1:1|max_per_minute:10"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_aggregate() {
        send_custom_aggregate_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_aggregate_notification(Some("http://127.0.0.1:1"), &Ok(()), Path::new("/t.yaml"));
        send_custom_aggregate_notification(Some("http://127.0.0.1:1|window_seconds:120"), &Ok(()), Path::new("/t.yaml"));
        send_custom_aggregate_notification(Some("http://127.0.0.1:1|window_seconds:xyz"), &Ok(()), Path::new("/t.yaml"));
        send_custom_aggregate_notification(Some("http://127.0.0.1:1|window_seconds:30"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_priority() {
        send_custom_priority_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_priority_notification(Some("http://127.0.0.1:1"), &Ok(()), Path::new("/t.yaml"));
        send_custom_priority_notification(Some("http://127.0.0.1:1|default:high"), &Ok(()), Path::new("/t.yaml"));
        send_custom_priority_notification(Some("http://127.0.0.1:1|default:low"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_routing() {
        send_custom_routing_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_routing_notification(Some("http://127.0.0.1:1"), &Ok(()), Path::new("/t.yaml"));
        send_custom_routing_notification(Some("http://127.0.0.1:1|P:slack"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_dedup_window() {
        send_custom_dedup_window_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_dedup_window_notification(Some("http://127.0.0.1:1|120"), &Ok(()), Path::new("/t.yaml"));
        send_custom_dedup_window_notification(Some("http://127.0.0.1:1|60"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_rate_limit() {
        send_custom_rate_limit_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_rate_limit_notification(Some("http://127.0.0.1:1|50"), &Ok(()), Path::new("/t.yaml"));
        send_custom_rate_limit_notification(Some("http://127.0.0.1:1|10"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_backoff() {
        send_custom_backoff_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_backoff_notification(Some("http://127.0.0.1:1|linear"), &Ok(()), Path::new("/t.yaml"));
        send_custom_backoff_notification(Some("http://127.0.0.1:1|exp"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_circuit_breaker() {
        send_custom_circuit_breaker_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_circuit_breaker_notification(Some("http://127.0.0.1:1|10"), &Ok(()), Path::new("/t.yaml"));
        send_custom_circuit_breaker_notification(Some("http://127.0.0.1:1|3"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_dead_letter() {
        send_custom_dead_letter_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_dead_letter_notification(Some("http://127.0.0.1:1|dlq"), &Ok(()), Path::new("/t.yaml"));
        send_custom_dead_letter_notification(Some("http://127.0.0.1:1|dlq"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_escalation() {
        send_custom_escalation_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_escalation_notification(Some("http://127.0.0.1:1|warn"), &Ok(()), Path::new("/t.yaml"));
        send_custom_escalation_notification(Some("http://127.0.0.1:1|info"), &Err("e".into()), Path::new("/t.yaml"));
        send_custom_escalation_notification(Some("http://127.0.0.1:1"), &Ok(()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_correlation() {
        send_custom_correlation_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_correlation_notification(Some("http://127.0.0.1:1|120s"), &Ok(()), Path::new("/t.yaml"));
        send_custom_correlation_notification(Some("http://127.0.0.1:1|30s"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_sampling() {
        send_custom_sampling_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_sampling_notification(Some("http://127.0.0.1:1|50"), &Ok(()), Path::new("/t.yaml"));
        send_custom_sampling_notification(Some("http://127.0.0.1:1|25"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_digest() {
        send_custom_digest_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_digest_notification(Some("http://127.0.0.1:1|4h"), &Ok(()), Path::new("/t.yaml"));
        send_custom_digest_notification(Some("http://127.0.0.1:1|1h"), &Err("e".into()), Path::new("/t.yaml"));
    }
    #[test]
    fn test_custom_severity_filter() {
        send_custom_severity_filter_notification(None, &Ok(()), Path::new("/t.yaml"));
        send_custom_severity_filter_notification(Some("http://127.0.0.1:1|info"), &Ok(()), Path::new("/t.yaml"));
        send_custom_severity_filter_notification(Some("http://127.0.0.1:1|warn"), &Ok(()), Path::new("/t.yaml"));
        send_custom_severity_filter_notification(Some("http://127.0.0.1:1|critical"), &Err("e".into()), Path::new("/t.yaml"));
        send_custom_severity_filter_notification(Some("http://127.0.0.1:1|warn"), &Err("e".into()), Path::new("/t.yaml"));
        send_custom_severity_filter_notification(Some("http://127.0.0.1:1|info"), &Err("e".into()), Path::new("/t.yaml"));
        send_custom_severity_filter_notification(Some("http://127.0.0.1:1"), &Err("e".into()), Path::new("/t.yaml"));
    }

    // secrets.rs
    #[test]
    fn test_find_enc_markers() {
        assert!(find_enc_markers("").is_empty());
        assert!(find_enc_markers("no markers").is_empty());
        assert_eq!(find_enc_markers("ENC[age,abc]").len(), 1);
        assert_eq!(find_enc_markers("a: ENC[age,x] b: ENC[age,y]").len(), 2);
        assert!(find_enc_markers("ENC[age,no_closing").is_empty());
        assert_eq!(find_enc_markers("ENC[age,a]ENC[age,b]").len(), 2);
    }
    #[test]
    fn test_secrets_keygen() { assert!(cmd_secrets_keygen().is_ok()); }
    #[test]
    fn test_secrets_view() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("p.yaml");
        std::fs::write(&f, "k: v\n").unwrap();
        assert!(cmd_secrets_view(&f, None).is_ok());
        assert!(cmd_secrets_view(Path::new("/tmp/ne-12345.yaml"), None).is_err());
        let f2 = d.path().join("e.yaml");
        std::fs::write(&f2, "pw: ENC[age,fake]\n").unwrap();
        let _ = cmd_secrets_view(&f2, None);
    }
    #[test]
    fn test_secrets_encrypt() {
        assert!(cmd_secrets_encrypt("val", &["bad".into()]).is_err());
        assert!(cmd_secrets_encrypt("val", &[]).is_err());
    }
    #[test]
    fn test_secrets_decrypt() {
        assert!(cmd_secrets_decrypt("not_valid", None).is_err());
        assert!(cmd_secrets_decrypt("ENC[age,!!!]", None).is_err());
    }
    #[test]
    fn test_secrets_rekey() {
        assert!(cmd_secrets_rekey(Path::new("/tmp/ne-12345.yaml"), None, &["a".into()]).is_err());
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("p.yaml");
        std::fs::write(&f, "k: v\n").unwrap();
        assert!(cmd_secrets_rekey(&f, None, &["a".into()]).is_ok());
    }
    #[test]
    fn test_secrets_rotate() {
        let d = tempfile::tempdir().unwrap();
        let sd = d.path().join("state");
        let f = d.path().join("r.yaml");
        std::fs::write(&f, "v: ENC[age,d]\n").unwrap();
        assert!(cmd_secrets_rotate(&f, None, &["a".into()], false, &sd).is_err());
        assert!(cmd_secrets_rotate(Path::new("/tmp/ne-12345.yaml"), None, &["a".into()], true, &sd).is_err());
        let f2 = d.path().join("p.yaml");
        std::fs::write(&f2, "k: v\n").unwrap();
        assert!(cmd_secrets_rotate(&f2, None, &["a".into()], true, &sd).is_ok());
    }

    // check.rs
    fn check_cfg(d: &std::path::Path) -> std::path::PathBuf {
        let f = d.join("forjar.yaml");
        std::fs::write(&f, r#"version: "1.0"
name: ck
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  pkg1:
    type: package
    machine: local
    name: coreutils
    tags: [base, system]
  cfg1:
    type: file
    machine: local
    path: /tmp/forjar-ck-test.txt
    content: "hello"
    tags: [config]
"#).unwrap();
        f
    }
    #[test]
    fn test_check_filters() {
        let d = tempfile::tempdir().unwrap();
        let c = check_cfg(d.path());
        let _ = cmd_check(&c, None, None, Some("config"), false, false);
        let _ = cmd_check(&c, None, None, Some("config"), true, false);
        let _ = cmd_check(&c, None, Some("pkg1"), None, false, false);
        let _ = cmd_check(&c, None, Some("nonexistent"), None, false, false);
        let _ = cmd_check(&c, Some("local"), None, None, false, false);
        let _ = cmd_check(&c, Some("nonexistent"), None, None, false, false);
        let _ = cmd_check(&c, None, None, None, false, true);
        let _ = cmd_check(&c, None, None, None, true, false);
        let _ = cmd_check(&c, Some("local"), Some("pkg1"), Some("base"), false, false);
    }
    #[test]
    fn test_check_invalid() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("bad.yaml");
        std::fs::write(&f, "invalid: [[[").unwrap();
        assert!(cmd_check(&f, None, None, None, false, false).is_err());
    }
    #[test]
    fn test_cmd_test_variants() {
        let d = tempfile::tempdir().unwrap();
        let c = check_cfg(d.path());
        let _ = cmd_test(&c, None, None, None, None, false, false);
        let _ = cmd_test(&c, None, None, None, None, true, false);
        let _ = cmd_test(&c, None, None, None, None, false, true);
        let _ = cmd_test(&c, None, None, Some("base"), None, false, false);
        let _ = cmd_test(&c, None, Some("cfg1"), None, None, false, false);
        let _ = cmd_test(&c, Some("local"), None, None, None, false, false);
        let _ = cmd_test(&c, None, None, None, Some("web"), false, false);
        let _ = cmd_test(&c, None, None, None, Some("web"), true, false);
        let _ = cmd_test(&c, None, None, None, None, true, true);
    }
    #[test]
    fn test_cmd_test_invalid() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("bad.yaml");
        std::fs::write(&f, "invalid: [[[").unwrap();
        assert!(cmd_test(&f, None, None, None, None, false, false).is_err());
    }

    // doctor.rs
    #[test]
    fn test_doctor_fix() {
        assert!(cmd_doctor(None, false, true).is_ok());
        assert!(cmd_doctor(None, true, true).is_ok());
    }
    #[test]
    fn test_doctor_enc_markers() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(&f, r#"version: "1.0"
name: enc
machines:
  local: { hostname: localhost, addr: 127.0.0.1 }
resources:
  s: { type: file, machine: local, path: /tmp/s.txt, content: "ENC[age,fake]" }
"#).unwrap();
        let _ = cmd_doctor(Some(&f), false, false);
        let _ = cmd_doctor(Some(&f), true, false);
    }
    #[test]
    fn test_doctor_container() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(&f, r#"version: "1.0"
name: ctr
machines:
  ctr: { hostname: ctr, addr: container, transport: container, container: { image: alpine:3.18, runtime: podman } }
resources:
  f: { type: file, machine: ctr, path: /tmp/t, content: "x" }
"#).unwrap();
        let _ = cmd_doctor(Some(&f), true, false);
        let f2 = d.path().join("forjar2.yaml");
        std::fs::write(&f2, r#"version: "1.0"
name: ctr
machines:
  ctr: { hostname: ctr, addr: container, transport: container }
resources:
  f: { type: file, machine: ctr, path: /tmp/t, content: "x" }
"#).unwrap();
        let _ = cmd_doctor(Some(&f2), false, false);
    }
    #[test]
    fn test_doctor_network() {
        let _ = cmd_doctor_network(None, false);
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(&f, r#"version: "1.0"
name: n
machines:
  lo: { hostname: lo, addr: 127.0.0.1 }
resources:
  f: { type: file, machine: lo, path: /tmp/t, content: "x" }
"#).unwrap();
        assert!(cmd_doctor_network(Some(&f), false).is_ok());
        assert!(cmd_doctor_network(Some(&f), true).is_ok());
    }
    #[test]
    fn test_doctor_network_ssh_key() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(&f, r#"version: "1.0"
name: n
machines:
  r: { hostname: r, addr: 10.0.0.99, user: deploy, ssh_key: /nonexistent/key.pem }
resources:
  f: { type: file, machine: r, path: /tmp/t, content: "x" }
"#).unwrap();
        assert!(cmd_doctor_network(Some(&f), false).is_ok());
        assert!(cmd_doctor_network(Some(&f), true).is_ok());
    }
    #[test]
    fn test_doctor_network_localhost() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(&f, r#"version: "1.0"
name: n
machines:
  lo: { hostname: lo, addr: localhost }
resources:
  f: { type: file, machine: lo, path: /tmp/t, content: "x" }
"#).unwrap();
        assert!(cmd_doctor_network(Some(&f), false).is_ok());
    }
    #[test]
    fn test_doctor_network_invalid() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(&f, "invalid: [[[").unwrap();
        assert!(cmd_doctor_network(Some(&f), false).is_err());
    }
}
