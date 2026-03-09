//! FJ-3100/3106: Event-driven rules runtime falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-3100: Event types and pattern matching
//! - FJ-3106: Rulebook runtime evaluation with cooldown + deduplication
//! - Action type dispatch (apply, destroy, script, notify)
//! - Runtime summary aggregation
//!
//! Usage: cargo test --test falsification_rules_runtime

use forjar::core::rules_runtime::{
    evaluate_event, fired_actions, matching_rulebooks, runtime_summary,
};
use forjar::core::types::{
    ApplyAction, CooldownTracker, DestroyAction, EventPattern, EventType, InfraEvent, NotifyAction,
    Rulebook, RulebookAction, RulebookConfig,
};
use std::collections::HashMap;

fn event(event_type: EventType) -> InfraEvent {
    InfraEvent {
        event_type,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web-1".into()),
        payload: HashMap::new(),
    }
}

fn event_with_payload(event_type: EventType, key: &str, val: &str) -> InfraEvent {
    let mut payload = HashMap::new();
    payload.insert(key.to_string(), val.to_string());
    InfraEvent {
        event_type,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web-1".into()),
        payload,
    }
}

fn rulebook(name: &str, event_type: EventType, cooldown: u64) -> Rulebook {
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

// ============================================================================
// FJ-3100: Event Type Matching
// ============================================================================

#[test]
fn event_type_matching_file_changed() {
    let config = RulebookConfig {
        rulebooks: vec![rulebook("file-watch", EventType::FileChanged, 0)],
    };
    let matched = matching_rulebooks(&event(EventType::FileChanged), &config);
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].name, "file-watch");
}

#[test]
fn event_type_no_match_different_type() {
    let config = RulebookConfig {
        rulebooks: vec![rulebook("file-watch", EventType::FileChanged, 0)],
    };
    let matched = matching_rulebooks(&event(EventType::CronFired), &config);
    assert!(matched.is_empty());
}

#[test]
fn event_type_all_variants_matchable() {
    let types = [
        EventType::FileChanged,
        EventType::ProcessExit,
        EventType::CronFired,
        EventType::WebhookReceived,
        EventType::MetricThreshold,
        EventType::Manual,
    ];
    for et in types {
        let config = RulebookConfig {
            rulebooks: vec![rulebook("test", et.clone(), 0)],
        };
        let matched = matching_rulebooks(&event(et), &config);
        assert_eq!(matched.len(), 1, "failed for event type: {:?}", matched);
    }
}

#[test]
fn event_payload_match() {
    let mut rb = rulebook("payload-match", EventType::FileChanged, 0);
    rb.events[0]
        .match_fields
        .insert("path".into(), "/etc/nginx.conf".into());

    let config = RulebookConfig {
        rulebooks: vec![rb],
    };
    let evt = event_with_payload(EventType::FileChanged, "path", "/etc/nginx.conf");
    let matched = matching_rulebooks(&evt, &config);
    assert_eq!(matched.len(), 1);
}

#[test]
fn event_payload_mismatch() {
    let mut rb = rulebook("payload-match", EventType::FileChanged, 0);
    rb.events[0]
        .match_fields
        .insert("path".into(), "/etc/nginx.conf".into());

    let config = RulebookConfig {
        rulebooks: vec![rb],
    };
    let evt = event_with_payload(EventType::FileChanged, "path", "/etc/other.conf");
    let matched = matching_rulebooks(&evt, &config);
    assert!(matched.is_empty());
}

// ============================================================================
// FJ-3106: Evaluation with Cooldown
// ============================================================================

#[test]
fn evaluate_fires_on_match() {
    let config = RulebookConfig {
        rulebooks: vec![rulebook("rb1", EventType::FileChanged, 30)],
    };
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::FileChanged), &config, &mut tracker);
    assert_eq!(results.len(), 1);
    assert!(!results[0].cooldown_blocked);
    assert!(!results[0].disabled);
    assert_eq!(results[0].actions.len(), 1);
}

#[test]
fn evaluate_cooldown_blocks_second_fire() {
    let config = RulebookConfig {
        rulebooks: vec![rulebook("rb1", EventType::FileChanged, 60)],
    };
    let mut tracker = CooldownTracker::default();

    let r1 = evaluate_event(&event(EventType::FileChanged), &config, &mut tracker);
    assert!(!r1[0].cooldown_blocked);

    let r2 = evaluate_event(&event(EventType::FileChanged), &config, &mut tracker);
    assert!(r2[0].cooldown_blocked);
    assert!(r2[0].actions.is_empty());
}

#[test]
fn evaluate_zero_cooldown_always_fires() {
    let config = RulebookConfig {
        rulebooks: vec![rulebook("rapid", EventType::Manual, 0)],
    };
    let mut tracker = CooldownTracker::default();

    let r1 = evaluate_event(&event(EventType::Manual), &config, &mut tracker);
    assert!(!r1[0].cooldown_blocked);

    let r2 = evaluate_event(&event(EventType::Manual), &config, &mut tracker);
    assert!(!r2[0].cooldown_blocked);
}

#[test]
fn evaluate_disabled_rulebook_skipped() {
    let mut rb = rulebook("disabled", EventType::FileChanged, 0);
    rb.enabled = false;

    let config = RulebookConfig {
        rulebooks: vec![rb],
    };
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::FileChanged), &config, &mut tracker);
    assert_eq!(results.len(), 1);
    assert!(results[0].disabled);
    assert!(results[0].actions.is_empty());
}

#[test]
fn evaluate_multiple_rulebooks_match() {
    let config = RulebookConfig {
        rulebooks: vec![
            rulebook("rb1", EventType::FileChanged, 0),
            rulebook("rb2", EventType::FileChanged, 0),
        ],
    };
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::FileChanged), &config, &mut tracker);
    assert_eq!(results.len(), 2);
}

#[test]
fn evaluate_no_match_returns_empty() {
    let config = RulebookConfig {
        rulebooks: vec![rulebook("rb1", EventType::FileChanged, 0)],
    };
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::CronFired), &config, &mut tracker);
    assert!(results.is_empty());
}

// ============================================================================
// FJ-3106: fired_actions
// ============================================================================

#[test]
fn fired_actions_filters_blocked() {
    let config = RulebookConfig {
        rulebooks: vec![rulebook("rb1", EventType::FileChanged, 60)],
    };
    let mut tracker = CooldownTracker::default();

    let actions = fired_actions(&event(EventType::FileChanged), &config, &mut tracker);
    assert_eq!(actions.len(), 1);

    // Second fire — blocked by cooldown
    let actions = fired_actions(&event(EventType::FileChanged), &config, &mut tracker);
    assert!(actions.is_empty());
}

#[test]
fn fired_actions_excludes_disabled() {
    let mut rb = rulebook("disabled", EventType::Manual, 0);
    rb.enabled = false;
    let config = RulebookConfig {
        rulebooks: vec![rb],
    };
    let mut tracker = CooldownTracker::default();
    let actions = fired_actions(&event(EventType::Manual), &config, &mut tracker);
    assert!(actions.is_empty());
}

// ============================================================================
// FJ-3100: Action Types
// ============================================================================

#[test]
fn action_type_apply() {
    let action = RulebookAction {
        apply: Some(ApplyAction {
            file: "forjar.yaml".into(),
            subset: vec!["nginx".into()],
            tags: Vec::new(),
            machine: None,
        }),
        destroy: None,
        script: None,
        notify: None,
    };
    assert_eq!(action.action_type(), "apply");
}

#[test]
fn action_type_destroy() {
    let action = RulebookAction {
        apply: None,
        destroy: Some(DestroyAction {
            file: "forjar.yaml".into(),
            resources: vec!["temp".into()],
        }),
        script: None,
        notify: None,
    };
    assert_eq!(action.action_type(), "destroy");
}

#[test]
fn action_type_script() {
    let action = RulebookAction {
        apply: None,
        destroy: None,
        script: Some("echo hello".into()),
        notify: None,
    };
    assert_eq!(action.action_type(), "script");
}

#[test]
fn action_type_notify() {
    let action = RulebookAction {
        apply: None,
        destroy: None,
        script: None,
        notify: Some(NotifyAction {
            channel: "https://hooks.slack.com/xxx".into(),
            message: "Alert!".into(),
        }),
    };
    assert_eq!(action.action_type(), "notify");
}

#[test]
fn action_type_unknown() {
    let action = RulebookAction {
        apply: None,
        destroy: None,
        script: None,
        notify: None,
    };
    assert_eq!(action.action_type(), "unknown");
}

// ============================================================================
// FJ-3106: Runtime Summary
// ============================================================================

#[test]
fn runtime_summary_counts() {
    let mut disabled = rulebook("disabled", EventType::Manual, 0);
    disabled.enabled = false;

    let config = RulebookConfig {
        rulebooks: vec![
            rulebook("active1", EventType::FileChanged, 0),
            rulebook("active2", EventType::CronFired, 0),
            disabled,
        ],
    };
    let tracker = CooldownTracker::default();
    let summary = runtime_summary(&config, &tracker);
    assert_eq!(summary.total_rulebooks, 3);
    assert_eq!(summary.enabled, 2);
    assert_eq!(summary.disabled, 1);
    assert_eq!(summary.in_cooldown, 0);
}

#[test]
fn runtime_summary_cooldown_count() {
    let config = RulebookConfig {
        rulebooks: vec![rulebook("rb1", EventType::FileChanged, 60)],
    };
    let mut tracker = CooldownTracker::default();

    // Fire once to trigger cooldown
    evaluate_event(&event(EventType::FileChanged), &config, &mut tracker);

    let summary = runtime_summary(&config, &tracker);
    assert_eq!(summary.in_cooldown, 1);
}

#[test]
fn runtime_summary_empty_config() {
    let config = RulebookConfig { rulebooks: vec![] };
    let tracker = CooldownTracker::default();
    let summary = runtime_summary(&config, &tracker);
    assert_eq!(summary.total_rulebooks, 0);
    assert_eq!(summary.enabled, 0);
}

// ============================================================================
// Cross-cutting: EvalResult properties
// ============================================================================

#[test]
fn eval_result_fired_has_actions() {
    let config = RulebookConfig {
        rulebooks: vec![rulebook("rb", EventType::Manual, 0)],
    };
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::Manual), &config, &mut tracker);
    let result = &results[0];
    assert!(!result.cooldown_blocked);
    assert!(!result.disabled);
    assert!(!result.actions.is_empty());
    assert_eq!(result.rulebook, "rb");
}

#[test]
fn eval_result_blocked_has_no_actions() {
    let config = RulebookConfig {
        rulebooks: vec![rulebook("rb", EventType::Manual, 600)],
    };
    let mut tracker = CooldownTracker::default();

    // First fire
    evaluate_event(&event(EventType::Manual), &config, &mut tracker);
    // Second fire — blocked
    let results = evaluate_event(&event(EventType::Manual), &config, &mut tracker);
    assert!(results[0].cooldown_blocked);
    assert!(results[0].actions.is_empty());
}

#[test]
fn notify_action_in_evaluation() {
    let rb = Rulebook {
        name: "alert".into(),
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
                message: "CPU exceeded".into(),
            }),
        }],
        cooldown_secs: 300,
        max_retries: 1,
        enabled: true,
    };

    let config = RulebookConfig {
        rulebooks: vec![rb],
    };
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::MetricThreshold), &config, &mut tracker);
    assert_eq!(results.len(), 1);
    assert!(results[0].actions[0].notify.is_some());
}
