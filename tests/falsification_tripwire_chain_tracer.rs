//! FJ-1386/015/050/563: Tripwire chain integrity, event log, trace session,
//! and OTLP export falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1386: Tamper-evident transparency log
//!   - compute_chain_hash: determinism, genesis base
//!   - write_chain_sidecar + verify_chain: roundtrip integrity
//!   - verify_chain: tamper detection
//!   - verify_all_chains: directory traversal
//! - FJ-015: Provenance event log
//!   - now_iso8601: ISO format, non-empty
//!   - generate_run_id: uniqueness, format
//!   - event_log_path: deterministic derivation
//!   - append_event: JSONL accumulation
//! - FJ-050: Trace session
//!   - TraceSession: start, record_span, finalize, write/read roundtrip
//!   - Logical clock ordering, span ID uniqueness
//! - FJ-563: OTLP export
//!   - spans_to_otlp_json: valid JSON structure
//! - collect_machines: unique machine extraction
//!
//! Usage: cargo test --test falsification_tripwire_chain_tracer

use forjar::core::executor::collect_machines;
use forjar::core::types::*;
use forjar::tripwire::chain::{
    compute_chain_hash, verify_all_chains, verify_chain, write_chain_sidecar,
};
use forjar::tripwire::eventlog::{append_event, event_log_path, generate_run_id, now_iso8601};
use forjar::tripwire::otlp_export::spans_to_otlp_json;
use forjar::tripwire::tracer::{read_trace, trace_path, write_trace, TraceSession};
use indexmap::IndexMap;
use std::time::Duration;

// ============================================================================
// FJ-1386: compute_chain_hash
// ============================================================================

#[test]
fn chain_hash_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    std::fs::write(&path, "{\"event\":\"test\"}\n{\"event\":\"two\"}\n").unwrap();
    let h1 = compute_chain_hash(&path).unwrap();
    let h2 = compute_chain_hash(&path).unwrap();
    assert_eq!(h1, h2);
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn chain_hash_empty_file_genesis() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.jsonl");
    std::fs::write(&path, "").unwrap();
    let h = compute_chain_hash(&path).unwrap();
    // Empty file → hash of "genesis" string (no lines processed)
    assert_eq!(h, "genesis");
}

#[test]
fn chain_hash_different_content_different_hash() {
    let dir = tempfile::tempdir().unwrap();
    let p1 = dir.path().join("a.jsonl");
    let p2 = dir.path().join("b.jsonl");
    std::fs::write(&p1, "{\"a\":1}\n").unwrap();
    std::fs::write(&p2, "{\"b\":2}\n").unwrap();
    assert_ne!(
        compute_chain_hash(&p1).unwrap(),
        compute_chain_hash(&p2).unwrap()
    );
}

// ============================================================================
// FJ-1386: write_chain_sidecar + verify_chain
// ============================================================================

#[test]
fn chain_sidecar_verify_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let events = dir.path().join("events.jsonl");
    std::fs::write(&events, "{\"line\":1}\n{\"line\":2}\n").unwrap();

    write_chain_sidecar(&events).unwrap();

    // Sidecar should exist
    let sidecar = events.with_extension("chain");
    assert!(sidecar.exists());

    // Verification should pass
    let v = verify_chain(&events).unwrap();
    assert_eq!(v.total_lines, 2);
    assert_eq!(v.verified, 2);
    assert!(v.failures.is_empty());
}

#[test]
fn chain_verify_detects_tamper() {
    let dir = tempfile::tempdir().unwrap();
    let events = dir.path().join("events.jsonl");
    std::fs::write(&events, "{\"original\":true}\n").unwrap();

    write_chain_sidecar(&events).unwrap();

    // Tamper with events
    std::fs::write(&events, "{\"tampered\":true}\n").unwrap();

    let v = verify_chain(&events).unwrap();
    assert!(
        !v.failures.is_empty(),
        "tampered events should fail verification"
    );
}

#[test]
fn chain_verify_no_sidecar_passes() {
    let dir = tempfile::tempdir().unwrap();
    let events = dir.path().join("events.jsonl");
    std::fs::write(&events, "{\"data\":1}\n").unwrap();
    // No sidecar written
    let v = verify_chain(&events).unwrap();
    assert!(
        v.failures.is_empty(),
        "no sidecar should not cause failures"
    );
    assert_eq!(v.total_lines, 1);
}

// ============================================================================
// FJ-1386: verify_all_chains
// ============================================================================

#[test]
fn chain_verify_all_traverses_machines() {
    let dir = tempfile::tempdir().unwrap();
    // Create two machine dirs with events
    for machine in ["web-01", "db-01"] {
        let mdir = dir.path().join(machine);
        std::fs::create_dir_all(&mdir).unwrap();
        let events = mdir.join("events.jsonl");
        std::fs::write(&events, "{\"m\":\"test\"}\n").unwrap();
        write_chain_sidecar(&events).unwrap();
    }
    let results = verify_all_chains(dir.path());
    assert_eq!(results.len(), 2);
    for (_, v) in &results {
        assert!(v.failures.is_empty());
    }
}

#[test]
fn chain_verify_all_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let results = verify_all_chains(dir.path());
    assert!(results.is_empty());
}

// ============================================================================
// FJ-015: now_iso8601
// ============================================================================

#[test]
fn eventlog_now_iso8601_format() {
    let ts = now_iso8601();
    assert!(ts.ends_with('Z'), "must end with Z for UTC");
    assert!(ts.contains('T'), "must contain T separator");
    assert!(ts.len() >= 19, "must be at least YYYY-MM-DDTHH:MM:SSZ");
}

#[test]
fn eventlog_now_iso8601_non_empty() {
    let ts = now_iso8601();
    assert!(!ts.is_empty());
}

// ============================================================================
// FJ-015: generate_run_id
// ============================================================================

#[test]
fn eventlog_run_id_format() {
    let id = generate_run_id();
    assert!(id.starts_with("r-"), "run ID must start with r-");
    assert!(id.len() > 2, "run ID must have hex suffix");
}

#[test]
fn eventlog_run_id_unique() {
    let id1 = generate_run_id();
    std::thread::sleep(std::time::Duration::from_millis(1));
    let id2 = generate_run_id();
    assert_ne!(id1, id2, "consecutive run IDs should differ");
}

// ============================================================================
// FJ-015: event_log_path
// ============================================================================

#[test]
fn eventlog_path_deterministic() {
    let p = event_log_path(std::path::Path::new("/state"), "web-01");
    assert_eq!(p, std::path::PathBuf::from("/state/web-01/events.jsonl"));
}

// ============================================================================
// FJ-015: append_event
// ============================================================================

#[test]
fn eventlog_append_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let event = ProvenanceEvent::ApplyStarted {
        machine: "test-m".into(),
        run_id: "r-test".into(),
        forjar_version: "1.0".into(),
        operator: None,
        config_hash: None,
        param_count: None,
    };
    append_event(dir.path(), "test-m", event).unwrap();

    let log_path = dir.path().join("test-m/events.jsonl");
    assert!(log_path.exists());
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("apply_started"));
    assert!(content.contains("r-test"));
}

#[test]
fn eventlog_append_accumulates() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        let event = ProvenanceEvent::ResourceConverged {
            machine: "m1".into(),
            resource: format!("r-{i}"),
            duration_seconds: 1.0,
            hash: format!("blake3:hash{i}"),
        };
        append_event(dir.path(), "m1", event).unwrap();
    }
    let content = std::fs::read_to_string(dir.path().join("m1/events.jsonl")).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3);
}

// ============================================================================
// FJ-050: TraceSession
// ============================================================================

#[test]
fn trace_session_start_valid() {
    let session = TraceSession::start("r-test");
    assert!(!session.trace_id().is_empty());
    assert!(!session.run_span_id().is_empty());
    assert_eq!(session.spans().len(), 0);
}

#[test]
fn trace_session_deterministic_ids() {
    let s1 = TraceSession::start("r-same");
    let s2 = TraceSession::start("r-same");
    assert_eq!(s1.trace_id(), s2.trace_id());
}

#[test]
fn trace_session_record_and_finalize() {
    let mut session = TraceSession::start("r-test");
    session.record_span(
        "nginx",
        "package",
        "web-01",
        "create",
        Duration::from_millis(100),
        0,
        None,
    );
    session.record_noop("config", "file", "web-01");
    assert_eq!(session.spans().len(), 2);

    let root = session.finalize();
    assert_eq!(root.name, "forjar:apply");
    assert!(root.parent_span_id.is_none());
    assert_eq!(root.exit_code, 0);
}

#[test]
fn trace_session_logical_clock_monotonic() {
    let mut session = TraceSession::start("r-test");
    session.record_noop("r1", "file", "m1");
    session.record_noop("r2", "file", "m1");
    session.record_noop("r3", "file", "m1");
    assert_eq!(session.spans()[0].logical_clock, 1);
    assert_eq!(session.spans()[1].logical_clock, 2);
    assert_eq!(session.spans()[2].logical_clock, 3);
}

#[test]
fn trace_session_failed_child_marks_root() {
    let mut session = TraceSession::start("r-test");
    session.record_span(
        "bad",
        "package",
        "m1",
        "create",
        Duration::from_secs(1),
        1,
        None,
    );
    let root = session.finalize();
    assert_eq!(root.exit_code, 1);
}

// ============================================================================
// FJ-050: write_trace / read_trace roundtrip
// ============================================================================

#[test]
fn trace_write_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = TraceSession::start("r-test");
    session.record_noop("r1", "file", "m1");
    session.record_span(
        "r2",
        "package",
        "m1",
        "create",
        Duration::from_millis(50),
        0,
        Some("blake3:abc"),
    );

    write_trace(dir.path(), "m1", &session).unwrap();
    let spans = read_trace(dir.path(), "m1").unwrap();
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].name, "apply:r1");
    assert_eq!(spans[1].name, "apply:r2");
    assert_eq!(spans[1].content_hash.as_deref(), Some("blake3:abc"));
}

#[test]
fn trace_read_nonexistent_empty() {
    let dir = tempfile::tempdir().unwrap();
    let spans = read_trace(dir.path(), "nonexistent").unwrap();
    assert!(spans.is_empty());
}

#[test]
fn trace_path_derivation() {
    let p = trace_path(std::path::Path::new("/state"), "web-01");
    assert_eq!(p, std::path::PathBuf::from("/state/web-01/trace.jsonl"));
}

// ============================================================================
// FJ-563: spans_to_otlp_json
// ============================================================================

#[test]
fn otlp_json_valid_structure() {
    let mut session = TraceSession::start("r-test");
    session.record_span(
        "nginx",
        "package",
        "web-01",
        "create",
        Duration::from_millis(200),
        0,
        None,
    );
    let json = spans_to_otlp_json(session.spans(), "forjar-test");

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["resourceSpans"].is_array());
    let scope_spans = &parsed["resourceSpans"][0]["scopeSpans"][0];
    assert!(scope_spans["spans"].is_array());
    assert_eq!(scope_spans["spans"].as_array().unwrap().len(), 1);

    let span = &scope_spans["spans"][0];
    assert_eq!(span["name"], "apply:nginx");
}

#[test]
fn otlp_json_service_name() {
    let mut session = TraceSession::start("r-test");
    session.record_noop("r1", "file", "m1");
    let json = spans_to_otlp_json(session.spans(), "my-service");

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let svc_attr = &parsed["resourceSpans"][0]["resource"]["attributes"][0];
    assert_eq!(svc_attr["key"], "service.name");
    assert_eq!(svc_attr["value"]["stringValue"], "my-service");
}

#[test]
fn otlp_json_empty_spans() {
    let json = spans_to_otlp_json(&[], "svc");
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let spans = &parsed["resourceSpans"][0]["scopeSpans"][0]["spans"];
    assert_eq!(spans.as_array().unwrap().len(), 0);
}

// ============================================================================
// collect_machines
// ============================================================================

#[test]
fn collect_machines_deduplicates() {
    let mut resources = IndexMap::new();
    resources.insert(
        "r1".into(),
        Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("web-01".into()),
            ..Default::default()
        },
    );
    resources.insert(
        "r2".into(),
        Resource {
            resource_type: ResourceType::Package,
            machine: MachineTarget::Single("web-01".into()),
            ..Default::default()
        },
    );
    resources.insert(
        "r3".into(),
        Resource {
            resource_type: ResourceType::Service,
            machine: MachineTarget::Single("db-01".into()),
            ..Default::default()
        },
    );
    let config = ForjarConfig {
        name: "test".into(),
        resources,
        ..Default::default()
    };
    let machines = collect_machines(&config);
    assert_eq!(machines.len(), 2);
    assert!(machines.contains(&"web-01".to_string()));
    assert!(machines.contains(&"db-01".to_string()));
}

#[test]
fn collect_machines_empty_config() {
    let config = ForjarConfig::default();
    let machines = collect_machines(&config);
    assert!(machines.is_empty());
}

#[test]
fn collect_machines_preserves_order() {
    let mut resources = IndexMap::new();
    resources.insert(
        "r1".into(),
        Resource {
            machine: MachineTarget::Single("alpha".into()),
            ..Default::default()
        },
    );
    resources.insert(
        "r2".into(),
        Resource {
            machine: MachineTarget::Single("beta".into()),
            ..Default::default()
        },
    );
    resources.insert(
        "r3".into(),
        Resource {
            machine: MachineTarget::Single("alpha".into()),
            ..Default::default()
        },
    );
    let config = ForjarConfig {
        name: "test".into(),
        resources,
        ..Default::default()
    };
    let machines = collect_machines(&config);
    assert_eq!(machines, vec!["alpha", "beta"]);
}
