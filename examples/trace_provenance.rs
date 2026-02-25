//! Demonstrate the trace provenance system (FJ-050).
//!
//! Shows how forjar records W3C-compatible trace spans during apply,
//! with Lamport logical clocks for causal ordering and JSONL output.
//!
//! Usage: cargo run --example trace_provenance

use forjar::tripwire::tracer;
use std::time::Duration;

fn main() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let state_dir = dir.path();
    let machine = "web-server";

    println!("=== Trace Provenance Example ===\n");

    // Start a trace session (like apply_machine does)
    let run_id = "r-example-001";
    let mut session = tracer::TraceSession::start(run_id);
    println!("Trace ID:  {}", session.trace_id());
    println!("Root Span: {}", session.run_span_id());
    println!();

    // Record resource apply spans
    println!("Recording resource spans...\n");

    // 1. Package install (success, 2.3s)
    session.record_span(
        "nginx-pkg",
        "package",
        machine,
        "create",
        Duration::from_millis(2300),
        0,
        None,
    );

    // 2. Config file (success, 0.1s, with content hash)
    session.record_span(
        "nginx-config",
        "file",
        machine,
        "create",
        Duration::from_millis(100),
        0,
        Some("blake3:7f83b1657ff1fc53b92dc18148a1d65d"),
    );

    // 3. Service already running (noop)
    session.record_noop("nginx-svc", "service", machine);

    // 4. Failed resource (bad mount, exit code 1)
    session.record_span(
        "data-mount",
        "mount",
        machine,
        "create",
        Duration::from_millis(500),
        1,
        None,
    );

    // Print spans
    for span in session.spans() {
        println!(
            "  [{:>3}] {} — {} ({:?}, exit={})",
            span.logical_clock,
            span.name,
            span.action,
            Duration::from_micros(span.duration_us),
            span.exit_code,
        );
    }

    // Finalize root span
    println!();
    let root = session.finalize();
    println!(
        "Root span: {} — exit={} ({})",
        root.name,
        root.exit_code,
        if root.exit_code == 0 {
            "success"
        } else {
            "FAILED"
        }
    );

    // Write trace to JSONL file
    tracer::write_trace(state_dir, machine, &session).expect("write trace");
    let trace_path = tracer::trace_path(state_dir, machine);
    println!("\nTrace written to: {}", trace_path.display());

    // Read it back
    let spans = tracer::read_trace(state_dir, machine).expect("read trace");
    println!("Read back {} spans\n", spans.len());

    // Demonstrate deterministic IDs
    let session2 = tracer::TraceSession::start(run_id);
    assert_eq!(
        session.trace_id(),
        session2.trace_id(),
        "same run_id → same trace_id"
    );
    println!("Determinism: same run_id produces same trace_id ✓");

    let session3 = tracer::TraceSession::start("r-different");
    assert_ne!(session.trace_id(), session3.trace_id());
    println!("Uniqueness: different run_id produces different trace_id ✓");

    // Show causal ordering via logical clock
    println!("\nCausal ordering (Lamport clock):");
    for span in &spans {
        println!("  clock={} → {}", span.logical_clock, span.name);
    }

    println!("\n=== Trace Provenance Example Complete ===");
}
