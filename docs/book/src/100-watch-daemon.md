# Watch Daemon & Apply Gates

FJ-3102 event-driven watch daemon and CLI apply gate extraction.

## Watch Daemon Orchestrator

Pure logic for the event-driven automation pipeline. The daemon coordinates
6 event sources through rulebook evaluation to action dispatch:

```rust
use forjar::core::watch_daemon::*;
use forjar::core::types::{EventType, Rulebook, RulebookConfig};

// Configure daemon
let config = WatchDaemonConfig {
    poll_interval_secs: 5,
    cron_schedules: vec![("nightly".into(), "0 0 * * *".into())],
    watch_paths: vec!["/etc/nginx/nginx.conf".into()],
    ..Default::default()
};
let mut state = DaemonState::new(&config);

// Process an event through rulebooks
let event = file_changed_event("/etc/nginx/nginx.conf", "2026-03-10T12:00:00Z");
let result = process_event(&event, &rb_config, &mut state);
for (rb_name, action) in &result.pending_actions {
    let kind = classify_action(action);
    println!("{rb_name} dispatches {kind}");
}
```

### Event Sources

| Source | Function | Event Type |
|--------|----------|------------|
| File watcher | `detect_file_changes()` | `FileChanged` |
| Cron scheduler | `check_cron_schedules()` | `CronFired` |
| Metric poller | `check_metrics()` | `MetricThreshold` |
| Webhook HTTP | `webhook_source::request_to_event()` | `WebhookReceived` |
| Process monitor | (daemon loop) | `ProcessExit` |
| Manual trigger | `forjar trigger <rulebook>` | `Manual` |

### Action Dispatch

```rust
match classify_action(&action) {
    ActionKind::Apply   => /* forjar apply with subset/tags */,
    ActionKind::Destroy => /* forjar destroy resources */,
    ActionKind::Script  => /* run shell command */,
    ActionKind::Notify  => /* send webhook/slack notification */,
    ActionKind::Unknown => /* skip */,
}
```

### Event Logging

```rust
let log_line = format_event_log(&event, &[("config-repair".into(), ActionKind::Apply)]);
// {"timestamp":"...","event_type":"file_changed","actions":[{"rulebook":"config-repair","action":"apply"}]}
```

## Apply Gates (CLI Logic Extraction)

Pure decision logic extracted from CLI dispatch for testability:

```rust
use forjar::cli::apply_gates::*;

// Convergence budget check
check_convergence_budget_pure(Some(60), elapsed_secs)?;

// Security gate threshold
let blocked = security_gate_should_block("high", crit, high, med, total)?;

// Subset/exclude resource filtering
let count = filter_subset(&mut resources, "web-*")?;
let removed = filter_exclude(&mut resources, "test-*");

// Drift gate
if let Some(msg) = should_block_on_drift(tripwire, force, drift_count) {
    return Err(msg);
}
```

## Rejection Criteria

24 watch daemon tests + 40 apply gates tests in `src/core/watch_daemon.rs` and `src/cli/apply_gates.rs`:

- Daemon state initialization (cron parsing, invalid cron skipped)
- Event processing (match, no-match, multiple rulebooks, cooldown blocking)
- Action classification (apply, destroy, script, notify, unknown)
- Cron schedule checking (match, no-match)
- File change detection (initial no-fire, change fire, no-change)
- Metric threshold evaluation (fire, no-fire, consecutive)
- Event log formatting (with/without actions)
- Convergence budget (none, within, at, exceeded)
- Security gate (critical/high/medium/low thresholds, case-insensitive, unknown)
- Subset/exclude filters (exact, wildcard, no-match, star)
- Drift gate (disabled, force override, blocks, no-drift)
- Destructive action gate (blocks, not-confirmed, dry-run, yes override)
