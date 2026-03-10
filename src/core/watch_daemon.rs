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
