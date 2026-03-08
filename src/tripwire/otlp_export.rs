//! FJ-563: OTLP/HTTP JSON trace export.
//!
//! Converts forjar TraceSpan records to the OpenTelemetry Protocol (OTLP)
//! JSON format and POSTs them to a collector endpoint.
//!
//! Uses `curl` for HTTP transport (no extra Rust dependencies).
//! Compatible with Jaeger, Grafana Tempo, Datadog, and any OTLP collector.

use super::tracer::TraceSpan;
use std::path::Path;

/// Convert forjar TraceSpans to OTLP/HTTP JSON format.
///
/// See: <https://opentelemetry.io/docs/specs/otlp/#otlphttp-request>
pub fn spans_to_otlp_json(spans: &[TraceSpan], service_name: &str) -> String {
    let mut otlp_spans = Vec::new();
    for span in spans {
        let start_ns = iso8601_to_nanos(&span.start_time);
        let end_ns = start_ns + (span.duration_us as u64) * 1000;
        let status_code = if span.exit_code == 0 { 1 } else { 2 }; // OK=1, ERROR=2

        let otlp_span = serde_json::json!({
            "traceId": span.trace_id,
            "spanId": span.span_id,
            "parentSpanId": span.parent_span_id.as_deref().unwrap_or(""),
            "name": span.name,
            "kind": 1, // INTERNAL
            "startTimeUnixNano": start_ns.to_string(),
            "endTimeUnixNano": end_ns.to_string(),
            "status": { "code": status_code },
            "attributes": build_attributes(span),
        });
        otlp_spans.push(otlp_span);
    }

    let payload = serde_json::json!({
        "resourceSpans": [{
            "resource": {
                "attributes": [{
                    "key": "service.name",
                    "value": { "stringValue": service_name }
                }]
            },
            "scopeSpans": [{
                "scope": { "name": "forjar", "version": env!("CARGO_PKG_VERSION") },
                "spans": otlp_spans
            }]
        }]
    });

    serde_json::to_string(&payload).unwrap_or_default()
}

/// Build OTLP attribute array from span metadata.
fn build_attributes(span: &TraceSpan) -> Vec<serde_json::Value> {
    let mut attrs = vec![
        attr_str("forjar.resource_type", &span.resource_type),
        attr_str("forjar.machine", &span.machine),
        attr_str("forjar.action", &span.action),
        attr_int("forjar.exit_code", span.exit_code as i64),
        attr_int("forjar.logical_clock", span.logical_clock as i64),
    ];
    if let Some(hash) = &span.content_hash {
        attrs.push(attr_str("forjar.content_hash", hash));
    }
    attrs
}

/// OTLP string attribute.
fn attr_str(key: &str, value: &str) -> serde_json::Value {
    serde_json::json!({ "key": key, "value": { "stringValue": value } })
}

/// OTLP int attribute.
fn attr_int(key: &str, value: i64) -> serde_json::Value {
    serde_json::json!({ "key": key, "value": { "intValue": value.to_string() } })
}

/// POST OTLP JSON payload to the endpoint via curl.
///
/// Endpoint should be the OTLP/HTTP traces endpoint, e.g.:
/// `http://localhost:4318/v1/traces`
pub fn export_traces(
    endpoint: &str,
    spans: &[TraceSpan],
    service_name: &str,
) -> Result<(), String> {
    if spans.is_empty() {
        return Ok(());
    }

    let url = normalize_endpoint(endpoint);
    let payload = spans_to_otlp_json(spans, service_name);

    let output = std::process::Command::new("curl")
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
            &url,
        ])
        .output()
        .map_err(|e| format!("otlp export: curl failed: {e}"))?;

    let status = String::from_utf8_lossy(&output.stdout).to_string();
    let code: u16 = status.trim().parse().unwrap_or(0);

    if (200..300).contains(&code) {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("otlp export: POST {url} returned {code}: {stderr}"))
    }
}

/// Normalize endpoint URL — append `/v1/traces` if not present.
fn normalize_endpoint(endpoint: &str) -> String {
    if endpoint.ends_with("/v1/traces") {
        endpoint.to_string()
    } else {
        let base = endpoint.trim_end_matches('/');
        format!("{base}/v1/traces")
    }
}

/// Export traces from state directory for all machines.
pub fn export_from_state_dir(
    state_dir: &Path,
    endpoint: &str,
    service_name: &str,
) -> Result<usize, String> {
    let mut total = 0;
    if !state_dir.exists() {
        return Ok(0);
    }

    let entries = std::fs::read_dir(state_dir).map_err(|e| format!("read state dir: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let machine = path.file_name().unwrap().to_string_lossy().to_string();
        let spans = super::tracer::read_trace(state_dir, &machine)?;
        if !spans.is_empty() {
            export_traces(endpoint, &spans, service_name)?;
            total += spans.len();
        }
    }

    Ok(total)
}

/// Convert ISO 8601 timestamp to nanoseconds since epoch.
///
/// Handles basic format: `2026-03-08T12:00:00Z`
fn iso8601_to_nanos(ts: &str) -> u64 {
    // Parse manually — no chrono dependency
    let ts = ts.trim_end_matches('Z');
    let parts: Vec<&str> = ts.split('T').collect();
    if parts.len() != 2 {
        return 0;
    }
    let date_parts: Vec<i64> = parts[0].split('-').filter_map(|s| s.parse().ok()).collect();
    let time_parts: Vec<i64> = parts[1].split(':').filter_map(|s| s.parse().ok()).collect();
    if date_parts.len() != 3 || time_parts.len() < 2 {
        return 0;
    }
    let (y, m, d) = (date_parts[0], date_parts[1], date_parts[2]);
    let days = ymd_to_epoch_days(y, m, d);
    let secs = time_parts[0] * 3600 + time_parts[1] * 60 + time_parts.get(2).copied().unwrap_or(0);
    ((days * 86400 + secs) as u64) * 1_000_000_000
}

/// Convert (year, month, day) to epoch days — inverse of epoch_days_to_ymd.
fn ymd_to_epoch_days(y: i64, m: i64, d: i64) -> i64 {
    // Howard Hinnant's days_from_civil
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_endpoint_with_path() {
        assert_eq!(
            normalize_endpoint("http://localhost:4318/v1/traces"),
            "http://localhost:4318/v1/traces"
        );
    }

    #[test]
    fn test_normalize_endpoint_bare() {
        assert_eq!(
            normalize_endpoint("http://localhost:4318"),
            "http://localhost:4318/v1/traces"
        );
    }

    #[test]
    fn test_normalize_endpoint_trailing_slash() {
        assert_eq!(
            normalize_endpoint("http://localhost:4318/"),
            "http://localhost:4318/v1/traces"
        );
    }

    #[test]
    fn test_iso8601_to_nanos() {
        // 2020-01-01T00:00:00Z = epoch day 18262
        let nanos = iso8601_to_nanos("2020-01-01T00:00:00Z");
        assert_eq!(nanos, 18262 * 86400 * 1_000_000_000);
    }

    #[test]
    fn test_iso8601_to_nanos_with_time() {
        let nanos = iso8601_to_nanos("2020-01-01T01:30:00Z");
        let expected = (18262 * 86400 + 3600 + 1800) as u64 * 1_000_000_000;
        assert_eq!(nanos, expected);
    }

    #[test]
    fn test_iso8601_to_nanos_invalid() {
        assert_eq!(iso8601_to_nanos("invalid"), 0);
        assert_eq!(iso8601_to_nanos(""), 0);
    }

    #[test]
    fn test_ymd_to_epoch_days() {
        assert_eq!(ymd_to_epoch_days(1970, 1, 1), 0);
        assert_eq!(ymd_to_epoch_days(2020, 1, 1), 18262);
    }

    #[test]
    fn test_spans_to_otlp_json_empty() {
        let json = spans_to_otlp_json(&[], "forjar");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let scope_spans = &parsed["resourceSpans"][0]["scopeSpans"][0]["spans"];
        assert!(scope_spans.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_spans_to_otlp_json_structure() {
        let spans = vec![TraceSpan {
            trace_id: "00000000000000001234567890abcdef".into(),
            span_id: "fedcba9876543210".into(),
            parent_span_id: Some("0000000000000001".into()),
            name: "apply:nginx-pkg".into(),
            start_time: "2026-03-08T12:00:00Z".into(),
            duration_us: 150000,
            exit_code: 0,
            resource_type: "package".into(),
            machine: "intel".into(),
            action: "create".into(),
            content_hash: Some("blake3:abc123".into()),
            logical_clock: 1,
        }];

        let json = spans_to_otlp_json(&spans, "forjar-test");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify structure
        let resource = &parsed["resourceSpans"][0]["resource"];
        let svc_name = &resource["attributes"][0]["value"]["stringValue"];
        assert_eq!(svc_name, "forjar-test");

        let otlp_span = &parsed["resourceSpans"][0]["scopeSpans"][0]["spans"][0];
        assert_eq!(otlp_span["name"], "apply:nginx-pkg");
        assert_eq!(otlp_span["traceId"], "00000000000000001234567890abcdef");
        assert_eq!(otlp_span["status"]["code"], 1); // OK

        // Verify attributes
        let attrs = otlp_span["attributes"].as_array().unwrap();
        let rt = attrs
            .iter()
            .find(|a| a["key"] == "forjar.resource_type")
            .unwrap();
        assert_eq!(rt["value"]["stringValue"], "package");
    }

    #[test]
    fn test_spans_to_otlp_json_error_status() {
        let spans = vec![TraceSpan {
            trace_id: "a".repeat(32),
            span_id: "b".repeat(16),
            parent_span_id: None,
            name: "apply:failed-pkg".into(),
            start_time: "2026-03-08T12:00:00Z".into(),
            duration_us: 5000,
            exit_code: 1,
            resource_type: "package".into(),
            machine: "web".into(),
            action: "create".into(),
            content_hash: None,
            logical_clock: 1,
        }];

        let json = spans_to_otlp_json(&spans, "forjar");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let otlp_span = &parsed["resourceSpans"][0]["scopeSpans"][0]["spans"][0];
        assert_eq!(otlp_span["status"]["code"], 2); // ERROR
    }

    #[test]
    fn test_export_empty_spans() {
        // Empty spans should be no-op
        let result = export_traces("http://localhost:4318", &[], "forjar");
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_from_empty_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = export_from_state_dir(dir.path(), "http://localhost:4318", "forjar");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_export_from_nonexistent_dir() {
        let result =
            export_from_state_dir(Path::new("/nonexistent"), "http://localhost:4318", "forjar");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
