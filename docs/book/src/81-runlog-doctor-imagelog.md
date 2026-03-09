# Run Logs, Doctor Diagnostics & Image Build Logs

Falsification coverage for FJ-2301.

## Run Logs

Every `forjar apply`, `destroy`, or `undo` creates persistent run logs:

```rust
use forjar::core::types::*;

// Run metadata tracks per-resource status
let mut meta = RunMeta::new("r-abc123".into(), "intel".into(), "apply".into());
meta.record_resource("nginx-pkg", ResourceRunStatus::Noop);
meta.record_resource("nginx-conf", ResourceRunStatus::Converged {
    exit_code: Some(0), duration_secs: Some(0.3), failed: false,
});
assert_eq!(meta.summary.total, 2);

// Structured log entries with delimited sections
let entry = RunLogEntry { resource_id: "pkg".into(), /* ... */ };
let log = entry.format_log();   // FORJAR TRANSPORT LOG format
let json = entry.format_json(); // compact JSON

// Log retention policy
let retention = LogRetention::default();
assert_eq!(retention.keep_runs, 10);
assert_eq!(retention.max_log_size, 10 * 1024 * 1024);
```

## Doctor Diagnostics

`forjar doctor` checks system health, SSH connectivity, and tool availability:

```rust
use forjar::core::types::*;

let report = DoctorReport { system, machines, tools, issues };
assert!(report.is_healthy());          // true if no Error-severity issues
let (e, w, i) = report.issue_counts(); // (errors, warnings, info)
println!("{}", report.format_summary());
```

SSH status variants: `Ok { latency_ms }`, `Failed { error }`, `Local`, `Container`.

## Image Build Logs

Per-layer build output capture for OCI image builds:

```rust
use forjar::core::types::*;

let log = ImageBuildLog {
    image_ref: "training:2.0".into(),
    layers: vec![
        LayerBuildLog::cached("base", 0),
        LayerBuildLog::new("ml-deps", 1, 47.3),
    ],
    ..Default::default()
};
assert!(log.all_succeeded());
assert_eq!(log.cached_count(), 1);
assert_eq!(log.total_log_bytes(), 0);
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_runlog_doctor_imagelog.rs` | 32 | ~426 |
