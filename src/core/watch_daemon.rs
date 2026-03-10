//! FJ-3102: Watch daemon orchestrator — pure logic for event-driven automation.
//!
//! Provides the testable decision logic for the watch daemon:
//! event classification, action dispatch planning, event log formatting,
//! and daemon lifecycle state management. I/O (threads, channels, HTTP)
//! remains in `cli::observe`.

use crate::core::cron_source::{self, CronSchedule, CronTime};
use crate::core::metric_source::{self, MetricEvalResult, MetricThreshold, ThresholdTracker};
use crate::core::rules_runtime::{self, EvalResult};
use crate::core::types::{
    CooldownTracker, EventType, InfraEvent, Rulebook, RulebookAction, RulebookConfig,
};
use std::collections::HashMap;

/// Configuration for the watch daemon.
#[derive(Debug, Clone)]
pub struct WatchDaemonConfig {
    /// Polling interval in seconds for file watcher and metrics.
    pub poll_interval_secs: u64,
    /// Cron schedules (rulebook name → cron expression).
    pub cron_schedules: Vec<(String, String)>,
    /// Metric thresholds to poll.
    pub metric_thresholds: Vec<MetricThreshold>,
    /// Webhook listener port (0 = disabled).
    pub webhook_port: u16,
    /// Paths to watch for file changes.
    pub watch_paths: Vec<String>,
    /// Maximum events to buffer before processing.
    pub event_buffer_size: usize,
    /// Whether to log events to events.jsonl.
    pub event_logging: bool,
}

impl Default for WatchDaemonConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 5,
            cron_schedules: Vec::new(),
            metric_thresholds: Vec::new(),
            webhook_port: 0,
            watch_paths: Vec::new(),
            event_buffer_size: 1024,
            event_logging: true,
        }
    }
}

/// Runtime state for the watch daemon (testable, no I/O).
#[derive(Debug)]
pub struct DaemonState {
    /// Cooldown tracker for rulebook deduplication.
    pub cooldown: CooldownTracker,
    /// Metric threshold tracker for consecutive violations.
    pub metrics: ThresholdTracker,
    /// Parsed cron schedules (name, schedule).
    pub cron_parsed: Vec<(String, CronSchedule)>,
    /// Event counter since daemon start.
    pub events_processed: u64,
    /// Action counter since daemon start.
    pub actions_dispatched: u64,
    /// Last file hashes for change detection.
    pub file_hashes: HashMap<String, String>,
    /// Whether the daemon should shut down.
    pub shutdown: bool,
}

impl DaemonState {
    /// Create a new daemon state from configuration.
    pub fn new(config: &WatchDaemonConfig) -> Self {
        let cron_parsed = config
            .cron_schedules
            .iter()
            .filter_map(|(name, expr)| {
                cron_source::parse_cron(expr)
                    .ok()
                    .map(|s| (name.clone(), s))
            })
            .collect();

        Self {
            cooldown: CooldownTracker::default(),
            metrics: ThresholdTracker::default(),
            cron_parsed,
            events_processed: 0,
            actions_dispatched: 0,
            file_hashes: HashMap::new(),
            shutdown: false,
        }
    }
}

/// Result of processing a single event through the daemon.
#[derive(Debug, Clone)]
pub struct ProcessedEvent {
    /// The original event.
    pub event: InfraEvent,
    /// Evaluation results from all matching rulebooks.
    pub eval_results: Vec<EvalResult>,
    /// Actions that should be dispatched (not blocked by cooldown).
    pub pending_actions: Vec<(String, RulebookAction)>,
}

/// Process an event through rulebook evaluation.
///
/// Pure function: takes event + config + state, returns actions to dispatch.
/// Does NOT execute the actions — that's the caller's responsibility.
pub fn process_event(
    event: &InfraEvent,
    rulebook_config: &RulebookConfig,
    state: &mut DaemonState,
) -> ProcessedEvent {
    state.events_processed += 1;

    let eval_results = rules_runtime::evaluate_event(event, rulebook_config, &mut state.cooldown);

    let pending_actions: Vec<(String, RulebookAction)> = eval_results
        .iter()
        .filter(|r| !r.cooldown_blocked && !r.disabled)
        .flat_map(|r| {
            r.actions
                .iter()
                .map(|a| (r.rulebook.clone(), a.clone()))
                .collect::<Vec<_>>()
        })
        .collect();

    state.actions_dispatched += pending_actions.len() as u64;

    ProcessedEvent {
        event: event.clone(),
        eval_results,
        pending_actions,
    }
}

/// Classify which action type a RulebookAction represents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionKind {
    Apply,
    Destroy,
    Script,
    Notify,
    Unknown,
}

impl std::fmt::Display for ActionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Apply => write!(f, "apply"),
            Self::Destroy => write!(f, "destroy"),
            Self::Script => write!(f, "script"),
            Self::Notify => write!(f, "notify"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Classify a rulebook action into its kind.
pub fn classify_action(action: &RulebookAction) -> ActionKind {
    if action.apply.is_some() {
        ActionKind::Apply
    } else if action.destroy.is_some() {
        ActionKind::Destroy
    } else if action.script.is_some() {
        ActionKind::Script
    } else if action.notify.is_some() {
        ActionKind::Notify
    } else {
        ActionKind::Unknown
    }
}

/// Check which cron schedules match the given time.
pub fn check_cron_schedules(state: &DaemonState, time: &CronTime) -> Vec<String> {
    state
        .cron_parsed
        .iter()
        .filter(|(_, sched)| cron_source::matches(sched, time))
        .map(|(name, _)| name.clone())
        .collect()
}

/// Create a CronFired event for a matched schedule.
pub fn cron_event(schedule_name: &str, timestamp: &str) -> InfraEvent {
    let mut payload = HashMap::new();
    payload.insert("schedule".to_string(), schedule_name.to_string());
    InfraEvent {
        event_type: EventType::CronFired,
        timestamp: timestamp.to_string(),
        machine: None,
        payload,
    }
}

/// Create a FileChanged event.
pub fn file_changed_event(path: &str, timestamp: &str) -> InfraEvent {
    let mut payload = HashMap::new();
    payload.insert("path".to_string(), path.to_string());
    InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: timestamp.to_string(),
        machine: None,
        payload,
    }
}

/// Create a MetricThreshold event from evaluation results.
pub fn metric_threshold_event(result: &MetricEvalResult, timestamp: &str) -> InfraEvent {
    let mut payload = HashMap::new();
    payload.insert("metric".to_string(), result.name.clone());
    payload.insert("current".to_string(), result.current.to_string());
    payload.insert("threshold".to_string(), result.threshold.to_string());
    payload.insert("operator".to_string(), result.operator.to_string());
    InfraEvent {
        event_type: EventType::MetricThreshold,
        timestamp: timestamp.to_string(),
        machine: None,
        payload,
    }
}

/// Evaluate metric thresholds and return events for those that should fire.
pub fn check_metrics(
    thresholds: &[MetricThreshold],
    values: &HashMap<String, f64>,
    state: &mut DaemonState,
) -> Vec<InfraEvent> {
    let results = metric_source::evaluate_metrics(thresholds, values, &mut state.metrics);
    let ts = timestamp_now();
    results
        .iter()
        .filter(|r| r.should_fire)
        .map(|r| metric_threshold_event(r, &ts))
        .collect()
}

/// Detect file changes by comparing hashes.
///
/// Returns a list of changed file paths. Updates `state.file_hashes`.
pub fn detect_file_changes(
    paths: &[String],
    current_hashes: &HashMap<String, String>,
    state: &mut DaemonState,
) -> Vec<String> {
    let mut changed = Vec::new();
    for path in paths {
        let new_hash = current_hashes.get(path);
        let old_hash = state.file_hashes.get(path);
        match (old_hash, new_hash) {
            (Some(old), Some(new)) if old != new => {
                changed.push(path.clone());
            }
            (None, Some(_)) => {
                // First time seeing this file — don't fire on initial load
            }
            _ => {}
        }
    }
    // Update stored hashes
    for (path, hash) in current_hashes {
        state.file_hashes.insert(path.clone(), hash.clone());
    }
    changed
}

/// Format an event log entry as JSON line (for events.jsonl).
pub fn format_event_log(event: &InfraEvent, actions_taken: &[(String, ActionKind)]) -> String {
    let actions_json: Vec<serde_json::Value> = actions_taken
        .iter()
        .map(|(rb, kind)| {
            serde_json::json!({
                "rulebook": rb,
                "action": kind.to_string(),
            })
        })
        .collect();

    let log_entry = serde_json::json!({
        "timestamp": event.timestamp,
        "event_type": event.event_type.to_string(),
        "machine": event.machine,
        "payload": event.payload,
        "actions": actions_json,
    });

    serde_json::to_string(&log_entry).unwrap_or_default()
}

/// Summary of daemon runtime state.
#[derive(Debug, Clone)]
pub struct DaemonSummary {
    /// Total events processed.
    pub events_processed: u64,
    /// Total actions dispatched.
    pub actions_dispatched: u64,
    /// Number of configured cron schedules.
    pub cron_schedules: usize,
    /// Number of watched file paths.
    pub watched_paths: usize,
    /// Number of metric thresholds.
    pub metric_thresholds: usize,
    /// Whether daemon is shutting down.
    pub shutdown: bool,
}

/// Get a summary of the current daemon state.
pub fn daemon_summary(config: &WatchDaemonConfig, state: &DaemonState) -> DaemonSummary {
    DaemonSummary {
        events_processed: state.events_processed,
        actions_dispatched: state.actions_dispatched,
        cron_schedules: state.cron_parsed.len(),
        watched_paths: config.watch_paths.len(),
        metric_thresholds: config.metric_thresholds.len(),
        shutdown: state.shutdown,
    }
}

/// Build a RulebookConfig from a list of Rulebook structs.
pub fn build_rulebook_config(rulebooks: Vec<Rulebook>) -> RulebookConfig {
    RulebookConfig { rulebooks }
}

fn timestamp_now() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}Z", dur.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{
        ApplyAction, EventPattern, EventType, NotifyAction, Rulebook, RulebookAction,
    };

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

        // First call: stores initial hashes, no changes reported
        let changes = detect_file_changes(&paths, &hashes, &mut state);
        assert!(changes.is_empty());
        assert_eq!(state.file_hashes.get("/etc/foo").unwrap(), "hash1");
    }

    #[test]
    fn detect_file_changes_fires_on_change() {
        let mut state = DaemonState::new(&test_config());
        let paths = vec!["/etc/foo".to_string()];

        // Seed initial hash
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

        // First fire — succeeds
        let r1 = process_event(&event, &rb_config, &mut state);
        assert_eq!(r1.pending_actions.len(), 1);

        // Second fire — blocked by cooldown
        let r2 = process_event(&event, &rb_config, &mut state);
        assert!(r2.pending_actions.is_empty());
        assert_eq!(state.events_processed, 2);
        // Only 1 action dispatched total (second was blocked)
        assert_eq!(state.actions_dispatched, 1);
    }

    #[test]
    fn end_to_end_cron_to_action() {
        // Setup: cron schedule + matching rulebook
        let config = WatchDaemonConfig {
            cron_schedules: vec![("nightly".into(), "0 0 * * *".into())],
            ..Default::default()
        };
        let mut state = DaemonState::new(&config);

        let rb = test_rulebook("nightly-rb", EventType::CronFired);
        let rb_config = build_rulebook_config(vec![rb]);

        // Step 1: Check cron at midnight
        let time = CronTime {
            minute: 0,
            hour: 0,
            day: 10,
            month: 3,
            weekday: 2,
        };
        let matched = check_cron_schedules(&state, &time);
        assert_eq!(matched, vec!["nightly"]);

        // Step 2: Create cron event
        let event = cron_event(&matched[0], "2026-03-10T00:00:00Z");

        // Step 3: Process through rulebook
        let result = process_event(&event, &rb_config, &mut state);

        // Step 4: Verify action was produced
        assert_eq!(result.pending_actions.len(), 1);
        assert_eq!(
            classify_action(&result.pending_actions[0].1),
            ActionKind::Apply
        );
    }
}
