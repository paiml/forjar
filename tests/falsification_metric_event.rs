//! FJ-3105/3100: Metric thresholds and event-driven rulebook matching.
//!
//! Popperian rejection criteria for:
//! - FJ-3105: evaluate_threshold (gt/gte/lt/lte/eq), ThresholdTracker (consecutive,
//!   reset on ok), evaluate_metrics (multi, missing, consecutive firing), ThresholdOp Display
//! - FJ-3100: event_matches_pattern (type match, payload match, mismatch),
//!   event_matches_rulebook (enabled/disabled, multi-pattern), RulebookAction type,
//!   CooldownTracker (can_fire, record_fire), EventType Display
//!
//! Usage: cargo test --test falsification_metric_event

use forjar::core::metric_source::{
    evaluate_metrics, evaluate_threshold, MetricThreshold, ThresholdOp, ThresholdTracker,
};
use forjar::core::types::{
    event_matches_pattern, event_matches_rulebook, CooldownTracker, EventPattern, EventType,
    InfraEvent, Rulebook, RulebookAction,
};
use std::collections::HashMap;

// ============================================================================
// FJ-3105: evaluate_threshold
// ============================================================================

fn threshold(name: &str, op: ThresholdOp, value: f64) -> MetricThreshold {
    MetricThreshold {
        name: name.into(),
        operator: op,
        value,
        consecutive: 1,
    }
}

#[test]
fn threshold_gt() {
    let t = threshold("cpu", ThresholdOp::Gt, 80.0);
    assert!(evaluate_threshold(&t, 81.0));
    assert!(!evaluate_threshold(&t, 80.0));
    assert!(!evaluate_threshold(&t, 79.0));
}

#[test]
fn threshold_gte() {
    let t = threshold("cpu", ThresholdOp::Gte, 80.0);
    assert!(evaluate_threshold(&t, 80.0));
    assert!(evaluate_threshold(&t, 81.0));
    assert!(!evaluate_threshold(&t, 79.9));
}

#[test]
fn threshold_lt() {
    let t = threshold("free", ThresholdOp::Lt, 10.0);
    assert!(evaluate_threshold(&t, 5.0));
    assert!(!evaluate_threshold(&t, 10.0));
    assert!(!evaluate_threshold(&t, 15.0));
}

#[test]
fn threshold_lte() {
    let t = threshold("free", ThresholdOp::Lte, 10.0);
    assert!(evaluate_threshold(&t, 10.0));
    assert!(evaluate_threshold(&t, 5.0));
    assert!(!evaluate_threshold(&t, 10.1));
}

#[test]
fn threshold_eq() {
    let t = threshold("replicas", ThresholdOp::Eq, 3.0);
    assert!(evaluate_threshold(&t, 3.0));
    assert!(!evaluate_threshold(&t, 4.0));
}

#[test]
fn threshold_op_display() {
    assert_eq!(ThresholdOp::Gt.to_string(), ">");
    assert_eq!(ThresholdOp::Gte.to_string(), ">=");
    assert_eq!(ThresholdOp::Lt.to_string(), "<");
    assert_eq!(ThresholdOp::Lte.to_string(), "<=");
    assert_eq!(ThresholdOp::Eq.to_string(), "==");
}

// ============================================================================
// FJ-3105: ThresholdTracker
// ============================================================================

#[test]
fn tracker_single_violation_fires() {
    let mut tracker = ThresholdTracker::default();
    assert!(tracker.record("cpu", true, 1));
}

#[test]
fn tracker_consecutive_violations() {
    let mut tracker = ThresholdTracker::default();
    assert!(!tracker.record("cpu", true, 3));
    assert!(!tracker.record("cpu", true, 3));
    assert!(tracker.record("cpu", true, 3));
}

#[test]
fn tracker_reset_on_ok() {
    let mut tracker = ThresholdTracker::default();
    tracker.record("cpu", true, 3);
    tracker.record("cpu", true, 3);
    tracker.record("cpu", false, 3);
    assert_eq!(tracker.count("cpu"), 0);
    assert!(!tracker.record("cpu", true, 3));
}

#[test]
fn tracker_count() {
    let mut tracker = ThresholdTracker::default();
    assert_eq!(tracker.count("cpu"), 0);
    tracker.record("cpu", true, 5);
    assert_eq!(tracker.count("cpu"), 1);
}

#[test]
fn tracker_reset_all() {
    let mut tracker = ThresholdTracker::default();
    tracker.record("cpu", true, 5);
    tracker.record("mem", true, 5);
    tracker.reset();
    assert_eq!(tracker.count("cpu"), 0);
    assert_eq!(tracker.count("mem"), 0);
}

// ============================================================================
// FJ-3105: evaluate_metrics
// ============================================================================

#[test]
fn evaluate_multiple_metrics() {
    let thresholds = vec![
        threshold("cpu", ThresholdOp::Gt, 80.0),
        threshold("mem", ThresholdOp::Gt, 90.0),
        threshold("disk", ThresholdOp::Lt, 10.0),
    ];
    let mut values = HashMap::new();
    values.insert("cpu".into(), 85.0);
    values.insert("mem".into(), 70.0);
    values.insert("disk".into(), 5.0);

    let mut tracker = ThresholdTracker::default();
    let results = evaluate_metrics(&thresholds, &values, &mut tracker);
    assert_eq!(results.len(), 3);
    assert!(results[0].violated);
    assert!(!results[1].violated);
    assert!(results[2].violated);
}

#[test]
fn evaluate_missing_metric_skipped() {
    let thresholds = vec![threshold("missing", ThresholdOp::Gt, 50.0)];
    let values = HashMap::new();
    let mut tracker = ThresholdTracker::default();
    let results = evaluate_metrics(&thresholds, &values, &mut tracker);
    assert!(results.is_empty());
}

#[test]
fn evaluate_consecutive_firing() {
    let t = MetricThreshold {
        name: "cpu".into(),
        operator: ThresholdOp::Gt,
        value: 80.0,
        consecutive: 3,
    };
    let mut values = HashMap::new();
    values.insert("cpu".into(), 90.0);
    let mut tracker = ThresholdTracker::default();
    let r1 = evaluate_metrics(std::slice::from_ref(&t), &values, &mut tracker);
    assert!(!r1[0].should_fire);
    let r2 = evaluate_metrics(std::slice::from_ref(&t), &values, &mut tracker);
    assert!(!r2[0].should_fire);
    let r3 = evaluate_metrics(&[t], &values, &mut tracker);
    assert!(r3[0].should_fire);
}

// ============================================================================
// FJ-3100: event_matches_pattern
// ============================================================================

fn make_event(event_type: EventType, payload: Vec<(&str, &str)>) -> InfraEvent {
    InfraEvent {
        event_type,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("intel".into()),
        payload: payload
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect(),
    }
}

fn make_pattern(event_type: EventType, match_fields: Vec<(&str, &str)>) -> EventPattern {
    EventPattern {
        event_type,
        match_fields: match_fields
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect(),
    }
}

#[test]
fn event_matches_type_only() {
    let event = make_event(EventType::FileChanged, vec![]);
    let pattern = make_pattern(EventType::FileChanged, vec![]);
    assert!(event_matches_pattern(&event, &pattern));
}

#[test]
fn event_type_mismatch() {
    let event = make_event(EventType::FileChanged, vec![]);
    let pattern = make_pattern(EventType::CronFired, vec![]);
    assert!(!event_matches_pattern(&event, &pattern));
}

#[test]
fn event_matches_with_payload() {
    let event = make_event(
        EventType::FileChanged,
        vec![("path", "/etc/nginx/nginx.conf")],
    );
    let pattern = make_pattern(
        EventType::FileChanged,
        vec![("path", "/etc/nginx/nginx.conf")],
    );
    assert!(event_matches_pattern(&event, &pattern));
}

#[test]
fn event_payload_mismatch() {
    let event = make_event(EventType::FileChanged, vec![("path", "/etc/hosts")]);
    let pattern = make_pattern(
        EventType::FileChanged,
        vec![("path", "/etc/nginx/nginx.conf")],
    );
    assert!(!event_matches_pattern(&event, &pattern));
}

#[test]
fn event_missing_payload_key() {
    let event = make_event(EventType::FileChanged, vec![]);
    let pattern = make_pattern(EventType::FileChanged, vec![("path", "/etc/hosts")]);
    assert!(!event_matches_pattern(&event, &pattern));
}

// ============================================================================
// FJ-3100: event_matches_rulebook
// ============================================================================

fn make_rulebook(name: &str, enabled: bool, patterns: Vec<EventPattern>) -> Rulebook {
    Rulebook {
        name: name.into(),
        description: None,
        events: patterns,
        conditions: vec![],
        actions: vec![],
        cooldown_secs: 30,
        max_retries: 3,
        enabled,
    }
}

#[test]
fn rulebook_matches_event() {
    let rb = make_rulebook(
        "repair",
        true,
        vec![make_pattern(EventType::FileChanged, vec![])],
    );
    let event = make_event(EventType::FileChanged, vec![]);
    assert!(event_matches_rulebook(&event, &rb));
}

#[test]
fn rulebook_disabled_no_match() {
    let rb = make_rulebook(
        "repair",
        false,
        vec![make_pattern(EventType::FileChanged, vec![])],
    );
    let event = make_event(EventType::FileChanged, vec![]);
    assert!(!event_matches_rulebook(&event, &rb));
}

#[test]
fn rulebook_multi_pattern_any_matches() {
    let rb = make_rulebook(
        "multi",
        true,
        vec![
            make_pattern(EventType::CronFired, vec![]),
            make_pattern(EventType::Manual, vec![]),
        ],
    );
    let event = make_event(EventType::Manual, vec![]);
    assert!(event_matches_rulebook(&event, &rb));
}

#[test]
fn rulebook_no_pattern_match() {
    let rb = make_rulebook(
        "repair",
        true,
        vec![make_pattern(EventType::CronFired, vec![])],
    );
    let event = make_event(EventType::FileChanged, vec![]);
    assert!(!event_matches_rulebook(&event, &rb));
}

// ============================================================================
// FJ-3100: RulebookAction type / CooldownTracker / EventType Display
// ============================================================================

#[test]
fn action_type_detection() {
    let apply = RulebookAction {
        apply: Some(forjar::core::types::ApplyAction {
            file: "f.yaml".into(),
            subset: vec![],
            tags: vec![],
            machine: None,
        }),
        destroy: None,
        script: None,
        notify: None,
    };
    assert_eq!(apply.action_type(), "apply");

    let script = RulebookAction {
        apply: None,
        destroy: None,
        script: Some("echo hi".into()),
        notify: None,
    };
    assert_eq!(script.action_type(), "script");

    let empty = RulebookAction {
        apply: None,
        destroy: None,
        script: None,
        notify: None,
    };
    assert_eq!(empty.action_type(), "unknown");
}

#[test]
fn cooldown_tracker_initially_can_fire() {
    let tracker = CooldownTracker::default();
    assert!(tracker.can_fire("any-rulebook", 30));
}

#[test]
fn cooldown_tracker_records_fire() {
    let mut tracker = CooldownTracker::default();
    tracker.record_fire("repair");
    assert!(!tracker.can_fire("repair", 30));
    assert!(tracker.can_fire("other", 30));
}

#[test]
fn cooldown_tracker_zero_cooldown_always_fires() {
    let mut tracker = CooldownTracker::default();
    tracker.record_fire("repair");
    assert!(tracker.can_fire("repair", 0));
}

#[test]
fn event_type_display() {
    assert_eq!(EventType::FileChanged.to_string(), "file_changed");
    assert_eq!(EventType::ProcessExit.to_string(), "process_exit");
    assert_eq!(EventType::CronFired.to_string(), "cron_fired");
    assert_eq!(EventType::Manual.to_string(), "manual");
}
