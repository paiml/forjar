use super::dispatch_notify::*;
use std::path::Path;

pub(super) fn send_pagerduty_notification(
    key: Option<&str>,
    result: &Result<(), String>,
    config: &Path,
) {
    if let Some(key) = key {
        let action = if result.is_ok() { "resolve" } else { "trigger" };
        let severity = if result.is_ok() { "info" } else { "error" };
        send_webhook(
            "https://events.pagerduty.com/v2/enqueue",
            &format!(
                r#"{{"routing_key":"{}","event_action":"{}","payload":{{"summary":"Forjar apply: {}","source":"forjar","severity":"{}","component":"infrastructure"}}}}"#,
                key,
                action,
                config.display(),
                severity
            ),
        );
    }
}
pub(super) fn send_discord_webhook_notification(
    url: Option<&str>,
    result: &Result<(), String>,
    config: &Path,
) {
    if let Some(url) = url {
        let color = if result.is_ok() { 3066993 } else { 15158332 };
        let title = if result.is_ok() {
            "Apply Succeeded"
        } else {
            "Apply Failed"
        };
        send_webhook(
            url,
            &format!(
                r#"{{"embeds":[{{"title":"{}","description":"Config: {}","color":{},"footer":{{"text":"forjar"}}}}]}}"#,
                title,
                config.display(),
                color
            ),
        );
    }
}
pub(super) fn send_teams_webhook_notification(
    url: Option<&str>,
    result: &Result<(), String>,
    config: &Path,
) {
    if let Some(url) = url {
        let color = if result.is_ok() { "Good" } else { "Attention" };
        let title = if result.is_ok() {
            "Apply Succeeded"
        } else {
            "Apply Failed"
        };
        send_webhook(
            url,
            &format!(
                r#"{{"type":"message","attachments":[{{"contentType":"application/vnd.microsoft.card.adaptive","content":{{"type":"AdaptiveCard","body":[{{"type":"TextBlock","text":"{}","weight":"Bolder","color":"{}"}},{{"type":"TextBlock","text":"Config: {}"}}],"$schema":"http://adaptivecards.io/schemas/adaptive-card.json","version":"1.4"}}}}]}}"#,
                title,
                color,
                config.display()
            ),
        );
    }
}
pub(super) fn send_slack_blocks_notification(
    url: Option<&str>,
    result: &Result<(), String>,
    config: &Path,
) {
    if let Some(url) = url {
        let emoji = if result.is_ok() {
            ":white_check_mark:"
        } else {
            ":x:"
        };
        let title = if result.is_ok() {
            "Apply Succeeded"
        } else {
            "Apply Failed"
        };
        send_webhook(
            url,
            &format!(
                r#"{{"blocks":[{{"type":"header","text":{{"type":"plain_text","text":"{} {}"}}}},{{"type":"section","text":{{"type":"mrkdwn","text":"*Config:* `{}`"}}}}]}}"#,
                emoji,
                title,
                config.display()
            ),
        );
    }
}
pub(super) fn send_custom_template_notification(
    template: Option<&str>,
    result: &Result<(), String>,
    config: &Path,
) {
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
pub(super) fn send_custom_webhook_notification(
    url: Option<&str>,
    result: &Result<(), String>,
    config: &Path,
) {
    if let Some(url) = url {
        let status = if result.is_ok() { "success" } else { "failure" };
        let payload = format!(
            r#"{{"event":"forjar_apply","status":"{}","config":"{}","timestamp":"{}"}}"#,
            status,
            config.display(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
        send_webhook(url, &payload);
    }
}
pub(super) fn send_custom_headers_notification(
    headers: Option<&str>,
    result: &Result<(), String>,
    config: &Path,
) {
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
pub(super) fn send_custom_json_notification(
    template: Option<&str>,
    result: &Result<(), String>,
    config: &Path,
) {
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
pub(super) fn send_email_notification(email: Option<&str>, status: &str, config: &Path) {
    if let Some(addr) = email {
        let body = format!(
            "Subject: forjar apply {}\n\nApply {} for {}\n",
            status,
            status,
            config.display()
        );
        publish_stdin("sendmail", &[addr], &body);
    }
}
pub(super) fn send_cloud_notifications(opts: &NotifyOpts<'_>, msg: &str) {
    if let Some(arn) = opts.sns {
        let _ = std::process::Command::new("aws")
            .args(["sns", "publish", "--topic-arn", arn, "--message", msg])
            .output();
    }
    if let Some(bus) = opts.eventbridge {
        let detail = msg.replace('"', "\\\"");
        let entry = format!(
            r#"[{{"Source":"forjar","DetailType":"ApplyEvent","Detail":"{detail}","EventBusName":"{bus}"}}]"#
        );
        let _ = std::process::Command::new("aws")
            .args(["events", "put-events", "--entries", &entry])
            .output();
    }
    if let Some(url) = opts.sqs {
        let _ = std::process::Command::new("aws")
            .args([
                "sqs",
                "send-message",
                "--queue-url",
                url,
                "--message-body",
                msg,
            ])
            .output();
    }
    if let Some(topic) = opts.pubsub {
        let _ = std::process::Command::new("gcloud")
            .args(["pubsub", "topics", "publish", topic, "--message", msg])
            .output();
    }
    if let Some(topic) = opts.gcp_pubsub_v2 {
        let _ = std::process::Command::new("gcloud")
            .args([
                "pubsub",
                "topics",
                "publish",
                topic,
                "--message",
                msg,
                "--ordering-key",
                "forjar",
            ])
            .output();
    }
    if let Some(conn) = opts.azure_servicebus {
        let _ = std::process::Command::new("az")
            .args([
                "servicebus",
                "topic",
                "subscription",
                "create",
                "--connection-string",
                conn,
                "--body",
                msg,
            ])
            .output();
    }
}
pub(super) fn send_broker_notifications(opts: &NotifyOpts<'_>, msg: &str) {
    if let Some(topic) = opts.kafka {
        publish_stdin("kcat", &["-P", "-t", topic], msg);
    }
    if let Some(queue) = opts.rabbitmq {
        let _ = std::process::Command::new("rabbitmqadmin")
            .args([
                "publish",
                "routing_key=forjar",
                &format!("exchange={queue}"),
                &format!("payload={msg}"),
            ])
            .output();
    }
    if let Some(subj) = opts.nats {
        let _ = std::process::Command::new("nats")
            .args(["pub", subj, msg])
            .output();
    }
    if let Some(topic) = opts.mqtt {
        let _ = std::process::Command::new("mosquitto_pub")
            .args(["-t", topic, "-m", msg])
            .output();
    }
    if let Some(ch) = opts.redis {
        let _ = std::process::Command::new("redis-cli")
            .args(["PUBLISH", ch, msg])
            .output();
    }
    if let Some(ex) = opts.amqp {
        let _ = std::process::Command::new("amqp-publish")
            .args(["--exchange", ex, "--body", msg])
            .output();
    }
    if let Some(dest) = opts.stomp {
        let _ = std::process::Command::new("stomp-send")
            .args(["-d", dest, "-m", msg])
            .output();
    }
    if let Some(ep) = opts.zeromq {
        let _ = std::process::Command::new("zmq-send")
            .args(["--endpoint", ep, "--message", msg])
            .output();
    }
    if let Some(ep) = opts.grpc {
        let _ = std::process::Command::new("grpcurl")
            .args(["--plaintext", ep, "--data", msg])
            .output();
    }
}
pub(super) fn send_custom_filter_notification(
    filter: Option<&str>,
    result: &Result<(), String>,
    config: &Path,
) {
    let spec = match filter {
        Some(s) => s,
        None => return,
    };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    if parts.len() != 2 {
        return;
    }
    let (url, filter_expr) = (parts[0], parts[1]);
    let status = if result.is_ok() { "success" } else { "failure" };
    let payload = format!(
        r#"{{"event":"forjar_apply","status":"{}","config":"{}","filter":"{}"}}"#,
        status,
        config.display(),
        filter_expr,
    );
    send_webhook(url, &payload);
}
pub(super) fn send_custom_retry_notification(
    spec: Option<&str>,
    result: &Result<(), String>,
    config: &Path,
) {
    let spec = match spec {
        Some(s) => s,
        None => return,
    };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    if parts.len() != 2 {
        return;
    }
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
        status,
        config.display(),
        retries,
    );
    for attempt in 0..=retries {
        let out = std::process::Command::new("curl")
            .args([
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "-X",
                "POST",
                "-H",
                "Content-Type: application/json",
                "-d",
                &payload,
                url,
            ])
            .output();
        let ok = match out {
            Ok(ref o) => String::from_utf8_lossy(&o.stdout).starts_with('2'),
            Err(_) => false,
        };
        if ok || attempt == retries {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
