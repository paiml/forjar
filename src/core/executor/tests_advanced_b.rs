//! FJ-050: Trace tests, FJ-051: Anomaly detection.

use super::*;

#[test]
fn test_fj050_trace_written_on_apply() {
    // Verify that apply_machine writes trace.jsonl when tripwire is enabled
    let dir = tempfile::tempdir().unwrap();

    let yaml = r#"
version: "1.0"
name: trace-test
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
policy:
  tripwire: true
resources:
  test-dir:
    type: file
    machine: localhost
    path: /tmp/forjar-trace-test
    content: "trace-test"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
    };

    let results = apply(&cfg).unwrap();
    assert!(!results.is_empty());

    // Check that trace.jsonl was written
    let trace_path = dir.path().join("localhost").join("trace.jsonl");
    assert!(
        trace_path.exists(),
        "trace.jsonl should be written when tripwire is enabled"
    );

    // Parse the trace spans
    let spans = tracer::read_trace(dir.path(), "localhost").unwrap();
    assert!(!spans.is_empty(), "trace should contain at least one span");

    // All spans should have the same trace ID
    let trace_id = &spans[0].trace_id;
    for span in &spans {
        assert_eq!(&span.trace_id, trace_id, "all spans share trace ID");
    }
}

#[test]
fn test_fj050_trace_not_written_when_tripwire_off() {
    let dir = tempfile::tempdir().unwrap();

    let yaml = r#"
version: "1.0"
name: no-trace-test
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
policy:
  tripwire: false
resources:
  test-dir:
    type: file
    machine: localhost
    path: /tmp/forjar-no-trace-test
    content: "no-trace"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
    };

    let _results = apply(&cfg).unwrap();

    // trace.jsonl should NOT exist when tripwire is off
    let trace_path = dir.path().join("localhost").join("trace.jsonl");
    assert!(
        !trace_path.exists(),
        "trace.jsonl should not be written when tripwire is off"
    );
}

#[test]
fn test_fj050_trace_span_fields() {
    let dir = tempfile::tempdir().unwrap();

    let yaml = r#"
version: "1.0"
name: span-fields-test
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
policy:
  tripwire: true
resources:
  config-file:
    type: file
    machine: localhost
    path: /tmp/forjar-span-fields
    content: "span-fields"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
    };

    let _results = apply(&cfg).unwrap();
    let spans = tracer::read_trace(dir.path(), "localhost").unwrap();
    assert!(!spans.is_empty());

    let span = &spans[0];
    assert!(
        span.name.starts_with("apply:"),
        "span name should start with apply: got: {}",
        span.name
    );
    assert_eq!(span.machine, "localhost");
    assert!(span.logical_clock > 0, "logical clock should be positive");
    assert!(
        span.parent_span_id.is_some(),
        "child spans should have parent_span_id"
    );
}

#[test]
fn test_fj051_cmd_anomaly_uses_module() {
    // Verify that detect_anomalies is callable and returns consistent results
    let metrics = vec![
        ("stable:web".to_string(), 5u32, 0u32, 0u32),
        ("stable:db".to_string(), 5, 0, 0),
        ("stable:cache".to_string(), 5, 0, 0),
    ];
    let findings = crate::tripwire::anomaly::detect_anomalies(&metrics, 3);
    assert!(
        findings.is_empty(),
        "uniform metrics should produce no anomalies"
    );

    // Add a churny resource
    let mut metrics2 = metrics.clone();
    metrics2.push(("churny:web".to_string(), 500, 0, 0));
    metrics2.push(("drifty:db".to_string(), 10, 0, 5));
    let findings2 = crate::tripwire::anomaly::detect_anomalies(&metrics2, 3);
    assert!(
        !findings2.is_empty(),
        "should detect anomalies in mixed metrics"
    );
    // Drift events should always be flagged
    assert!(
        findings2.iter().any(|f| f.resource == "drifty:db"),
        "drift events should be flagged"
    );
}
