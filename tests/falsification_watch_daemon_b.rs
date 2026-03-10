//! FJ-3102: Watch daemon and apply gates falsification tests.
//! Usage: cargo test --test falsification_watch_daemon
#![allow(dead_code)]

use forjar::core::cron_source::CronTime;
use forjar::core::metric_source::{MetricThreshold, ThresholdOp};
use forjar::core::types::{
    ApplyAction, DestroyAction, EventPattern, EventType, InfraEvent, NotifyAction, Rulebook,
    RulebookAction,
};
use forjar::core::watch_daemon::{
    build_rulebook_config, check_cron_schedules, check_metrics, classify_action, cron_event,
    daemon_summary, detect_file_changes, file_changed_event, format_event_log,
    metric_threshold_event, process_event, ActionKind, DaemonState, WatchDaemonConfig,
};
use std::collections::HashMap;

// ── helpers ──

fn test_rulebook(name: &str, event_type: EventType, cooldown: u64) -> Rulebook {
    Rulebook {
        name: name.into(),
        description: None,
        events: vec![EventPattern {
            event_type,
            match_fields: HashMap::new(),
        }],
        conditions: Vec::new(),
        actions: vec![RulebookAction {
            apply: Some(ApplyAction {
                file: "forjar.yaml".into(),
                subset: vec!["nginx".into()],
                tags: Vec::new(),
                machine: None,
            }),
            destroy: None,
            script: None,
            notify: None,
        }],
        cooldown_secs: cooldown,
        max_retries: 3,
        enabled: true,
    }
}

fn make_config() -> WatchDaemonConfig {
    WatchDaemonConfig {
        poll_interval_secs: 5,
        cron_schedules: vec![
            ("midnight".into(), "0 0 * * *".into()),
            ("hourly".into(), "0 * * * *".into()),
        ],
        metric_thresholds: vec![MetricThreshold {
            name: "cpu".into(),
            operator: ThresholdOp::Gt,
            value: 80.0,
            consecutive: 1,
        }],
        webhook_port: 8484,
        watch_paths: vec!["/etc/app.conf".into(), "/var/data/state".into()],
        event_buffer_size: 512,
        event_logging: true,
    }
}

// ── daemon initialization ──

#[test]
fn process_event_cooldown_blocks() {
    let rb = test_rulebook("throttled", EventType::FileChanged, 3600);
    let rb_config = build_rulebook_config(vec![rb]);
    let mut state = DaemonState::new(&make_config());

    let event = file_changed_event("/path", "2026-03-10T12:00:00Z");

    // First: fires
    let r1 = process_event(&event, &rb_config, &mut state);
    assert_eq!(r1.pending_actions.len(), 1);

    // Second: cooldown blocks
    let r2 = process_event(&event, &rb_config, &mut state);
    assert!(r2.pending_actions.is_empty());

    // Counters track both events
    assert_eq!(state.events_processed, 2);
    assert_eq!(state.actions_dispatched, 1);
}

#[test]
fn process_event_disabled_rulebook() {
    let mut rb = test_rulebook("disabled", EventType::FileChanged, 0);
    rb.enabled = false;
    let rb_config = build_rulebook_config(vec![rb]);
    let mut state = DaemonState::new(&make_config());

    let event = file_changed_event("/path", "2026-03-10T12:00:00Z");
    let result = process_event(&event, &rb_config, &mut state);

    assert!(result.pending_actions.is_empty());
}

#[test]
fn process_event_multiple_rulebooks_fire() {
    let rb1 = test_rulebook("rb1", EventType::Manual, 0);
    let rb2 = test_rulebook("rb2", EventType::Manual, 0);
    let rb_config = build_rulebook_config(vec![rb1, rb2]);
    let mut state = DaemonState::new(&make_config());

    let event = InfraEvent {
        event_type: EventType::Manual,
        timestamp: "2026-03-10T12:00:00Z".into(),
        machine: None,
        payload: HashMap::new(),
    };

    let result = process_event(&event, &rb_config, &mut state);
    assert_eq!(result.pending_actions.len(), 2);
    assert_eq!(state.actions_dispatched, 2);
}

// ── action classification ──

#[test]
fn classify_all_action_kinds() {
    let apply = RulebookAction {
        apply: Some(ApplyAction {
            file: "f".into(),
            subset: vec![],
            tags: vec![],
            machine: None,
        }),
        destroy: None,
        script: None,
        notify: None,
    };
    assert_eq!(classify_action(&apply), ActionKind::Apply);

    let destroy = RulebookAction {
        apply: None,
        destroy: Some(DestroyAction {
            file: "f".into(),
            resources: vec![],
        }),
        script: None,
        notify: None,
    };
    assert_eq!(classify_action(&destroy), ActionKind::Destroy);

    let script = RulebookAction {
        apply: None,
        destroy: None,
        script: Some("echo hi".into()),
        notify: None,
    };
    assert_eq!(classify_action(&script), ActionKind::Script);

    let notify = RulebookAction {
        apply: None,
        destroy: None,
        script: None,
        notify: Some(NotifyAction {
            channel: "ch".into(),
            message: "msg".into(),
        }),
    };
    assert_eq!(classify_action(&notify), ActionKind::Notify);

    let unknown = RulebookAction {
        apply: None,
        destroy: None,
        script: None,
        notify: None,
    };
    assert_eq!(classify_action(&unknown), ActionKind::Unknown);
}

// ── cron schedule checking ──

#[test]
fn cron_midnight_matches() {
    let state = DaemonState::new(&make_config());
    let time = CronTime {
        minute: 0,
        hour: 0,
        day: 1,
        month: 1,
        weekday: 0,
    };
    let matched = check_cron_schedules(&state, &time);
    // Both midnight and hourly fire at 00:00
    assert!(matched.contains(&"midnight".to_string()));
    assert!(matched.contains(&"hourly".to_string()));
}

#[test]
fn cron_noon_only_hourly() {
    let state = DaemonState::new(&make_config());
    let time = CronTime {
        minute: 0,
        hour: 12,
        day: 1,
        month: 1,
        weekday: 0,
    };
    let matched = check_cron_schedules(&state, &time);
    assert!(matched.contains(&"hourly".to_string()));
    assert!(!matched.contains(&"midnight".to_string()));
}

#[test]
fn cron_non_zero_minute_no_match() {
    let state = DaemonState::new(&make_config());
    let time = CronTime {
        minute: 15,
        hour: 3,
        day: 1,
        month: 1,
        weekday: 0,
    };
    let matched = check_cron_schedules(&state, &time);
    assert!(matched.is_empty());
}

// ── file change detection ──

#[test]
fn file_change_initial_load_no_fire() {
    let config = make_config();
    let mut state = DaemonState::new(&config);
    let mut hashes = HashMap::new();
    hashes.insert("/etc/app.conf".into(), "hash1".into());

    let changes = detect_file_changes(&config.watch_paths, &hashes, &mut state);
    assert!(changes.is_empty());
}

#[test]
fn file_change_fires_on_modification() {
    let config = make_config();
    let mut state = DaemonState::new(&config);
    state
        .file_hashes
        .insert("/etc/app.conf".into(), "old-hash".into());

    let mut hashes = HashMap::new();
    hashes.insert("/etc/app.conf".into(), "new-hash".into());

    let changes = detect_file_changes(&config.watch_paths, &hashes, &mut state);
    assert_eq!(changes, vec!["/etc/app.conf"]);
}

#[test]
fn file_change_no_change_same_hash() {
    let config = make_config();
    let mut state = DaemonState::new(&config);
    state
        .file_hashes
        .insert("/etc/app.conf".into(), "same".into());

    let mut hashes = HashMap::new();
    hashes.insert("/etc/app.conf".into(), "same".into());

    let changes = detect_file_changes(&config.watch_paths, &hashes, &mut state);
    assert!(changes.is_empty());
}

#[test]
fn file_change_multiple_files() {
    let config = make_config();
    let mut state = DaemonState::new(&config);
    state
        .file_hashes
        .insert("/etc/app.conf".into(), "h1".into());
    state
        .file_hashes
        .insert("/var/data/state".into(), "h2".into());

    let mut hashes = HashMap::new();
    hashes.insert("/etc/app.conf".into(), "h1-new".into());
    hashes.insert("/var/data/state".into(), "h2-new".into());

    let changes = detect_file_changes(&config.watch_paths, &hashes, &mut state);
    assert_eq!(changes.len(), 2);
}

// ── metric threshold events ──

#[test]
fn metrics_fire_above_threshold() {
    let mut state = DaemonState::new(&make_config());
    let thresholds = vec![MetricThreshold {
        name: "cpu".into(),
        operator: ThresholdOp::Gt,
        value: 80.0,
        consecutive: 1,
    }];
    let mut values = HashMap::new();
    values.insert("cpu".into(), 95.0);

    let events = check_metrics(&thresholds, &values, &mut state);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, EventType::MetricThreshold);
}

#[test]
fn metrics_no_fire_below_threshold() {
    let mut state = DaemonState::new(&make_config());
    let thresholds = vec![MetricThreshold {
        name: "cpu".into(),
        operator: ThresholdOp::Gt,
        value: 80.0,
        consecutive: 1,
    }];
    let mut values = HashMap::new();
    values.insert("cpu".into(), 50.0);

    let events = check_metrics(&thresholds, &values, &mut state);
    assert!(events.is_empty());
}

#[test]
fn metrics_consecutive_threshold() {
    let mut state = DaemonState::new(&make_config());
    let thresholds = vec![MetricThreshold {
        name: "mem".into(),
        operator: ThresholdOp::Gte,
        value: 90.0,
        consecutive: 3,
    }];
    let mut values = HashMap::new();
    values.insert("mem".into(), 95.0);

    // 1st and 2nd: no fire (need 3 consecutive)
    assert!(check_metrics(&thresholds, &values, &mut state).is_empty());
    assert!(check_metrics(&thresholds, &values, &mut state).is_empty());
    // 3rd: fires
    assert_eq!(check_metrics(&thresholds, &values, &mut state).len(), 1);
}

// ── event log formatting ──

#[test]
fn event_log_has_all_fields() {
    let event = cron_event("nightly", "2026-03-10T00:00:00Z");
    let actions = vec![
        ("backup-rb".into(), ActionKind::Script),
        ("notify-rb".into(), ActionKind::Notify),
    ];
    let json = format_event_log(&event, &actions);

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["event_type"], "cron_fired");
    assert_eq!(parsed["timestamp"], "2026-03-10T00:00:00Z");
    assert_eq!(parsed["actions"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["actions"][0]["rulebook"], "backup-rb");
    assert_eq!(parsed["actions"][0]["action"], "script");
    assert_eq!(parsed["actions"][1]["action"], "notify");
}

// ── daemon summary ──

#[test]
fn daemon_summary_tracks_state() {
    let config = make_config();
    let rb = test_rulebook("rb", EventType::FileChanged, 0);
    let rb_config = build_rulebook_config(vec![rb]);
    let mut state = DaemonState::new(&config);

    // Process 3 events
    for _ in 0..3 {
        let event = file_changed_event("/etc/app.conf", "t");
        process_event(&event, &rb_config, &mut state);
    }

    let summary = daemon_summary(&config, &state);
    assert_eq!(summary.events_processed, 3);
    assert_eq!(summary.actions_dispatched, 3);
    assert_eq!(summary.cron_schedules, 2);
    assert_eq!(summary.watched_paths, 2);
    assert_eq!(summary.metric_thresholds, 1);
    assert!(!summary.shutdown);
}

// ── event constructors ──

#[test]
fn metric_event_carries_payload() {
    use forjar::core::metric_source::MetricEvalResult;
    let result = MetricEvalResult {
        name: "disk_used_gb".into(),
        current: 450.0,
        threshold: 400.0,
        operator: ThresholdOp::Gt,
        violated: true,
        should_fire: true,
    };
    let event = metric_threshold_event(&result, "2026-03-10T12:00:00Z");
    assert_eq!(event.payload.get("metric").unwrap(), "disk_used_gb");
    assert_eq!(event.payload.get("current").unwrap(), "450");
    assert_eq!(event.payload.get("threshold").unwrap(), "400");
    assert_eq!(event.payload.get("operator").unwrap(), ">");
}

// ── end-to-end pipeline ──

#[test]
fn e2e_file_change_to_action() {
    let config = make_config();
    let mut state = DaemonState::new(&config);

    // Seed initial file hash
    state
        .file_hashes
        .insert("/etc/app.conf".into(), "v1".into());

    // Detect change
    let mut hashes = HashMap::new();
    hashes.insert("/etc/app.conf".into(), "v2".into());
    let changes = detect_file_changes(&config.watch_paths, &hashes, &mut state);
    assert_eq!(changes.len(), 1);

    // Create event
    let event = file_changed_event(&changes[0], "2026-03-10T12:00:00Z");
    assert_eq!(event.event_type, EventType::FileChanged);

    // Process through rulebooks
    let mut rb = test_rulebook("repair", EventType::FileChanged, 0);
    rb.events[0]
        .match_fields
        .insert("path".into(), "/etc/app.conf".into());
    let rb_config = build_rulebook_config(vec![rb]);
    let result = process_event(&event, &rb_config, &mut state);

    // Verify action
    assert_eq!(result.pending_actions.len(), 1);
    assert_eq!(
        classify_action(&result.pending_actions[0].1),
        ActionKind::Apply
    );

    // Format log
    let log = format_event_log(&result.event, &[("repair".into(), ActionKind::Apply)]);
    assert!(log.contains("file_changed"));
    assert!(log.contains("repair"));
}

#[test]
fn e2e_metric_to_notification() {
    let config = make_config();
    let mut state = DaemonState::new(&config);

    // Create metric event
    let thresholds = vec![MetricThreshold {
        name: "cpu".into(),
        operator: ThresholdOp::Gt,
        value: 80.0,
        consecutive: 1,
    }];
    let mut values = HashMap::new();
    values.insert("cpu".into(), 95.0);

    let events = check_metrics(&thresholds, &values, &mut state);
    assert_eq!(events.len(), 1);

    // Process through notify rulebook
    let rb = Rulebook {
        name: "cpu-alert".into(),
        description: None,
        events: vec![EventPattern {
            event_type: EventType::MetricThreshold,
            match_fields: HashMap::new(),
        }],
        conditions: Vec::new(),
        actions: vec![RulebookAction {
            apply: None,
            destroy: None,
            script: None,
            notify: Some(NotifyAction {
                channel: "https://hooks.slack.com/xxx".into(),
                message: "CPU critical".into(),
            }),
        }],
        cooldown_secs: 0,
        max_retries: 1,
        enabled: true,
    };
    let rb_config = build_rulebook_config(vec![rb]);
    let result = process_event(&events[0], &rb_config, &mut state);

    assert_eq!(result.pending_actions.len(), 1);
    assert_eq!(
        classify_action(&result.pending_actions[0].1),
        ActionKind::Notify
    );
}
