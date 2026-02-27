//! Apply notification dispatch helpers.
//!
//! Sends apply results to various notification channels (Slack, Teams, etc.).

use std::path::Path;


/// POST JSON to a webhook URL via curl.
fn send_webhook(url: &str, payload: &str) {
    let _ = std::process::Command::new("curl")
        .args(["-s", "-X", "POST", "-H", "Content-Type: application/json", "-d", payload, url])
        .output();
}

/// POST JSON to a webhook URL with an extra auth header.
fn send_webhook_with_header(url: &str, header: &str, payload: &str) {
    let _ = std::process::Command::new("curl")
        .args(["-s", "-X", "POST", "-H", "Content-Type: application/json", "-H", header, "-d", payload, url])
        .output();
}

/// Simple JSON event payload for messaging systems.
fn event_json(status: &str, config: &Path) -> String {
    format!(r#"{{"event":"forjar_apply","status":"{}","config":"{}"}}"#, status, config.display())
}

/// Publish a message to a CLI-based messaging system (stdin pipe).
fn publish_stdin(cmd: &str, args: &[&str], message: &str) {
    let child = std::process::Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .spawn();
    if let Ok(mut c) = child {
        if let Some(ref mut stdin) = c.stdin {
            use std::io::Write;
            let _ = stdin.write_all(message.as_bytes());
        }
        let _ = c.wait();
    }
}


/// Configuration for all notification channels on an apply.
pub(crate) struct NotifyOpts<'a> {
    pub slack: Option<&'a str>,
    pub email: Option<&'a str>,
    pub webhook: Option<&'a str>,
    pub teams: Option<&'a str>,
    pub discord: Option<&'a str>,
    pub opsgenie: Option<&'a str>,
    pub datadog: Option<&'a str>,
    pub newrelic: Option<&'a str>,
    pub grafana: Option<&'a str>,
    pub victorops: Option<&'a str>,
    pub msteams_adaptive: Option<&'a str>,
    pub incident: Option<&'a str>,
    pub sns: Option<&'a str>,
    pub pubsub: Option<&'a str>,
    pub eventbridge: Option<&'a str>,
    pub kafka: Option<&'a str>,
    pub azure_servicebus: Option<&'a str>,
    pub gcp_pubsub_v2: Option<&'a str>,
    pub rabbitmq: Option<&'a str>,
    pub nats: Option<&'a str>,
    pub mqtt: Option<&'a str>,
    pub redis: Option<&'a str>,
    pub amqp: Option<&'a str>,
    pub stomp: Option<&'a str>,
    pub zeromq: Option<&'a str>,
    pub grpc: Option<&'a str>,
    pub sqs: Option<&'a str>,
    pub mattermost: Option<&'a str>,
    pub ntfy: Option<&'a str>,
}


/// Send notifications for an apply result to all configured channels.
pub(crate) fn send_apply_notifications(
    opts: &NotifyOpts<'_>,
    result: &Result<(), String>,
    config_path: &Path,
) {
    let status = if result.is_ok() { "success" } else { "failure" };
    let msg = event_json(status, config_path);

    send_webhook_notifications(opts, status, config_path);
    send_monitoring_notifications(opts, status, config_path);
    send_incident_notifications(opts, result, config_path);
    send_email_notification(opts.email, status, config_path);
    send_cloud_notifications(opts, &msg);
    send_broker_notifications(opts, &msg);
}


/// Send to webhook-based services (Slack, Teams, Discord, etc.).
fn send_webhook_notifications(opts: &NotifyOpts<'_>, status: &str, config: &Path) {
    if let Some(url) = opts.slack {
        send_webhook(url, &format!(r#"{{"text":"forjar apply {}: {}"}}"#, status, config.display()));
    }
    if let Some(url) = opts.webhook {
        send_webhook(url, &format!(r#"{{"event":"apply_complete","status":"{}","config":"{}"}}"#, status, config.display()));
    }
    if let Some(url) = opts.teams {
        send_webhook(url, &format!(r#"{{"@type":"MessageCard","summary":"forjar apply {}","text":"Apply {} for {}"}}"#, status, status, config.display()));
    }
    if let Some(url) = opts.discord {
        send_webhook(url, &format!(r#"{{"content":"forjar apply {}: {}"}}"#, status, config.display()));
    }
    if let Some(url) = opts.msteams_adaptive {
        send_webhook(url, &format!(
            r#"{{"type":"message","attachments":[{{"contentType":"application/vnd.microsoft.card.adaptive","content":{{"type":"AdaptiveCard","body":[{{"type":"TextBlock","text":"Forjar Apply: {}","weight":"bolder"}},{{"type":"TextBlock","text":"Config: {}"}}],"$schema":"http://adaptivecards.io/schemas/adaptive-card.json","version":"1.4"}}}}]}}"#,
            status, config.display()
        ));
    }
    if let Some(url) = opts.mattermost {
        send_webhook(url, &format!(r#"{{"text":"forjar apply {}: {}"}}"#, status, config.display()));
    }
    if let Some(topic) = opts.ntfy {
        let url = format!("https://ntfy.sh/{}", topic);
        let msg = format!("forjar apply {}: {}", status, config.display());
        let _ = std::process::Command::new("curl")
            .args(["-s", "-d", &msg, &url])
            .output();
    }
    if let Some(url) = opts.grafana {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        send_webhook(url, &format!(r#"{{"text":"forjar apply {}","tags":["forjar","deploy"],"time":{}}}"#, status, ts));
    }
}

/// Send to monitoring services (OpsGenie, Datadog, NewRelic).
fn send_monitoring_notifications(opts: &NotifyOpts<'_>, status: &str, config: &Path) {
    if let Some(key) = opts.opsgenie {
        send_webhook_with_header(
            "https://api.opsgenie.com/v2/alerts",
            &format!("Authorization: GenieKey {}", key),
            &format!(r#"{{"message":"forjar apply {}","description":"Apply {} for {}","priority":"P3"}}"#, status, status, config.display()),
        );
    }
    if let Some(key) = opts.datadog {
        send_webhook_with_header(
            "https://api.datadoghq.com/api/v1/events",
            &format!("DD-API-KEY: {}", key),
            &format!(r#"{{"title":"forjar apply {}","text":"Apply {} for {}","alert_type":"info"}}"#, status, status, config.display()),
        );
    }
    if let Some(key) = opts.newrelic {
        send_webhook_with_header(
            "https://insights-collector.newrelic.com/v1/accounts/events",
            &format!("Api-Key: {}", key),
            &format!(r#"{{"eventType":"ForjarApply","status":"{}","config":"{}"}}"#, status, config.display()),
        );
    }
}

/// Send to incident management services (VictorOps, PagerDuty).
fn send_incident_notifications(opts: &NotifyOpts<'_>, result: &Result<(), String>, config: &Path) {
    if let Some(key) = opts.victorops {
        let (vo_status, verb) = if result.is_ok() { ("RECOVERY", "succeeded") } else { ("CRITICAL", "failed") };
        send_webhook(
            &format!("https://alert.victorops.com/integrations/generic/20131114/alert/{}/forjar", key),
            &format!(r#"{{"message_type":"{}","entity_display_name":"forjar apply","state_message":"Apply {} for {}"}}"#, vo_status, verb, config.display()),
        );
    }
    if let Some(key) = opts.incident {
        if result.is_err() {
            send_webhook("https://events.pagerduty.com/v2/enqueue", &format!(
                r#"{{"routing_key":"{}","event_action":"trigger","payload":{{"summary":"Forjar apply failed: {}","source":"forjar","severity":"critical","component":"infrastructure","group":"apply"}}}}"#,
                key, config.display()
            ));
        }
    }
}

/// Send email notification via sendmail.
fn send_email_notification(email: Option<&str>, status: &str, config: &Path) {
    if let Some(addr) = email {
        let body = format!("Subject: forjar apply {}\n\nApply {} for {}\n", status, status, config.display());
        publish_stdin("sendmail", &[addr], &body);
    }
}

/// Send to cloud services (AWS, GCP, Azure).
fn send_cloud_notifications(opts: &NotifyOpts<'_>, msg: &str) {
    if let Some(arn) = opts.sns {
        let _ = std::process::Command::new("aws").args(["sns", "publish", "--topic-arn", arn, "--message", msg]).output();
    }
    if let Some(bus) = opts.eventbridge {
        let detail = msg.replace('"', "\\\"");
        let entry = format!(r#"[{{"Source":"forjar","DetailType":"ApplyEvent","Detail":"{}","EventBusName":"{}"}}]"#, detail, bus);
        let _ = std::process::Command::new("aws").args(["events", "put-events", "--entries", &entry]).output();
    }
    if let Some(url) = opts.sqs {
        let _ = std::process::Command::new("aws").args(["sqs", "send-message", "--queue-url", url, "--message-body", msg]).output();
    }
    if let Some(topic) = opts.pubsub {
        let _ = std::process::Command::new("gcloud").args(["pubsub", "topics", "publish", topic, "--message", msg]).output();
    }
    if let Some(topic) = opts.gcp_pubsub_v2 {
        let _ = std::process::Command::new("gcloud").args(["pubsub", "topics", "publish", topic, "--message", msg, "--ordering-key", "forjar"]).output();
    }
    if let Some(conn) = opts.azure_servicebus {
        let _ = std::process::Command::new("az").args(["servicebus", "topic", "subscription", "create", "--connection-string", conn, "--body", msg]).output();
    }
}

/// Send to message brokers (Kafka, RabbitMQ, NATS, etc.).
fn send_broker_notifications(opts: &NotifyOpts<'_>, msg: &str) {
    if let Some(topic) = opts.kafka {
        publish_stdin("kcat", &["-P", "-t", topic], msg);
    }
    if let Some(queue) = opts.rabbitmq {
        let _ = std::process::Command::new("rabbitmqadmin")
            .args(["publish", "routing_key=forjar", &format!("exchange={}", queue), &format!("payload={}", msg)])
            .output();
    }
    if let Some(subj) = opts.nats {
        let _ = std::process::Command::new("nats").args(["pub", subj, msg]).output();
    }
    if let Some(topic) = opts.mqtt {
        let _ = std::process::Command::new("mosquitto_pub").args(["-t", topic, "-m", msg]).output();
    }
    if let Some(ch) = opts.redis {
        let _ = std::process::Command::new("redis-cli").args(["PUBLISH", ch, msg]).output();
    }
    if let Some(ex) = opts.amqp {
        let _ = std::process::Command::new("amqp-publish").args(["--exchange", ex, "--body", msg]).output();
    }
    if let Some(dest) = opts.stomp {
        let _ = std::process::Command::new("stomp-send").args(["-d", dest, "-m", msg]).output();
    }
    if let Some(ep) = opts.zeromq {
        let _ = std::process::Command::new("zmq-send").args(["--endpoint", ep, "--message", msg]).output();
    }
    if let Some(ep) = opts.grpc {
        let _ = std::process::Command::new("grpcurl").args(["--plaintext", ep, "--data", msg]).output();
    }
}
