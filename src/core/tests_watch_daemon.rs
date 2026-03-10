//! Unit tests for watch_daemon module (FJ-3102).

use super::metric_source::{self, MetricEvalResult, MetricThreshold};
use super::types::{
    ApplyAction, EventPattern, EventType, InfraEvent, NotifyAction, Rulebook, RulebookAction,
};
use super::watch_daemon::*;
use crate::core::cron_source::CronTime;
use std::collections::HashMap;

fn test_rulebook(name: &str, event_type: EventType) -> Rulebook {
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
        cooldown_secs: 0,
        max_retries: 3,
        enabled: true,
    }
}

fn test_config() -> WatchDaemonConfig {
    WatchDaemonConfig {
        poll_interval_secs: 5,
        cron_schedules: vec![("daily".into(), "0 0 * * *".into())],
        metric_thresholds: Vec::new(),
        webhook_port: 0,
        watch_paths: vec!["/etc/nginx/nginx.conf".into()],
        event_buffer_size: 100,
        event_logging: true,
    }
}

#[test]
fn daemon_state_new_parses_cron() {
    let config = test_config();
    let state = DaemonState::new(&config);
    assert_eq!(state.cron_parsed.len(), 1);
    assert_eq!(state.cron_parsed[0].0, "daily");
    assert_eq!(state.events_processed, 0);
    assert_eq!(state.actions_dispatched, 0);
    assert!(!state.shutdown);
}

#[test]
fn daemon_state_invalid_cron_skipped() {
    let config = WatchDaemonConfig {
        cron_schedules: vec![
            ("valid".into(), "0 0 * * *".into()),
            ("invalid".into(), "bad cron".into()),
        ],
        ..Default::default()
    };
    let state = DaemonState::new(&config);
    assert_eq!(state.cron_parsed.len(), 1);
    assert_eq!(state.cron_parsed[0].0, "valid");
}

#[test]
fn process_event_fires_matching_rulebook() {
    let rb = test_rulebook("restart", EventType::FileChanged);
    let rb_config = build_rulebook_config(vec![rb]);
    let mut state = DaemonState::new(&test_config());

    let event = InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-10T00:00:00Z".into(),
        machine: None,
        payload: HashMap::new(),
    };

    let result = process_event(&event, &rb_config, &mut state);
    assert_eq!(result.pending_actions.len(), 1);
    assert_eq!(result.pending_actions[0].0, "restart");
    assert_eq!(state.events_processed, 1);
    assert_eq!(state.actions_dispatched, 1);
}

#[test]
fn process_event_no_match() {
    let rb = test_rulebook("restart", EventType::FileChanged);
    let rb_config = build_rulebook_config(vec![rb]);
    let mut state = DaemonState::new(&test_config());

    let event = InfraEvent {
        event_type: EventType::CronFired,
        timestamp: "2026-03-10T00:00:00Z".into(),
        machine: None,
        payload: HashMap::new(),
    };

    let result = process_event(&event, &rb_config, &mut state);
    assert!(result.pending_actions.is_empty());
    assert_eq!(state.events_processed, 1);
    assert_eq!(state.actions_dispatched, 0);
}

#[test]
fn process_event_multiple_rulebooks() {
    let rb1 = test_rulebook("rb1", EventType::FileChanged);
    let rb2 = test_rulebook("rb2", EventType::FileChanged);
    let rb_config = build_rulebook_config(vec![rb1, rb2]);
    let mut state = DaemonState::new(&test_config());

    let event = InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-10T00:00:00Z".into(),
        machine: None,
        payload: HashMap::new(),
    };

    let result = process_event(&event, &rb_config, &mut state);
    assert_eq!(result.pending_actions.len(), 2);
    assert_eq!(state.actions_dispatched, 2);
}

#[test]
fn classify_action_types() {
    let apply = RulebookAction {
        apply: Some(ApplyAction {
            file: "f.yaml".into(),
            subset: vec![],
            tags: vec![],
            machine: None,
        }),
        destroy: None,
        script: None,
        notify: None,
    };
    assert_eq!(classify_action(&apply), ActionKind::Apply);

    let script = RulebookAction {
        apply: None,
        destroy: None,
        script: Some("echo hello".into()),
        notify: None,
    };
    assert_eq!(classify_action(&script), ActionKind::Script);

    let notify = RulebookAction {
        apply: None,
        destroy: None,
        script: None,
        notify: Some(NotifyAction {
            channel: "slack".into(),
            message: "hi".into(),
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

#[test]
fn check_cron_schedules_match() {
    let config = WatchDaemonConfig {
        cron_schedules: vec![("midnight".into(), "0 0 * * *".into())],
        ..Default::default()
    };
    let state = DaemonState::new(&config);
    let time = CronTime {
        minute: 0,
        hour: 0,
        day: 1,
        month: 1,
        weekday: 3,
    };
    let matched = check_cron_schedules(&state, &time);
    assert_eq!(matched, vec!["midnight"]);
}

#[test]
fn check_cron_schedules_no_match() {
    let config = WatchDaemonConfig {
        cron_schedules: vec![("midnight".into(), "0 0 * * *".into())],
        ..Default::default()
    };
    let state = DaemonState::new(&config);
    let time = CronTime {
        minute: 30,
        hour: 12,
        day: 1,
        month: 1,
        weekday: 3,
    };
    let matched = check_cron_schedules(&state, &time);
    assert!(matched.is_empty());
}

#[test]
fn cron_event_construction() {
    let event = cron_event("nightly-backup", "2026-03-10T00:00:00Z");
    assert_eq!(event.event_type, EventType::CronFired);
    assert_eq!(event.payload.get("schedule").unwrap(), "nightly-backup");
}

#[test]
fn file_changed_event_construction() {
    let event = file_changed_event("/etc/nginx/nginx.conf", "2026-03-10T12:00:00Z");
    assert_eq!(event.event_type, EventType::FileChanged);
    assert_eq!(event.payload.get("path").unwrap(), "/etc/nginx/nginx.conf");
}

#[test]
fn metric_threshold_event_construction() {
    let result = MetricEvalResult {
        name: "cpu_percent".into(),
        current: 95.0,
        threshold: 80.0,
        operator: metric_source::ThresholdOp::Gt,
        violated: true,
        should_fire: true,
    };
    let event = metric_threshold_event(&result, "2026-03-10T12:00:00Z");
    assert_eq!(event.event_type, EventType::MetricThreshold);
    assert_eq!(event.payload.get("metric").unwrap(), "cpu_percent");
    assert_eq!(event.payload.get("current").unwrap(), "95");
    assert_eq!(event.payload.get("threshold").unwrap(), "80");
}

#[test]
fn detect_file_changes_initial_no_fire() {
    let mut state = DaemonState::new(&test_config());
    let paths = vec!["/etc/foo".to_string()];
    let mut hashes = HashMap::new();
    hashes.insert("/etc/foo".to_string(), "hash1".to_string());
    let changes = detect_file_changes(&paths, &hashes, &mut state);
    assert!(changes.is_empty());
    assert_eq!(state.file_hashes.get("/etc/foo").unwrap(), "hash1");
}

#[test]
fn detect_file_changes_fires_on_change() {
    let mut state = DaemonState::new(&test_config());
    let paths = vec!["/etc/foo".to_string()];
    state
        .file_hashes
        .insert("/etc/foo".to_string(), "hash1".to_string());
    let mut hashes = HashMap::new();
    hashes.insert("/etc/foo".to_string(), "hash2".to_string());
    let changes = detect_file_changes(&paths, &hashes, &mut state);
    assert_eq!(changes, vec!["/etc/foo"]);
    assert_eq!(state.file_hashes.get("/etc/foo").unwrap(), "hash2");
}

#[test]
fn detect_file_changes_no_change() {
    let mut state = DaemonState::new(&test_config());
    let paths = vec!["/etc/foo".to_string()];
    state
        .file_hashes
        .insert("/etc/foo".to_string(), "hash1".to_string());
    let mut hashes = HashMap::new();
    hashes.insert("/etc/foo".to_string(), "hash1".to_string());
    let changes = detect_file_changes(&paths, &hashes, &mut state);
    assert!(changes.is_empty());
}

#[test]
fn check_metrics_fires_threshold() {
    let thresholds = vec![MetricThreshold {
        name: "cpu".into(),
        operator: metric_source::ThresholdOp::Gt,
        value: 80.0,
        consecutive: 1,
    }];
    let mut values = HashMap::new();
    values.insert("cpu".into(), 95.0);
    let mut state = DaemonState::new(&test_config());
    let events = check_metrics(&thresholds, &values, &mut state);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, EventType::MetricThreshold);
}

#[test]
fn check_metrics_no_fire_below_threshold() {
    let thresholds = vec![MetricThreshold {
        name: "cpu".into(),
        operator: metric_source::ThresholdOp::Gt,
        value: 80.0,
        consecutive: 1,
    }];
    let mut values = HashMap::new();
    values.insert("cpu".into(), 50.0);
    let mut state = DaemonState::new(&test_config());
    let events = check_metrics(&thresholds, &values, &mut state);
    assert!(events.is_empty());
}

#[test]
fn format_event_log_json() {
    let event = cron_event("backup", "2026-03-10T00:00:00Z");
    let actions = vec![("backup-rb".to_string(), ActionKind::Apply)];
    let json = format_event_log(&event, &actions);
    assert!(json.contains("cron_fired"));
    assert!(json.contains("backup-rb"));
    assert!(json.contains("apply"));
}

#[test]
fn format_event_log_empty_actions() {
    let event = file_changed_event("/etc/test", "2026-03-10T12:00:00Z");
    let json = format_event_log(&event, &[]);
    assert!(json.contains("file_changed"));
    assert!(json.contains("\"actions\":[]"));
}

#[test]
fn daemon_summary_reflects_state() {
    let config = WatchDaemonConfig {
        watch_paths: vec!["/a".into(), "/b".into()],
        cron_schedules: vec![("daily".into(), "0 0 * * *".into())],
        metric_thresholds: vec![MetricThreshold {
            name: "cpu".into(),
            operator: metric_source::ThresholdOp::Gt,
            value: 80.0,
            consecutive: 1,
        }],
        ..Default::default()
    };
    let state = DaemonState::new(&config);
    let summary = daemon_summary(&config, &state);
    assert_eq!(summary.events_processed, 0);
    assert_eq!(summary.actions_dispatched, 0);
    assert_eq!(summary.cron_schedules, 1);
    assert_eq!(summary.watched_paths, 2);
    assert_eq!(summary.metric_thresholds, 1);
    assert!(!summary.shutdown);
}

#[test]
fn action_kind_display() {
    assert_eq!(ActionKind::Apply.to_string(), "apply");
    assert_eq!(ActionKind::Destroy.to_string(), "destroy");
    assert_eq!(ActionKind::Script.to_string(), "script");
    assert_eq!(ActionKind::Notify.to_string(), "notify");
    assert_eq!(ActionKind::Unknown.to_string(), "unknown");
}

#[test]
fn build_rulebook_config_wraps() {
    let rbs = vec![test_rulebook("a", EventType::Manual)];
    let config = build_rulebook_config(rbs);
    assert_eq!(config.rulebooks.len(), 1);
    assert_eq!(config.rulebooks[0].name, "a");
}

#[test]
fn default_daemon_config() {
    let config = WatchDaemonConfig::default();
    assert_eq!(config.poll_interval_secs, 5);
    assert_eq!(config.webhook_port, 0);
    assert!(config.cron_schedules.is_empty());
    assert!(config.metric_thresholds.is_empty());
    assert!(config.watch_paths.is_empty());
    assert_eq!(config.event_buffer_size, 1024);
    assert!(config.event_logging);
}

#[test]
fn process_event_cooldown_blocks_second() {
    let mut rb = test_rulebook("restart", EventType::FileChanged);
    rb.cooldown_secs = 60;
    let rb_config = build_rulebook_config(vec![rb]);
    let mut state = DaemonState::new(&test_config());

    let event = InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-10T00:00:00Z".into(),
        machine: None,
        payload: HashMap::new(),
    };

    let r1 = process_event(&event, &rb_config, &mut state);
    assert_eq!(r1.pending_actions.len(), 1);

    let r2 = process_event(&event, &rb_config, &mut state);
    assert!(r2.pending_actions.is_empty());
    assert_eq!(state.events_processed, 2);
    assert_eq!(state.actions_dispatched, 1);
}

#[test]
fn end_to_end_cron_to_action() {
    let config = WatchDaemonConfig {
        cron_schedules: vec![("nightly".into(), "0 0 * * *".into())],
        ..Default::default()
    };
    let mut state = DaemonState::new(&config);

    let rb = test_rulebook("nightly-rb", EventType::CronFired);
    let rb_config = build_rulebook_config(vec![rb]);

    let time = CronTime {
        minute: 0,
        hour: 0,
        day: 10,
        month: 3,
        weekday: 2,
    };
    let matched = check_cron_schedules(&state, &time);
    assert_eq!(matched, vec!["nightly"]);

    let event = cron_event(&matched[0], "2026-03-10T00:00:00Z");
    let result = process_event(&event, &rb_config, &mut state);

    assert_eq!(result.pending_actions.len(), 1);
    assert_eq!(
        classify_action(&result.pending_actions[0].1),
        ActionKind::Apply
    );
}
