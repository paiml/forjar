//! FJ-3109: Integration test — event-driven automation pipeline.
//!
//! Tests the full workflow: event detection → rulebook matching →
//! action execution → state logging → convergence verification.

use forjar::core::rules_runtime::{evaluate_event, fired_actions, matching_rulebooks};
use forjar::core::state::rulebook_log::{append_entry, make_entry, read_entries};
use forjar::core::types::{
    event_matches_pattern, event_matches_rulebook, ApplyAction, CooldownTracker, EventPattern,
    EventType, InfraEvent, NotifyAction, Rulebook, RulebookAction, RulebookConfig,
};
use std::collections::HashMap;
use tempfile::TempDir;

fn make_event(event_type: EventType, payload: HashMap<String, String>) -> InfraEvent {
    InfraEvent {
        event_type,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web-01".into()),
        payload,
    }
}

fn file_changed_event(path: &str) -> InfraEvent {
    let mut payload = HashMap::new();
    payload.insert("path".into(), path.into());
    make_event(EventType::FileChanged, payload)
}

fn config_repair_rulebook() -> Rulebook {
    let mut match_fields = HashMap::new();
    match_fields.insert("path".into(), "/etc/nginx/nginx.conf".into());

    Rulebook {
        name: "config-repair".into(),
        description: Some("Repair nginx config when changed".into()),
        events: vec![EventPattern {
            event_type: EventType::FileChanged,
            match_fields,
        }],
        conditions: Vec::new(),
        actions: vec![RulebookAction {
            apply: Some(ApplyAction {
                file: "forjar.yaml".into(),
                subset: vec!["nginx-conf".into()],
                tags: vec!["config".into()],
                machine: Some("web-01".into()),
            }),
            destroy: None,
            script: None,
            notify: None,
        }],
        cooldown_secs: 30,
        max_retries: 3,
        enabled: true,
    }
}

fn alert_rulebook() -> Rulebook {
    Rulebook {
        name: "alert-on-metric".into(),
        description: Some("Alert when metric threshold exceeded".into()),
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
                channel: "https://hooks.slack.com/services/xxx".into(),
                message: "CPU threshold exceeded on {{machine}}".into(),
            }),
        }],
        cooldown_secs: 300,
        max_retries: 1,
        enabled: true,
    }
}

/// Test: file change event → matches config-repair rulebook → produces apply action.
#[test]
fn file_change_triggers_config_repair() {
    let event = file_changed_event("/etc/nginx/nginx.conf");
    let rb = config_repair_rulebook();

    // Event matches the rulebook
    assert!(event_matches_rulebook(&event, &rb));

    // Runtime evaluation produces an action
    let config = RulebookConfig {
        rulebooks: vec![rb],
    };
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event, &config, &mut tracker);

    assert_eq!(results.len(), 1);
    assert!(!results[0].cooldown_blocked);
    assert_eq!(results[0].actions.len(), 1);
    assert_eq!(results[0].actions[0].action_type(), "apply");
}

/// Test: file change for wrong path does NOT trigger.
#[test]
fn wrong_path_no_trigger() {
    let event = file_changed_event("/etc/hosts");
    let rb = config_repair_rulebook();

    assert!(!event_matches_rulebook(&event, &rb));
}

/// Test: cooldown prevents rapid re-fire.
#[test]
fn cooldown_prevents_rapid_refire() {
    let event = file_changed_event("/etc/nginx/nginx.conf");
    let config = RulebookConfig {
        rulebooks: vec![config_repair_rulebook()],
    };
    let mut tracker = CooldownTracker::default();

    // First fire succeeds
    let r1 = fired_actions(&event, &config, &mut tracker);
    assert_eq!(r1.len(), 1);

    // Immediate re-fire blocked by 30s cooldown
    let r2 = fired_actions(&event, &config, &mut tracker);
    assert!(r2.is_empty());
}

/// Test: full pipeline — event → evaluate → log → read back.
#[test]
fn full_pipeline_event_to_log() {
    let dir = TempDir::new().unwrap();
    let event = file_changed_event("/etc/nginx/nginx.conf");
    let config = RulebookConfig {
        rulebooks: vec![config_repair_rulebook()],
    };
    let mut tracker = CooldownTracker::default();

    // Evaluate
    let actions = fired_actions(&event, &config, &mut tracker);
    assert_eq!(actions.len(), 1);
    let (rb_name, action_list) = &actions[0];

    // Log the action
    let entry = make_entry(&event, rb_name, action_list[0].action_type(), true, None);
    append_entry(dir.path(), &entry).unwrap();

    // Read back
    let entries = read_entries(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].rulebook, "config-repair");
    assert_eq!(entries[0].action_type, "apply");
    assert!(entries[0].success);
    assert_eq!(entries[0].event_type, EventType::FileChanged);
}

/// Test: multiple rulebooks evaluated for same event type.
#[test]
fn multiple_rulebooks_single_event() {
    let event = file_changed_event("/etc/nginx/nginx.conf");

    // Second rulebook also matches file_changed but with no path filter
    let mut backup_rb = config_repair_rulebook();
    backup_rb.name = "backup-trigger".into();
    backup_rb.events[0].match_fields.clear(); // match all file changes

    let config = RulebookConfig {
        rulebooks: vec![config_repair_rulebook(), backup_rb],
    };
    let mut tracker = CooldownTracker::default();

    let results = evaluate_event(&event, &config, &mut tracker);
    assert_eq!(results.len(), 2);
}

/// Test: matching_rulebooks ignores cooldown state.
#[test]
fn matching_ignores_cooldown() {
    let event = file_changed_event("/etc/nginx/nginx.conf");
    let config = RulebookConfig {
        rulebooks: vec![config_repair_rulebook()],
    };

    let matched = matching_rulebooks(&event, &config);
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].name, "config-repair");
}

/// Test: disabled rulebook does not fire.
#[test]
fn disabled_rulebook_no_fire() {
    let mut rb = config_repair_rulebook();
    rb.enabled = false;

    let event = file_changed_event("/etc/nginx/nginx.conf");
    let config = RulebookConfig {
        rulebooks: vec![rb],
    };
    let mut tracker = CooldownTracker::default();

    let actions = fired_actions(&event, &config, &mut tracker);
    assert!(actions.is_empty());
}

/// Test: metric threshold → alert rulebook → notify action.
#[test]
fn metric_threshold_triggers_alert() {
    let event = make_event(EventType::MetricThreshold, HashMap::new());
    let config = RulebookConfig {
        rulebooks: vec![alert_rulebook()],
    };
    let mut tracker = CooldownTracker::default();

    let results = evaluate_event(&event, &config, &mut tracker);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].actions[0].action_type(), "notify");
}

/// Test: manual trigger event.
#[test]
fn manual_trigger_event() {
    let event = make_event(EventType::Manual, HashMap::new());
    let rb = Rulebook {
        name: "manual-deploy".into(),
        description: None,
        events: vec![EventPattern {
            event_type: EventType::Manual,
            match_fields: HashMap::new(),
        }],
        conditions: Vec::new(),
        actions: vec![RulebookAction {
            apply: Some(ApplyAction {
                file: "forjar.yaml".into(),
                subset: Vec::new(),
                tags: Vec::new(),
                machine: None,
            }),
            destroy: None,
            script: None,
            notify: None,
        }],
        cooldown_secs: 0,
        max_retries: 0,
        enabled: true,
    };

    let config = RulebookConfig {
        rulebooks: vec![rb],
    };
    let mut tracker = CooldownTracker::default();

    let actions = fired_actions(&event, &config, &mut tracker);
    assert_eq!(actions.len(), 1);
}

/// Test: log multiple events and verify ordering.
#[test]
fn log_ordering_preserved() {
    let dir = TempDir::new().unwrap();
    let event = file_changed_event("/etc/nginx/nginx.conf");

    for i in 0..5 {
        let entry = make_entry(&event, &format!("rulebook-{i}"), "apply", true, None);
        append_entry(dir.path(), &entry).unwrap();
    }

    let entries = read_entries(dir.path()).unwrap();
    assert_eq!(entries.len(), 5);
    for (i, entry) in entries.iter().enumerate() {
        assert_eq!(entry.rulebook, format!("rulebook-{i}"));
    }
}

/// Test: failed action gets logged with error.
#[test]
fn failed_action_logged() {
    let dir = TempDir::new().unwrap();
    let event = file_changed_event("/etc/nginx/nginx.conf");

    let entry = make_entry(
        &event,
        "config-repair",
        "apply",
        false,
        Some("resource nginx-conf failed: permission denied".into()),
    );
    append_entry(dir.path(), &entry).unwrap();

    let entries = read_entries(dir.path()).unwrap();
    assert!(!entries[0].success);
    assert!(entries[0]
        .error
        .as_ref()
        .unwrap()
        .contains("permission denied"));
}

/// Test: event pattern matching with multiple fields.
#[test]
fn multi_field_pattern_match() {
    let mut match_fields = HashMap::new();
    match_fields.insert("path".into(), "/etc/nginx/nginx.conf".into());
    match_fields.insert("action".into(), "modify".into());

    let pattern = EventPattern {
        event_type: EventType::FileChanged,
        match_fields,
    };

    // Event with both fields
    let mut payload = HashMap::new();
    payload.insert("path".into(), "/etc/nginx/nginx.conf".into());
    payload.insert("action".into(), "modify".into());
    let event = make_event(EventType::FileChanged, payload);

    assert!(event_matches_pattern(&event, &pattern));

    // Event missing one field
    let mut partial_payload = HashMap::new();
    partial_payload.insert("path".into(), "/etc/nginx/nginx.conf".into());
    let partial_event = make_event(EventType::FileChanged, partial_payload);

    assert!(!event_matches_pattern(&partial_event, &pattern));
}

/// Test: convergence — re-fire after cooldown expires.
#[test]
fn convergence_after_cooldown() {
    let rb = Rulebook {
        name: "fast-converge".into(),
        description: None,
        events: vec![EventPattern {
            event_type: EventType::FileChanged,
            match_fields: HashMap::new(),
        }],
        conditions: Vec::new(),
        actions: vec![RulebookAction {
            apply: Some(ApplyAction {
                file: "forjar.yaml".into(),
                subset: Vec::new(),
                tags: Vec::new(),
                machine: None,
            }),
            destroy: None,
            script: None,
            notify: None,
        }],
        cooldown_secs: 0, // zero cooldown = always re-fire
        max_retries: 3,
        enabled: true,
    };

    let config = RulebookConfig {
        rulebooks: vec![rb],
    };
    let event = make_event(EventType::FileChanged, HashMap::new());
    let mut tracker = CooldownTracker::default();

    // Should fire every time with zero cooldown
    for _ in 0..5 {
        let actions = fired_actions(&event, &config, &mut tracker);
        assert_eq!(actions.len(), 1);
    }
}
