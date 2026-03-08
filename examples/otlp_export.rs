//! FJ-563: OTLP/HTTP trace export.
//!
//! Demonstrates how forjar converts W3C-compatible trace spans
//! to the OpenTelemetry Protocol (OTLP) JSON format.
//!
//! Usage: cargo run --example otlp_export

use forjar::tripwire::tracer::TraceSpan;

fn main() {
    println!("=== FJ-563: OTLP/HTTP Trace Export ===\n");

    println!("Apply flag:");
    println!("  forjar apply -f forjar.yaml --telemetry-endpoint http://localhost:4318");
    println!();

    // Create sample spans matching what forjar generates during apply
    let spans = vec![
        TraceSpan {
            trace_id: "00000000000000001234567890abcdef".into(),
            span_id: "fedcba9876543210".into(),
            parent_span_id: Some("0000000000000001".into()),
            name: "apply:nginx-pkg".into(),
            start_time: "2026-03-08T12:00:00Z".into(),
            duration_us: 150_000,
            exit_code: 0,
            resource_type: "package".into(),
            machine: "intel".into(),
            action: "create".into(),
            content_hash: Some("blake3:abc123".into()),
            logical_clock: 1,
        },
        TraceSpan {
            trace_id: "00000000000000001234567890abcdef".into(),
            span_id: "1111111111111111".into(),
            parent_span_id: Some("0000000000000001".into()),
            name: "apply:nginx-conf".into(),
            start_time: "2026-03-08T12:00:01Z".into(),
            duration_us: 50_000,
            exit_code: 0,
            resource_type: "file".into(),
            machine: "intel".into(),
            action: "update".into(),
            content_hash: Some("blake3:def456".into()),
            logical_clock: 2,
        },
    ];

    // Convert to OTLP JSON
    let otlp_json = forjar::tripwire::otlp_export::spans_to_otlp_json(&spans, "my-infra");

    println!("OTLP JSON payload (2 spans):\n");
    // Pretty-print the JSON
    let parsed: serde_json::Value = serde_json::from_str(&otlp_json).unwrap();
    println!("{}", serde_json::to_string_pretty(&parsed).unwrap());

    println!("\n--- Endpoint normalization ---\n");
    let endpoints = [
        "http://localhost:4318",
        "http://localhost:4318/",
        "http://localhost:4318/v1/traces",
        "https://otel.example.com",
    ];
    for ep in endpoints {
        println!("  {ep:45} => (auto-appends /v1/traces if needed)");
    }

    println!("\n--- Compatible collectors ---\n");
    println!("  Jaeger:       http://localhost:4318");
    println!("  Grafana Tempo: http://localhost:4318");
    println!("  Datadog:      https://trace.agent.datadoghq.com");
    println!("  Any OTLP/HTTP collector that accepts JSON");
}
