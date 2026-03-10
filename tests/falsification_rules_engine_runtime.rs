//! FJ-3108/3106: Rulebook validation engine and runtime evaluator
//! falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-3108: Rulebook validation
//!   - validate_rulebook_yaml: parse + semantic validation
//!   - validate_rulebook_config: duplicate names, missing events/actions
//!   - event_type_coverage: event type counting
//! - FJ-3106: Rulebook runtime
//!   - evaluate_event: matching, cooldown, disabled
//!   - fired_actions: filtered action extraction
//!   - matching_rulebooks: pattern matching ignoring cooldown
//!   - runtime_summary: state aggregation
//!
//! Usage: cargo test --test falsification_rules_engine_runtime

use forjar::core::rules_engine::{
    event_type_coverage, validate_rulebook_yaml,
};
use forjar::core::rules_runtime::{
    evaluate_event, fired_actions, matching_rulebooks, runtime_summary,
};
use forjar::core::types::{
    ApplyAction, CooldownTracker, DestroyAction, EventPattern, EventType, InfraEvent, NotifyAction,
    Rulebook, RulebookAction, RulebookConfig,
};
use std::collections::HashMap;

// ── Helpers ──

fn event(et: EventType) -> InfraEvent {
    InfraEvent {
        event_type: et,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web-1".into()),
        payload: HashMap::new(),
    }
}

fn event_with(et: EventType, key: &str, val: &str) -> InfraEvent {
    let mut payload = HashMap::new();
    payload.insert(key.into(), val.into());
    InfraEvent {
        event_type: et,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web-1".into()),
        payload,
    }
}

fn rulebook(name: &str, et: EventType, cooldown: u64) -> Rulebook {
    Rulebook {
        name: name.into(),
        description: None,
        events: vec![EventPattern {
            event_type: et,
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

fn config(rbs: Vec<Rulebook>) -> RulebookConfig {
    RulebookConfig { rulebooks: rbs }
}

// ============================================================================
// FJ-3108: validate_rulebook_yaml — valid
// ============================================================================

#[test]
fn validate_valid_rulebook_yaml() {
    let yaml = r#"
rulebooks:
  - name: config-repair
    events:
      - type: file_changed
        match:
          path: /etc/nginx/nginx.conf
    actions:
      - apply:
          file: forjar.yaml
          tags: [config]
    cooldown_secs: 60
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(
        issues.is_empty(),
        "valid rulebook has no issues: {issues:?}"
    );
}

#[test]
fn validate_yaml_parse_error() {
    assert!(validate_rulebook_yaml("not: valid: [yaml").is_err());
}

// ============================================================================
// FJ-3108: validate_rulebook_config — semantic checks
// ============================================================================

#[test]
fn validate_no_events_error() {
    let yaml = r#"
rulebooks:
  - name: bad
    events: []
    actions:
      - script: "echo ok"
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("no event")));
}

#[test]
fn validate_no_actions_error() {
    let yaml = r#"
rulebooks:
  - name: bad
    events:
      - type: manual
    actions: []
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("no actions")));
}

#[test]
fn validate_duplicate_names_error() {
    let yaml = r#"
rulebooks:
  - name: dupe
    events: [{type: manual}]
    actions: [{script: "echo 1"}]
  - name: dupe
    events: [{type: manual}]
    actions: [{script: "echo 2"}]
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("duplicate")));
}

#[test]
fn validate_empty_apply_file() {
    let yaml = r#"
rulebooks:
  - name: bad-apply
    events: [{type: manual}]
    actions:
      - apply:
          file: ""
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues
        .iter()
        .any(|i| i.message.contains("apply.file is empty")));
}

#[test]
fn validate_zero_cooldown_warning() {
    let yaml = r#"
rulebooks:
  - name: rapid
    events: [{type: manual}]
    actions: [{script: "echo ok"}]
    cooldown_secs: 0
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("cooldown_secs=0")));
}

#[test]
fn validate_high_retries_warning() {
    let yaml = r#"
rulebooks:
  - name: retry
    events: [{type: manual}]
    actions: [{script: "echo ok"}]
    max_retries: 50
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("unusually high")));
}

#[test]
fn validate_empty_notify_channel() {
    let yaml = r#"
rulebooks:
  - name: bad-notify
    events: [{type: manual}]
    actions:
      - notify:
          channel: ""
          message: "test"
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues
        .iter()
        .any(|i| i.message.contains("notify.channel is empty")));
}

// ============================================================================
// FJ-3108: event_type_coverage
// ============================================================================

#[test]
fn event_coverage_counts() {
    let yaml = r#"
rulebooks:
  - name: r1
    events:
      - {type: file_changed}
      - {type: manual}
    actions: [{script: "echo 1"}]
  - name: r2
    events:
      - {type: file_changed}
    actions: [{script: "echo 2"}]
"#;
    let cfg: RulebookConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let coverage = event_type_coverage(&cfg);
    let fc = coverage
        .iter()
        .find(|(et, _)| *et == EventType::FileChanged)
        .unwrap();
    assert_eq!(fc.1, 2);
    let manual = coverage
        .iter()
        .find(|(et, _)| *et == EventType::Manual)
        .unwrap();
    assert_eq!(manual.1, 1);
    let cron = coverage
        .iter()
        .find(|(et, _)| *et == EventType::CronFired)
        .unwrap();
    assert_eq!(cron.1, 0);
}

#[test]
fn event_coverage_all_six_types_present() {
    let cfg = RulebookConfig { rulebooks: vec![] };
    let coverage = event_type_coverage(&cfg);
    assert_eq!(coverage.len(), 6);
}

// ============================================================================
// FJ-3106: evaluate_event — matching
// ============================================================================

#[test]
fn runtime_matching_event_fires() {
    let cfg = config(vec![rulebook("rb", EventType::FileChanged, 30)]);
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::FileChanged), &cfg, &mut tracker);
    assert_eq!(results.len(), 1);
    assert!(!results[0].cooldown_blocked);
    assert!(!results[0].disabled);
    assert_eq!(results[0].actions.len(), 1);
}

#[test]
fn runtime_no_match_empty() {
    let cfg = config(vec![rulebook("rb", EventType::FileChanged, 30)]);
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::CronFired), &cfg, &mut tracker);
    assert!(results.is_empty());
}

#[test]
fn runtime_cooldown_blocks_second_fire() {
    let cfg = config(vec![rulebook("rb", EventType::FileChanged, 60)]);
    let mut tracker = CooldownTracker::default();
    let e = event(EventType::FileChanged);
    let r1 = evaluate_event(&e, &cfg, &mut tracker);
    assert!(!r1[0].cooldown_blocked);
    let r2 = evaluate_event(&e, &cfg, &mut tracker);
    assert!(r2[0].cooldown_blocked);
}

#[test]
fn runtime_zero_cooldown_always_fires() {
    let cfg = config(vec![rulebook("rapid", EventType::Manual, 0)]);
    let mut tracker = CooldownTracker::default();
    let e = event(EventType::Manual);
    let r1 = evaluate_event(&e, &cfg, &mut tracker);
    assert!(!r1[0].cooldown_blocked);
    let r2 = evaluate_event(&e, &cfg, &mut tracker);
    assert!(!r2[0].cooldown_blocked);
}

#[test]
fn runtime_disabled_rulebook_skipped() {
    let mut rb = rulebook("disabled", EventType::FileChanged, 0);
    rb.enabled = false;
    let cfg = config(vec![rb]);
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::FileChanged), &cfg, &mut tracker);
    assert_eq!(results.len(), 1);
    assert!(results[0].disabled);
    assert!(results[0].actions.is_empty());
}

#[test]
fn runtime_multiple_match() {
    let cfg = config(vec![
        rulebook("rb1", EventType::FileChanged, 0),
        rulebook("rb2", EventType::FileChanged, 0),
    ]);
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::FileChanged), &cfg, &mut tracker);
    assert_eq!(results.len(), 2);
}

#[test]
fn runtime_payload_match() {
    let mut rb = rulebook("payload", EventType::FileChanged, 0);
    rb.events[0]
        .match_fields
        .insert("path".into(), "/etc/nginx".into());
    let cfg = config(vec![rb]);
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(
        &event_with(EventType::FileChanged, "path", "/etc/nginx"),
        &cfg,
        &mut tracker,
    );
    assert_eq!(results.len(), 1);
}

#[test]
fn runtime_payload_mismatch() {
    let mut rb = rulebook("payload", EventType::FileChanged, 0);
    rb.events[0]
        .match_fields
        .insert("path".into(), "/etc/nginx".into());
    let cfg = config(vec![rb]);
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(
        &event_with(EventType::FileChanged, "path", "/etc/other"),
        &cfg,
        &mut tracker,
    );
    assert!(results.is_empty());
}

// ============================================================================
// FJ-3106: fired_actions
// ============================================================================

#[test]
fn fired_actions_filters_blocked() {
    let cfg = config(vec![rulebook("rb", EventType::FileChanged, 60)]);
    let mut tracker = CooldownTracker::default();
    let e = event(EventType::FileChanged);
    let a1 = fired_actions(&e, &cfg, &mut tracker);
    assert_eq!(a1.len(), 1);
    let a2 = fired_actions(&e, &cfg, &mut tracker);
    assert!(a2.is_empty());
}

// ============================================================================
// FJ-3106: matching_rulebooks
// ============================================================================

#[test]
fn matching_ignores_cooldown() {
    let cfg = config(vec![rulebook("rb", EventType::Manual, 0)]);
    let matched = matching_rulebooks(&event(EventType::Manual), &cfg);
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].name, "rb");
}

// ============================================================================
// FJ-3106: runtime_summary
// ============================================================================

#[test]
fn summary_counts_enabled_disabled() {
    let mut rb_off = rulebook("off", EventType::Manual, 0);
    rb_off.enabled = false;
    let cfg = config(vec![rulebook("on", EventType::FileChanged, 0), rb_off]);
    let tracker = CooldownTracker::default();
    let s = runtime_summary(&cfg, &tracker);
    assert_eq!(s.total_rulebooks, 2);
    assert_eq!(s.enabled, 1);
    assert_eq!(s.disabled, 1);
    assert_eq!(s.in_cooldown, 0);
}

// ============================================================================
// FJ-3106: action types
// ============================================================================

#[test]
fn action_type_notify() {
    let rb = Rulebook {
        name: "notify-test".into(),
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
                message: "alert".into(),
            }),
        }],
        cooldown_secs: 300,
        max_retries: 1,
        enabled: true,
    };
    let cfg = config(vec![rb]);
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::MetricThreshold), &cfg, &mut tracker);
    assert_eq!(results[0].actions[0].action_type(), "notify");
}

#[test]
fn action_type_destroy() {
    let rb = Rulebook {
        name: "cleanup".into(),
        description: None,
        events: vec![EventPattern {
            event_type: EventType::CronFired,
            match_fields: HashMap::new(),
        }],
        conditions: Vec::new(),
        actions: vec![RulebookAction {
            apply: None,
            destroy: Some(DestroyAction {
                file: "forjar.yaml".into(),
                resources: vec!["temp".into()],
            }),
            script: None,
            notify: None,
        }],
        cooldown_secs: 3600,
        max_retries: 0,
        enabled: true,
    };
    let cfg = config(vec![rb]);
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event(EventType::CronFired), &cfg, &mut tracker);
    assert_eq!(results[0].actions[0].action_type(), "destroy");
}
