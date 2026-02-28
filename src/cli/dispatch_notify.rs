//! Apply notification dispatch helpers — sends apply results to notification channels.
use std::path::Path;
pub(super) fn send_webhook(url: &str, payload: &str) {
    let _ = std::process::Command::new("curl")
        .args(["-s", "-X", "POST", "-H", "Content-Type: application/json", "-d", payload, url])
        .output();
}
fn send_webhook_with_header(url: &str, header: &str, payload: &str) {
    let _ = std::process::Command::new("curl")
        .args(["-s", "-X", "POST", "-H", "Content-Type: application/json", "-H", header, "-d", payload, url])
        .output();
}
fn event_json(status: &str, config: &Path) -> String {
    format!(r#"{{"event":"forjar_apply","status":"{}","config":"{}"}}"#, status, config.display())
}
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
    pub pagerduty: Option<&'a str>,
    pub discord_webhook: Option<&'a str>,
    pub teams_webhook: Option<&'a str>,
    pub slack_blocks: Option<&'a str>,
    pub custom_template: Option<&'a str>,
    pub custom_webhook: Option<&'a str>,
    pub custom_headers: Option<&'a str>,
    pub custom_json: Option<&'a str>,
    pub custom_filter: Option<&'a str>,
    pub custom_retry: Option<&'a str>,
    pub custom_transform: Option<&'a str>,
    pub custom_batch: Option<&'a str>,
    pub custom_deduplicate: Option<&'a str>,
    pub custom_throttle: Option<&'a str>,
    pub custom_aggregate: Option<&'a str>,
    pub custom_priority: Option<&'a str>,
    pub custom_routing: Option<&'a str>,
    pub custom_dedup_window: Option<&'a str>,
    pub custom_rate_limit: Option<&'a str>,
    pub custom_backoff: Option<&'a str>,
    pub custom_circuit_breaker: Option<&'a str>,
    pub custom_dead_letter: Option<&'a str>,
    pub custom_escalation: Option<&'a str>,
    pub custom_correlation: Option<&'a str>,
    pub custom_sampling: Option<&'a str>,
}
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
    send_pagerduty_notification(opts.pagerduty, result, config);
    send_discord_webhook_notification(opts.discord_webhook, result, config);
    send_teams_webhook_notification(opts.teams_webhook, result, config);
    send_slack_blocks_notification(opts.slack_blocks, result, config);
    send_custom_template_notification(opts.custom_template, result, config);
    send_custom_webhook_notification(opts.custom_webhook, result, config);
    send_custom_headers_notification(opts.custom_headers, result, config);
    send_custom_json_notification(opts.custom_json, result, config);
    send_custom_filter_notification(opts.custom_filter, result, config);
    send_custom_retry_notification(opts.custom_retry, result, config);
    super::dispatch_notify_custom::send_custom_transform_notification(opts.custom_transform, result, config);
    super::dispatch_notify_custom::send_custom_batch_notification(opts.custom_batch, result, config);
    super::dispatch_notify_custom::send_custom_deduplicate_notification(opts.custom_deduplicate, result, config);
    super::dispatch_notify_custom::send_custom_throttle_notification(opts.custom_throttle, result, config);
    super::dispatch_notify_custom::send_custom_aggregate_notification(opts.custom_aggregate, result, config);
    super::dispatch_notify_custom::send_custom_priority_notification(opts.custom_priority, result, config);
    super::dispatch_notify_custom::send_custom_routing_notification(opts.custom_routing, result, config);
    super::dispatch_notify_custom::send_custom_dedup_window_notification(opts.custom_dedup_window, result, config);
    super::dispatch_notify_custom::send_custom_rate_limit_notification(opts.custom_rate_limit, result, config);
    super::dispatch_notify_custom::send_custom_backoff_notification(opts.custom_backoff, result, config);
    super::dispatch_notify_custom::send_custom_circuit_breaker_notification(opts.custom_circuit_breaker, result, config);
    super::dispatch_notify_custom::send_custom_dead_letter_notification(opts.custom_dead_letter, result, config);
    super::dispatch_notify_custom::send_custom_escalation_notification(opts.custom_escalation, result, config);
    super::dispatch_notify_custom::send_custom_correlation_notification(opts.custom_correlation, result, config);
    super::dispatch_notify_custom::send_custom_sampling_notification(opts.custom_sampling, result, config);
}
fn send_pagerduty_notification(key: Option<&str>, result: &Result<(), String>, config: &Path) {
    if let Some(key) = key {
        let action = if result.is_ok() { "resolve" } else { "trigger" };
        let severity = if result.is_ok() { "info" } else { "error" };
        send_webhook("https://events.pagerduty.com/v2/enqueue", &format!(
            r#"{{"routing_key":"{}","event_action":"{}","payload":{{"summary":"Forjar apply: {}","source":"forjar","severity":"{}","component":"infrastructure"}}}}"#,
            key, action, config.display(), severity
        ));
    }
}
fn send_discord_webhook_notification(url: Option<&str>, result: &Result<(), String>, config: &Path) {
    if let Some(url) = url {
        let color = if result.is_ok() { 3066993 } else { 15158332 };
        let title = if result.is_ok() { "Apply Succeeded" } else { "Apply Failed" };
        send_webhook(url, &format!(
            r#"{{"embeds":[{{"title":"{}","description":"Config: {}","color":{},"footer":{{"text":"forjar"}}}}]}}"#,
            title, config.display(), color
        ));
    }
}
fn send_teams_webhook_notification(url: Option<&str>, result: &Result<(), String>, config: &Path) {
    if let Some(url) = url {
        let color = if result.is_ok() { "Good" } else { "Attention" };
        let title = if result.is_ok() { "Apply Succeeded" } else { "Apply Failed" };
        send_webhook(url, &format!(
            r#"{{"type":"message","attachments":[{{"contentType":"application/vnd.microsoft.card.adaptive","content":{{"type":"AdaptiveCard","body":[{{"type":"TextBlock","text":"{}","weight":"Bolder","color":"{}"}},{{"type":"TextBlock","text":"Config: {}"}}],"$schema":"http://adaptivecards.io/schemas/adaptive-card.json","version":"1.4"}}}}]}}"#,
            title, color, config.display()
        ));
    }
}
fn send_slack_blocks_notification(url: Option<&str>, result: &Result<(), String>, config: &Path) {
    if let Some(url) = url {
        let emoji = if result.is_ok() { ":white_check_mark:" } else { ":x:" };
        let title = if result.is_ok() { "Apply Succeeded" } else { "Apply Failed" };
        send_webhook(url, &format!(
            r#"{{"blocks":[{{"type":"header","text":{{"type":"plain_text","text":"{} {}"}}}},{{"type":"section","text":{{"type":"mrkdwn","text":"*Config:* `{}`"}}}}]}}"#,
            emoji, title, config.display()
        ));
    }
}
fn send_custom_template_notification(template: Option<&str>, result: &Result<(), String>, config: &Path) {
    if let Some(template) = template {
        let status = if result.is_ok() { "success" } else { "failure" };
        let rendered = template
            .replace("{{status}}", status)
            .replace("{{config}}", &config.display().to_string());
        let _ = std::process::Command::new("bash")
            .arg("-c")
            .arg(&rendered)
            .output();
    }
}
fn send_custom_webhook_notification(url: Option<&str>, result: &Result<(), String>, config: &Path) {
    if let Some(url) = url {
        let status = if result.is_ok() { "success" } else { "failure" };
        let payload = format!(
            r#"{{"event":"forjar_apply","status":"{}","config":"{}","timestamp":"{}"}}"#,
            status, config.display(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
        );
        send_webhook(url, &payload);
    }
}
fn send_custom_headers_notification(headers: Option<&str>, result: &Result<(), String>, config: &Path) {
    if let Some(headers_str) = headers {
        let status = if result.is_ok() { "success" } else { "failure" };
        let payload = event_json(status, config);
        // Parse "url|Header1:Value1|Header2:Value2" format
        let parts: Vec<&str> = headers_str.splitn(2, '|').collect();
        if !parts.is_empty() {
            let url = parts[0];
            let mut args = vec!["-s", "-X", "POST", "-H", "Content-Type: application/json"];
            let header_parts: Vec<String> = if parts.len() > 1 {
                parts[1].split('|').map(|h| h.to_string()).collect()
            } else {
                Vec::new()
            };
            for h in &header_parts {
                args.push("-H");
                args.push(h);
            }
            args.push("-d");
            args.push(&payload);
            args.push(url);
            let _ = std::process::Command::new("curl").args(&args).output();
        }
    }
}
fn send_custom_json_notification(template: Option<&str>, result: &Result<(), String>, config: &Path) {
    if let Some(tmpl) = template {
        let status = if result.is_ok() { "success" } else { "failure" };
        // Parse "url|json_template" format, replacing {{status}} and {{config}}
        let parts: Vec<&str> = tmpl.splitn(2, '|').collect();
        if parts.len() == 2 {
            let url = parts[0];
            let body = parts[1]
                .replace("{{status}}", status)
                .replace("{{config}}", &config.display().to_string());
            send_webhook(url, &body);
        }
    }
}
fn send_email_notification(email: Option<&str>, status: &str, config: &Path) {
    if let Some(addr) = email {
        let body = format!("Subject: forjar apply {}\n\nApply {} for {}\n", status, status, config.display());
        publish_stdin("sendmail", &[addr], &body);
    }
}
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
fn send_custom_filter_notification(filter: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match filter { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    if parts.len() != 2 { return; }
    let (url, filter_expr) = (parts[0], parts[1]);
    let status = if result.is_ok() { "success" } else { "failure" };
    let payload = format!(
        r#"{{"event":"forjar_apply","status":"{}","config":"{}","filter":"{}"}}"#,
        status, config.display(), filter_expr,
    );
    send_webhook(url, &payload);
}
fn send_custom_retry_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    if parts.len() != 2 { return; }
    let (url, opts) = (parts[0], parts[1]);
    let mut retries: usize = 3;
    for kv in opts.split(',') {
        let kv: Vec<&str> = kv.splitn(2, ':').collect();
        if kv.len() == 2 && kv[0].trim() == "retries" {
            retries = kv[1].trim().parse().unwrap_or(3);
        }
    }
    let status = if result.is_ok() { "success" } else { "failure" };
    let payload = format!(
        r#"{{"event":"forjar_apply","status":"{}","config":"{}","retries":{}}}"#,
        status, config.display(), retries,
    );
    for attempt in 0..=retries {
        let out = std::process::Command::new("curl")
            .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", "-X", "POST", "-H", "Content-Type: application/json", "-d", &payload, url])
            .output();
        let ok = match out {
            Ok(ref o) => String::from_utf8_lossy(&o.stdout).starts_with('2'),
            Err(_) => false,
        };
        if ok || attempt == retries { break; }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
