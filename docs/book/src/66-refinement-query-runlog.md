# Refinement Types, Query Health & Run Logs

Falsification coverage for FJ-043, FJ-2001, and FJ-2301.

## Refinement Types (FJ-043)

Type-safe wrappers that enforce invariants at construction time.

### Port

```rust
use forjar::core::types::refinement::Port;

let port = Port::new(8080).unwrap();
assert_eq!(port.value(), 8080);

// Port 0 is invalid
assert!(Port::new(0).is_err());
```

### FileMode

```rust
use forjar::core::types::refinement::FileMode;

let mode = FileMode::new(0o644).unwrap();
assert_eq!(mode.as_octal_string(), "0644");

// Parse from octal string
let mode = FileMode::from_str("755").unwrap();
assert_eq!(mode.value(), 0o755);

// Values above 0o777 are rejected
assert!(FileMode::new(0o1000).is_err());
```

### SemVer

```rust
use forjar::core::types::refinement::SemVer;

let ver = SemVer::parse("1.2.3").unwrap();
assert_eq!(ver.to_string(), "1.2.3");

// Must be exactly X.Y.Z
assert!(SemVer::parse("1.2").is_err());
```

### Hostname (RFC 1123)

```rust
use forjar::core::types::refinement::Hostname;

let host = Hostname::new("web-01.example.com").unwrap();

// Rejects: empty, >253 chars, labels >63, leading/trailing dash, non-alphanumeric
assert!(Hostname::new("").is_err());
assert!(Hostname::new("-bad.com").is_err());
```

### AbsPath & ResourceName

```rust
use forjar::core::types::refinement::{AbsPath, ResourceName};

let path = AbsPath::new("/etc/nginx/nginx.conf").unwrap();
assert!(AbsPath::new("relative/path").is_err());

let name = ResourceName::new("pkg-nginx").unwrap();
assert!(ResourceName::new("has space").is_err());
```

## Query Health Summary (FJ-2001)

### HealthSummary

```rust
use forjar::core::types::{HealthSummary, MachineHealthRow};

let health = HealthSummary {
    machines: vec![MachineHealthRow {
        name: "intel".into(), total: 17, converged: 17,
        drifted: 0, failed: 0, last_apply: "2026-03-09T12:00:00Z".into(), generation: 12,
    }],
};
assert_eq!(health.stack_health_pct(), 100.0);
println!("{}", health.format_table());
```

Empty summaries return 100% health (no resources = no problems).

### TimingStats

```rust
use forjar::core::types::TimingStats;

let durations = vec![0.1, 0.2, 0.3, 0.5, 0.8, 1.0, 1.5, 2.0, 3.0, 5.0];
let stats = TimingStats::from_sorted(&durations).unwrap();
println!("{}", stats.format_compact()); // avg=1.44s p50=1.00s p95=5.00s
```

### ChurnMetric

```rust
use forjar::core::types::ChurnMetric;

let churn = ChurnMetric { resource_id: "bash-aliases".into(), changed_gens: 3, total_gens: 12 };
assert!((churn.churn_pct() - 25.0).abs() < 0.1);
```

## Run Logs (FJ-2301)

### RunMeta Resource Accounting

```rust
use forjar::core::types::{RunMeta, ResourceRunStatus};

let mut meta = RunMeta::new("r-abc".into(), "intel".into(), "apply".into());
meta.record_resource("pkg", ResourceRunStatus::Converged {
    exit_code: Some(0), duration_secs: Some(1.5), failed: false,
});
meta.record_resource("ok", ResourceRunStatus::Noop);
assert_eq!(meta.summary.total, 2);
assert_eq!(meta.summary.converged, 1);
assert_eq!(meta.summary.noop, 1);
```

### RunLogEntry Format

```
=== FORJAR TRANSPORT LOG ===
resource: pkg-nginx
type: package
action: apply
...
=== SCRIPT ===
apt-get install -y nginx

=== STDOUT ===
Reading package lists...

=== STDERR ===

=== RESULT ===
exit_code: 0
duration_secs: 1.234
finished: 2026-03-09T14:30:01Z
```

Also available as JSON via `format_json()` and `format_json_pretty()`.

### LogRetention Defaults

| Setting | Default |
|---------|---------|
| `keep_runs` | 10 |
| `keep_failed` | 50 |
| `max_log_size` | 10 MB |
| `max_total_size` | 500 MB |

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_refinement_types.rs` | 34 | 259 |
| `falsification_query_runlog.rs` | 28 | 436 |
