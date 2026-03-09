# Metric Thresholds, Event Matching & Generation Diffs

Falsification coverage for FJ-3105, FJ-3100, FJ-2003, and FJ-114.

## Metric Threshold Evaluation (FJ-3105)

### Threshold Operators

Five comparison operators for metric values:

| Operator | Symbol | Example |
|----------|--------|---------|
| `Gt`     | `>`    | cpu > 80 |
| `Gte`    | `>=`   | cpu >= 80 |
| `Lt`     | `<`    | disk < 10 |
| `Lte`    | `<=`   | disk <= 10 |
| `Eq`     | `==`   | replicas == 3 |

```rust
use forjar::core::metric_source::{evaluate_threshold, MetricThreshold, ThresholdOp};

let t = MetricThreshold {
    name: "cpu".into(),
    operator: ThresholdOp::Gt,
    value: 80.0,
    consecutive: 1,
};
assert!(evaluate_threshold(&t, 85.0));
assert!(!evaluate_threshold(&t, 75.0));
```

### Consecutive Violation Tracking

`ThresholdTracker` requires N consecutive violations before firing:

```rust
use forjar::core::metric_source::ThresholdTracker;

let mut tracker = ThresholdTracker::default();
assert!(!tracker.record("cpu", true, 3));  // 1 of 3
assert!(!tracker.record("cpu", true, 3));  // 2 of 3
assert!(tracker.record("cpu", true, 3));   // 3 of 3 → fires

// An OK reading resets the counter
tracker.record("cpu", false, 3);
assert_eq!(tracker.count("cpu"), 0);
```

### Multi-Metric Evaluation

```rust
use forjar::core::metric_source::{evaluate_metrics, MetricThreshold, ThresholdOp, ThresholdTracker};
use std::collections::HashMap;

let thresholds = vec![
    MetricThreshold { name: "cpu".into(), operator: ThresholdOp::Gt, value: 80.0, consecutive: 1 },
    MetricThreshold { name: "mem".into(), operator: ThresholdOp::Gt, value: 90.0, consecutive: 1 },
];
let mut values = HashMap::new();
values.insert("cpu".into(), 85.0);
values.insert("mem".into(), 70.0);
let mut tracker = ThresholdTracker::default();
let results = evaluate_metrics(&thresholds, &values, &mut tracker);
// cpu violated (85 > 80), mem ok (70 ≤ 90)
```

Missing metrics are silently skipped.

## Event-Driven Automation (FJ-3100)

### Event Pattern Matching

Events match patterns when the event type is equal and all `match_fields` are present with matching values:

```rust
use forjar::core::types::{event_matches_pattern, EventPattern, EventType, InfraEvent};

let event = InfraEvent {
    event_type: EventType::FileChanged,
    timestamp: "2026-03-09T12:00:00Z".into(),
    machine: Some("intel".into()),
    payload: vec![("path".into(), "/etc/nginx/nginx.conf".into())].into_iter().collect(),
};
let pattern = EventPattern {
    event_type: EventType::FileChanged,
    match_fields: vec![("path".into(), "/etc/nginx/nginx.conf".into())].into_iter().collect(),
};
assert!(event_matches_pattern(&event, &pattern));
```

### Rulebook Dispatch

Rulebooks bundle patterns with actions. A rulebook matches when enabled and any pattern matches:

```rust
use forjar::core::types::{event_matches_rulebook, Rulebook, EventPattern, EventType, InfraEvent};

let rb = Rulebook {
    name: "nginx-repair".into(),
    description: None,
    events: vec![/* patterns */],
    conditions: vec![],
    actions: vec![],
    cooldown_secs: 60,
    max_retries: 3,
    enabled: true,
};
// Disabled rulebooks never match regardless of events
```

### Cooldown Tracking

`CooldownTracker` prevents repeated firing within a cooldown window:

```rust
use forjar::core::types::CooldownTracker;

let mut tracker = CooldownTracker::default();
assert!(tracker.can_fire("repair", 60));   // never fired
tracker.record_fire("repair");
assert!(!tracker.can_fire("repair", 60));  // within cooldown
assert!(tracker.can_fire("repair", 0));    // zero cooldown always fires
```

## Generation Diffs (FJ-2003)

### Diff Computation

`diff_resource_sets` compares two generations of resource state:

```rust
use forjar::core::types::{diff_resource_sets, DiffAction};

let from = vec![("a", "file", "h1"), ("b", "pkg", "h2")];
let to = vec![("a", "file", "h1"), ("c", "svc", "h3")];
let diffs = diff_resource_sets(&from, &to);
// a → Unchanged, b → Removed, c → Added
```

Results are sorted by resource_id. `GenerationDiff::format_summary()` produces human-readable output with `+`/`~`/`-` prefixes.

## DO-330 Tool Qualification (FJ-114)

```rust
use forjar::core::do330::{generate_qualification_package, ToolQualLevel};

let pkg = generate_qualification_package("1.1.1", ToolQualLevel::Tql5);
assert_eq!(pkg.tool_name, "forjar");
assert!(pkg.qualification_complete);
```

Five qualification levels from TQL-5 (lowest rigor) to TQL-1 (highest).

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_metric_event.rs` | 28 | 377 |
| `falsification_diff_do330.rs` | 12 | 162 |
