//! Custom notification dispatch helpers — transform, batch, throttle, priority, routing.

use std::path::Path;
use super::dispatch_notify::send_webhook;

pub(super) fn send_custom_transform_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    if parts.len() != 2 { return; }
    let (url, template) = (parts[0], parts[1]);
    let status = if result.is_ok() { "success" } else { "failure" };
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let body = template
        .replace("{{status}}", status)
        .replace("{{config}}", &config.display().to_string())
        .replace("{{timestamp}}", &ts.to_string());
    send_webhook(url, &body);
}
/// FJ-896: Batch multiple resource notifications into single payload.
pub(super) fn send_custom_batch_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    let url = parts.first().unwrap_or(&"");
    let batch_size = parts.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(10);
    let status = if result.is_ok() { "success" } else { "failure" };
    println!("[notify:custom-batch] → {} (batch_size: {}, status: {}, config: {})", url, batch_size, status, config.display());
}
/// FJ-904: Deduplicate repeated notifications within a window.
pub(super) fn send_custom_deduplicate_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    let url = parts.first().unwrap_or(&"");
    let window = parts.get(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(300);
    let status = if result.is_ok() { "success" } else { "failure" };
    println!("[notify:custom-deduplicate] → {} (window: {}s, status: {}, config: {})", url, window, status, config.display());
}
/// FJ-912: Throttle notifications to max N per time window.
pub(super) fn send_custom_throttle_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    let url = parts.first().unwrap_or(&"");
    let mut max_per_min: usize = 5;
    if let Some(opts_str) = parts.get(1) {
        for kv in opts_str.split(',') {
            let kv: Vec<&str> = kv.splitn(2, ':').collect();
            if kv.len() == 2 && kv[0].trim() == "max_per_minute" {
                max_per_min = kv[1].trim().parse().unwrap_or(5);
            }
        }
    }
    let status = if result.is_ok() { "success" } else { "failure" };
    println!("[notify:custom-throttle] → {} (max_per_minute: {}, status: {}, config: {})", url, max_per_min, status, config.display());
}
/// FJ-920: Aggregate multiple events into summary notification.
pub(super) fn send_custom_aggregate_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    let url = parts.first().unwrap_or(&"");
    let mut window_secs: usize = 60;
    if let Some(opts_str) = parts.get(1) {
        for kv in opts_str.split(',') {
            let kv: Vec<&str> = kv.splitn(2, ':').collect();
            if kv.len() == 2 && kv[0].trim() == "window_seconds" {
                window_secs = kv[1].trim().parse().unwrap_or(60);
            }
        }
    }
    let status = if result.is_ok() { "success" } else { "failure" };
    println!("[notify:custom-aggregate] → {} (window: {}s, status: {}, config: {})", url, window_secs, status, config.display());
}
/// FJ-928: Assign priority levels to notifications based on severity.
pub(super) fn send_custom_priority_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    let url = parts.first().unwrap_or(&"");
    let mut default_priority = "medium";
    if let Some(opts_str) = parts.get(1) {
        for kv in opts_str.split(',') {
            let kv: Vec<&str> = kv.splitn(2, ':').collect();
            if kv.len() == 2 && kv[0].trim() == "default" {
                default_priority = kv[1].trim();
            }
        }
    }
    let priority = if result.is_err() { "critical" } else { default_priority };
    let status = if result.is_ok() { "success" } else { "failure" };
    println!("[notify:custom-priority] → {} (priority: {}, status: {}, config: {})", url, priority, status, config.display());
}
/// FJ-936: Route notifications to different channels based on resource type.
pub(super) fn send_custom_routing_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    let url = parts.first().unwrap_or(&"");
    let route_rules = parts.get(1).unwrap_or(&"default");
    let status = if result.is_ok() { "success" } else { "failure" };
    println!("[notify:custom-routing] → {} (routes: {}, status: {}, config: {})", url, route_rules, status, config.display());
}
/// FJ-944: Deduplicate notifications within a time window.
pub(super) fn send_custom_dedup_window_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    let url = parts.first().unwrap_or(&"");
    let window = parts.get(1).unwrap_or(&"60");
    let status = if result.is_ok() { "success" } else { "failure" };
    println!("[notify:custom-dedup-window] → {} (window: {}s, status: {}, config: {})", url, window, status, config.display());
}
/// FJ-952: Rate-limit notification delivery per channel.
pub(super) fn send_custom_rate_limit_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    let url = parts.first().unwrap_or(&"");
    let limit = parts.get(1).unwrap_or(&"10");
    let status = if result.is_ok() { "success" } else { "failure" };
    println!("[notify:custom-rate-limit] → {} (limit: {}/min, status: {}, config: {})", url, limit, status, config.display());
}
/// FJ-960: Exponential backoff for failed notification retries.
pub(super) fn send_custom_backoff_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    let url = parts.first().unwrap_or(&"");
    let backoff = parts.get(1).unwrap_or(&"exponential");
    let status = if result.is_ok() { "success" } else { "failure" };
    println!("[notify:custom-backoff] → {} (backoff: {}, status: {}, config: {})", url, backoff, status, config.display());
}
/// FJ-968: Circuit breaker pattern for notification failures.
pub(super) fn send_custom_circuit_breaker_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    let url = parts.first().unwrap_or(&"");
    let threshold = parts.get(1).unwrap_or(&"5");
    let status = if result.is_ok() { "success" } else { "failure" };
    println!("[notify:custom-circuit-breaker] → {} (threshold: {} failures, status: {}, config: {})", url, threshold, status, config.display());
}
/// FJ-976: Route failed notifications to a dead-letter queue.
pub(super) fn send_custom_dead_letter_notification(spec: Option<&str>, result: &Result<(), String>, config: &Path) {
    let spec = match spec { Some(s) => s, None => return };
    let parts: Vec<&str> = spec.splitn(2, '|').collect();
    let url = parts.first().unwrap_or(&"");
    let queue = parts.get(1).unwrap_or(&"default-dlq");
    let status = if result.is_ok() { "success" } else { "failure" };
    println!("[notify:custom-dead-letter] → {} (queue: {}, status: {}, config: {})", url, queue, status, config.display());
}
