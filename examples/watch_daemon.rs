//! FJ-3102: Watch daemon orchestrator demonstration.
//!
//! Shows the event-driven automation pipeline:
//! cron schedules → file changes → metric thresholds → rulebook evaluation → action dispatch.
//!
//! Usage: cargo run --example watch_daemon

use forjar::core::cron_source::CronTime;
use forjar::core::metric_source::{MetricThreshold, ThresholdOp};
use forjar::core::types::{ApplyAction, EventPattern, EventType, Rulebook, RulebookAction};
use forjar::core::watch_daemon::{
    build_rulebook_config, check_cron_schedules, check_metrics, classify_action, cron_event,
    daemon_summary, detect_file_changes, file_changed_event, format_event_log, process_event,
    ActionKind, DaemonState, WatchDaemonConfig,
};
use std::collections::HashMap;

fn main() {
    println!("=== FJ-3102: Watch Daemon Orchestrator ===\n");

    // 1. Configure the daemon
    let config = WatchDaemonConfig {
        poll_interval_secs: 5,
        cron_schedules: vec![
            ("nightly-backup".into(), "0 0 * * *".into()),
            ("hourly-check".into(), "0 * * * *".into()),
        ],
        metric_thresholds: vec![MetricThreshold {
            name: "cpu_percent".into(),
            operator: ThresholdOp::Gt,
            value: 80.0,
            consecutive: 2,
        }],
        webhook_port: 8484,
        watch_paths: vec![
            "/etc/nginx/nginx.conf".into(),
            "/etc/forjar/forjar.yaml".into(),
        ],
        event_buffer_size: 1024,
        event_logging: true,
    };

    let mut state = DaemonState::new(&config);
    println!("Daemon initialized:");
    println!("  Cron schedules: {} parsed", state.cron_parsed.len());
    println!("  Watch paths: {}", config.watch_paths.len());
    println!("  Metric thresholds: {}", config.metric_thresholds.len());
    println!();

    // 2. Set up rulebooks
    let rulebooks = vec![
        Rulebook {
            name: "config-repair".into(),
            description: Some("Repair nginx config on change".into()),
            events: vec![EventPattern {
                event_type: EventType::FileChanged,
                match_fields: {
                    let mut m = HashMap::new();
                    m.insert("path".into(), "/etc/nginx/nginx.conf".into());
                    m
                },
            }],
            conditions: Vec::new(),
            actions: vec![RulebookAction {
                apply: Some(ApplyAction {
                    file: "forjar.yaml".into(),
                    subset: vec!["nginx-config".into()],
                    tags: vec!["config".into()],
                    machine: None,
                }),
                destroy: None,
                script: None,
                notify: None,
            }],
            cooldown_secs: 30,
            max_retries: 3,
            enabled: true,
        },
        Rulebook {
            name: "nightly-backup".into(),
            description: Some("Run backup on cron schedule".into()),
            events: vec![EventPattern {
                event_type: EventType::CronFired,
                match_fields: HashMap::new(),
            }],
            conditions: Vec::new(),
            actions: vec![RulebookAction {
                apply: None,
                destroy: None,
                script: Some("/usr/local/bin/backup.sh".into()),
                notify: None,
            }],
            cooldown_secs: 3600,
            max_retries: 1,
            enabled: true,
        },
    ];
    let rb_config = build_rulebook_config(rulebooks);

    // 3. Simulate a daemon tick cycle

    // === Tick 1: Check cron at midnight ===
    println!("--- Tick 1: Midnight cron check ---");
    let time = CronTime {
        minute: 0,
        hour: 0,
        day: 10,
        month: 3,
        weekday: 1,
    };
    let matched_crons = check_cron_schedules(&state, &time);
    for name in &matched_crons {
        println!("  Cron matched: {name}");
        let event = cron_event(name, "2026-03-10T00:00:00Z");
        let result = process_event(&event, &rb_config, &mut state);
        for (rb_name, action) in &result.pending_actions {
            let kind = classify_action(action);
            println!("  → Action: {rb_name} dispatches {kind}");
        }
    }
    println!();

    // === Tick 2: File change detected ===
    println!("--- Tick 2: File change detected ---");
    // Seed initial hash
    state
        .file_hashes
        .insert("/etc/nginx/nginx.conf".into(), "hash-v1".into());

    let mut current_hashes = HashMap::new();
    current_hashes.insert("/etc/nginx/nginx.conf".into(), "hash-v2".into());

    let changed = detect_file_changes(&config.watch_paths, &current_hashes, &mut state);
    for path in &changed {
        println!("  File changed: {path}");
        let event = file_changed_event(path, "2026-03-10T00:01:00Z");
        let result = process_event(&event, &rb_config, &mut state);
        for (rb_name, action) in &result.pending_actions {
            let kind = classify_action(action);
            println!("  → Action: {rb_name} dispatches {kind}");

            // Format event log entry
            let log_line = format_event_log(&result.event, &[(rb_name.clone(), kind)]);
            println!("  → Log: {log_line}");
        }
    }
    println!();

    // === Tick 3: Metric threshold check ===
    println!("--- Tick 3: Metric threshold check ---");
    let mut metric_values = HashMap::new();
    metric_values.insert("cpu_percent".into(), 85.0);

    // First check: consecutive=2, so won't fire yet
    let events = check_metrics(&config.metric_thresholds, &metric_values, &mut state);
    println!(
        "  CPU at 85%: {} events fired (need 2 consecutive)",
        events.len()
    );

    // Second check: now fires
    let events = check_metrics(&config.metric_thresholds, &metric_values, &mut state);
    println!("  CPU still 85%: {} events fired", events.len());
    for event in &events {
        println!(
            "  → MetricThreshold: {} {} {}",
            event.payload.get("metric").unwrap(),
            event.payload.get("operator").unwrap(),
            event.payload.get("threshold").unwrap(),
        );
    }
    println!();

    // === Summary ===
    let summary = daemon_summary(&config, &state);
    println!("=== Daemon Summary ===");
    println!("  Events processed: {}", summary.events_processed);
    println!("  Actions dispatched: {}", summary.actions_dispatched);
    println!("  Cron schedules: {}", summary.cron_schedules);
    println!("  Watched paths: {}", summary.watched_paths);
    println!("  Metric thresholds: {}", summary.metric_thresholds);
    println!("  Shutdown: {}", summary.shutdown);

    // === Action classification demo ===
    println!();
    println!("=== Action Kinds ===");
    for kind in [
        ActionKind::Apply,
        ActionKind::Destroy,
        ActionKind::Script,
        ActionKind::Notify,
        ActionKind::Unknown,
    ] {
        println!("  {kind}");
    }
}
