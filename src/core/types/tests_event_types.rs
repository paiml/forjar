//! Tests for FJ-3100 event-driven automation types.

use super::*;
use std::collections::HashMap;

fn make_event(event_type: EventType, payload: &[(&str, &str)]) -> InfraEvent {
    InfraEvent {
        event_type,
        timestamp: "2026-03-09T00:00:00Z".into(),
        machine: Some("web-01".into()),
        payload: payload
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}

#[test]
fn event_type_display() {
    assert_eq!(EventType::FileChanged.to_string(), "file_changed");
    assert_eq!(EventType::ProcessExit.to_string(), "process_exit");
    assert_eq!(EventType::CronFired.to_string(), "cron_fired");
    assert_eq!(EventType::Manual.to_string(), "manual");
}

#[test]
fn event_type_serde_roundtrip() {
    for et in [
        EventType::FileChanged,
        EventType::ProcessExit,
        EventType::CronFired,
        EventType::WebhookReceived,
        EventType::MetricThreshold,
        EventType::Manual,
    ] {
        let json = serde_json::to_string(&et).unwrap();
        let parsed: EventType = serde_json::from_str(&json).unwrap();
        assert_eq!(et, parsed);
    }
}

#[test]
fn pattern_match_type_only() {
    let event = make_event(EventType::FileChanged, &[]);
    let pattern = EventPattern {
        event_type: EventType::FileChanged,
        match_fields: HashMap::new(),
    };
    assert!(event_matches_pattern(&event, &pattern));
}

#[test]
fn pattern_match_with_fields() {
    let event = make_event(EventType::FileChanged, &[("path", "/etc/nginx/nginx.conf")]);
    let mut fields = HashMap::new();
    fields.insert("path".into(), "/etc/nginx/nginx.conf".into());
    let pattern = EventPattern {
        event_type: EventType::FileChanged,
        match_fields: fields,
    };
    assert!(event_matches_pattern(&event, &pattern));
}

#[test]
fn pattern_no_match_wrong_type() {
    let event = make_event(EventType::CronFired, &[]);
    let pattern = EventPattern {
        event_type: EventType::FileChanged,
        match_fields: HashMap::new(),
    };
    assert!(!event_matches_pattern(&event, &pattern));
}

#[test]
fn pattern_no_match_missing_field() {
    let event = make_event(EventType::ProcessExit, &[]);
    let mut fields = HashMap::new();
    fields.insert("exit_code".into(), "137".into());
    let pattern = EventPattern {
        event_type: EventType::ProcessExit,
        match_fields: fields,
    };
    assert!(!event_matches_pattern(&event, &pattern));
}

#[test]
fn pattern_no_match_wrong_value() {
    let event = make_event(EventType::ProcessExit, &[("exit_code", "0")]);
    let mut fields = HashMap::new();
    fields.insert("exit_code".into(), "137".into());
    let pattern = EventPattern {
        event_type: EventType::ProcessExit,
        match_fields: fields,
    };
    assert!(!event_matches_pattern(&event, &pattern));
}

#[test]
fn rulebook_serde() {
    let yaml = r#"
name: config-repair
description: "Repair config drift"
events:
  - type: file_changed
    match:
      path: /etc/nginx/nginx.conf
actions:
  - apply:
      file: forjar.yaml
      tags: [config]
cooldown_secs: 30
max_retries: 5
"#;
    let rb: Rulebook = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(rb.name, "config-repair");
    assert_eq!(rb.events.len(), 1);
    assert_eq!(rb.actions.len(), 1);
    assert_eq!(rb.actions[0].action_type(), "apply");
    assert_eq!(rb.cooldown_secs, 30);
    assert_eq!(rb.max_retries, 5);
    assert!(rb.enabled); // default true
}

#[test]
fn rulebook_defaults() {
    let yaml = r#"
name: minimal
events:
  - type: manual
actions:
  - script: "echo ok"
"#;
    let rb: Rulebook = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(rb.cooldown_secs, 30); // default
    assert_eq!(rb.max_retries, 3); // default
    assert!(rb.enabled);
}

#[test]
fn event_matches_rulebook_enabled() {
    let event = make_event(EventType::FileChanged, &[]);
    let rb = Rulebook {
        name: "test".into(),
        description: None,
        events: vec![EventPattern {
            event_type: EventType::FileChanged,
            match_fields: HashMap::new(),
        }],
        conditions: vec![],
        actions: vec![],
        cooldown_secs: 30,
        max_retries: 3,
        enabled: true,
    };
    assert!(event_matches_rulebook(&event, &rb));
}

#[test]
fn event_matches_rulebook_disabled() {
    let event = make_event(EventType::FileChanged, &[]);
    let rb = Rulebook {
        name: "test".into(),
        description: None,
        events: vec![EventPattern {
            event_type: EventType::FileChanged,
            match_fields: HashMap::new(),
        }],
        conditions: vec![],
        actions: vec![],
        cooldown_secs: 30,
        max_retries: 3,
        enabled: false,
    };
    assert!(!event_matches_rulebook(&event, &rb));
}

#[test]
fn cooldown_tracker_initial() {
    let tracker = CooldownTracker::default();
    assert!(tracker.can_fire("test", 30));
}

#[test]
fn cooldown_tracker_blocks() {
    let mut tracker = CooldownTracker::default();
    tracker.record_fire("test");
    assert!(!tracker.can_fire("test", 30));
}

#[test]
fn cooldown_tracker_zero_cooldown() {
    let mut tracker = CooldownTracker::default();
    tracker.record_fire("test");
    assert!(tracker.can_fire("test", 0));
}

#[test]
fn cooldown_tracker_independent_rulebooks() {
    let mut tracker = CooldownTracker::default();
    tracker.record_fire("rule-a");
    assert!(!tracker.can_fire("rule-a", 60));
    assert!(tracker.can_fire("rule-b", 60));
}

#[test]
fn action_types() {
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
    assert_eq!(apply.action_type(), "apply");

    let script = RulebookAction {
        apply: None,
        destroy: None,
        script: Some("echo ok".into()),
        notify: None,
    };
    assert_eq!(script.action_type(), "script");
}

#[test]
fn rulebook_config_parse() {
    let yaml = r#"
rulebooks:
  - name: rule-1
    events:
      - type: cron_fired
    actions:
      - script: "forjar apply -f infra.yaml"
  - name: rule-2
    events:
      - type: manual
    actions:
      - notify:
          channel: "https://hooks.slack.com/abc"
          message: "Manual trigger fired"
"#;
    let config: RulebookConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(config.rulebooks.len(), 2);
    assert_eq!(config.rulebooks[0].name, "rule-1");
    assert_eq!(config.rulebooks[1].actions[0].action_type(), "notify");
}
