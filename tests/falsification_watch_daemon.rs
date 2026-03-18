//! FJ-3102: Watch daemon and apply gates falsification tests.
//! Usage: cargo test --test falsification_watch_daemon

use forjar::core::metric_source::{MetricThreshold, ThresholdOp};
use forjar::core::types::{ApplyAction, EventPattern, EventType, Rulebook, RulebookAction};
use forjar::core::watch_daemon::{
    build_rulebook_config, cron_event, file_changed_event, process_event, DaemonState,
    WatchDaemonConfig,
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
fn daemon_state_initializes_cron() {
    let config = make_config();
    let state = DaemonState::new(&config);
    assert_eq!(state.cron_parsed.len(), 2);
    assert_eq!(state.events_processed, 0);
}

#[test]
fn daemon_state_skips_bad_cron() {
    let config = WatchDaemonConfig {
        cron_schedules: vec![
            ("good".into(), "0 12 * * *".into()),
            ("bad".into(), "not-valid".into()),
            ("also-bad".into(), "1 2 3".into()),
        ],
        ..Default::default()
    };
    let state = DaemonState::new(&config);
    assert_eq!(state.cron_parsed.len(), 1);
    assert_eq!(state.cron_parsed[0].0, "good");
}

// ── event processing ──

#[test]
fn process_event_file_changed_matches() {
    let rb = test_rulebook("repair", EventType::FileChanged, 0);
    let rb_config = build_rulebook_config(vec![rb]);
    let mut state = DaemonState::new(&make_config());

    let event = file_changed_event("/etc/app.conf", "2026-03-10T12:00:00Z");
    let result = process_event(&event, &rb_config, &mut state);

    assert_eq!(result.pending_actions.len(), 1);
    assert_eq!(result.pending_actions[0].0, "repair");
    assert_eq!(state.events_processed, 1);
    assert_eq!(state.actions_dispatched, 1);
}

#[test]
fn process_event_cron_fired_matches() {
    let rb = test_rulebook("backup", EventType::CronFired, 0);
    let rb_config = build_rulebook_config(vec![rb]);
    let mut state = DaemonState::new(&make_config());

    let event = cron_event("midnight", "2026-03-10T00:00:00Z");
    let result = process_event(&event, &rb_config, &mut state);

    assert_eq!(result.pending_actions.len(), 1);
}

#[test]
fn process_event_no_match_different_type() {
    let rb = test_rulebook("only-cron", EventType::CronFired, 0);
    let rb_config = build_rulebook_config(vec![rb]);
    let mut state = DaemonState::new(&make_config());

    let event = file_changed_event("/etc/app.conf", "2026-03-10T12:00:00Z");
    let result = process_event(&event, &rb_config, &mut state);

    assert!(result.pending_actions.is_empty());
    assert_eq!(state.actions_dispatched, 0);
}
