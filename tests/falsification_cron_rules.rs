//! FJ-3103/3106: Cron source parsing and rules runtime evaluation.
//! Usage: cargo test --test falsification_cron_rules

use forjar::core::cron_source::{matches, parse_cron, schedule_summary, CronTime};
use forjar::core::rules_runtime::{
    evaluate_event, fired_actions, matching_rulebooks, runtime_summary,
};
use forjar::core::types::{
    ApplyAction, CooldownTracker, EventPattern, EventType, InfraEvent, NotifyAction, Rulebook,
    RulebookAction, RulebookConfig,
};
use std::collections::HashMap;

// ============================================================================
// FJ-3103: parse_cron
// ============================================================================

#[test]
fn cron_all_stars() {
    let sched = parse_cron("* * * * *").unwrap();
    assert_eq!(sched.minutes.len(), 60);
    assert_eq!(sched.hours.len(), 24);
    assert_eq!(sched.days_of_month.len(), 31);
    assert_eq!(sched.months.len(), 12);
    assert_eq!(sched.days_of_week.len(), 7);
}

#[test]
fn cron_exact_values() {
    let sched = parse_cron("30 12 15 6 3").unwrap();
    assert!(sched.minutes.contains(&30));
    assert_eq!(sched.minutes.len(), 1);
    assert!(sched.hours.contains(&12));
    assert!(sched.days_of_month.contains(&15));
    assert!(sched.months.contains(&6));
    assert!(sched.days_of_week.contains(&3));
}

#[test]
fn cron_step() {
    let sched = parse_cron("*/15 * * * *").unwrap();
    assert!(sched.minutes.contains(&0));
    assert!(sched.minutes.contains(&15));
    assert!(sched.minutes.contains(&30));
    assert!(sched.minutes.contains(&45));
    assert_eq!(sched.minutes.len(), 4);
}

#[test]
fn cron_range() {
    let sched = parse_cron("* 9-17 * * *").unwrap();
    assert_eq!(sched.hours.len(), 9);
    assert!(sched.hours.contains(&9));
    assert!(sched.hours.contains(&17));
    assert!(!sched.hours.contains(&8));
    assert!(!sched.hours.contains(&18));
}

#[test]
fn cron_list() {
    let sched = parse_cron("0 8,12,18 * * *").unwrap();
    assert_eq!(sched.hours.len(), 3);
    assert!(sched.hours.contains(&8));
    assert!(sched.hours.contains(&12));
    assert!(sched.hours.contains(&18));
}

#[test]
fn cron_mixed_fields() {
    let sched = parse_cron("0,30 */6 1-15 * 1-5").unwrap();
    assert_eq!(sched.minutes.len(), 2);
    assert_eq!(sched.hours.len(), 4); // 0,6,12,18
    assert_eq!(sched.days_of_month.len(), 15);
    assert_eq!(sched.days_of_week.len(), 5);
}

#[test]
fn cron_invalid_too_few_fields() {
    assert!(parse_cron("* * *").is_err());
}

#[test]
fn cron_invalid_too_many_fields() {
    assert!(parse_cron("* * * * * *").is_err());
}

#[test]
fn cron_invalid_value() {
    assert!(parse_cron("99 * * * *").is_err());
}

#[test]
fn cron_invalid_empty() {
    assert!(parse_cron("").is_err());
}

// ============================================================================
// FJ-3103: matches
// ============================================================================

#[test]
fn cron_matches_exact() {
    let sched = parse_cron("30 12 15 6 3").unwrap();
    let time = CronTime {
        minute: 30,
        hour: 12,
        day: 15,
        month: 6,
        weekday: 3,
    };
    assert!(matches(&sched, &time));
}

#[test]
fn cron_no_match_minute() {
    let sched = parse_cron("30 12 * * *").unwrap();
    let time = CronTime {
        minute: 31,
        hour: 12,
        day: 1,
        month: 1,
        weekday: 0,
    };
    assert!(!matches(&sched, &time));
}

#[test]
fn cron_matches_star() {
    let sched = parse_cron("* * * * *").unwrap();
    let time = CronTime {
        minute: 42,
        hour: 3,
        day: 28,
        month: 11,
        weekday: 5,
    };
    assert!(matches(&sched, &time));
}

#[test]
fn cron_matches_step() {
    let sched = parse_cron("*/15 * * * *").unwrap();
    let time = CronTime {
        minute: 45,
        hour: 0,
        day: 1,
        month: 1,
        weekday: 0,
    };
    assert!(matches(&sched, &time));
    let time2 = CronTime {
        minute: 7,
        hour: 0,
        day: 1,
        month: 1,
        weekday: 0,
    };
    assert!(!matches(&sched, &time2));
}

#[test]
fn cron_matches_weekday_boundary() {
    let sched = parse_cron("0 0 * * 0").unwrap(); // Sunday
    let sunday = CronTime {
        minute: 0,
        hour: 0,
        day: 1,
        month: 1,
        weekday: 0,
    };
    assert!(matches(&sched, &sunday));
    let monday = CronTime {
        minute: 0,
        hour: 0,
        day: 2,
        month: 1,
        weekday: 1,
    };
    assert!(!matches(&sched, &monday));
}

// ============================================================================
// FJ-3103: schedule_summary
// ============================================================================

#[test]
fn summary_every_minute() {
    let sched = parse_cron("* * * * *").unwrap();
    let s = schedule_summary(&sched);
    assert!(!s.is_empty());
}

#[test]
fn summary_specific() {
    let sched = parse_cron("0 9 * * 1-5").unwrap();
    let s = schedule_summary(&sched);
    assert!(!s.is_empty());
}

// ============================================================================
// Helpers for rules runtime
// ============================================================================

fn file_changed_event() -> InfraEvent {
    let mut payload = HashMap::new();
    payload.insert("path".into(), "/etc/nginx/nginx.conf".into());
    InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web1".into()),
        payload,
    }
}

fn cron_event() -> InfraEvent {
    InfraEvent {
        event_type: EventType::CronFired,
        timestamp: "2026-03-09T00:00:00Z".into(),
        machine: None,
        payload: HashMap::new(),
    }
}

fn config_repair_rulebook() -> Rulebook {
    Rulebook {
        name: "config-repair".into(),
        description: Some("Repair nginx config on change".into()),
        events: vec![EventPattern {
            event_type: EventType::FileChanged,
            match_fields: {
                let mut m = HashMap::new();
                m.insert("path".into(), "/etc/nginx/nginx.conf".into());
                m
            },
        }],
        conditions: vec![],
        actions: vec![RulebookAction {
            apply: Some(ApplyAction {
                file: "forjar.yaml".into(),
                subset: vec![],
                tags: vec!["config".into()],
                machine: None,
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
        name: "alert-on-cron".into(),
        description: None,
        events: vec![EventPattern {
            event_type: EventType::CronFired,
            match_fields: HashMap::new(),
        }],
        conditions: vec![],
        actions: vec![RulebookAction {
            apply: None,
            destroy: None,
            script: None,
            notify: Some(NotifyAction {
                channel: "slack://alerts".into(),
                message: "Cron fired".into(),
            }),
        }],
        cooldown_secs: 60,
        max_retries: 1,
        enabled: true,
    }
}

fn disabled_rulebook() -> Rulebook {
    let mut rb = config_repair_rulebook();
    rb.name = "disabled-repair".into();
    rb.enabled = false;
    rb
}

fn test_config() -> RulebookConfig {
    RulebookConfig {
        rulebooks: vec![
            config_repair_rulebook(),
            alert_rulebook(),
            disabled_rulebook(),
        ],
    }
}

// ============================================================================
// FJ-3106: evaluate_event
// ============================================================================

#[test]
fn eval_matching_event() {
    let config = test_config();
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&file_changed_event(), &config, &mut tracker);
    // config-repair matches, disabled-repair is disabled
    let names: Vec<&str> = results.iter().map(|r| r.rulebook.as_str()).collect();
    assert!(names.contains(&"config-repair"));
}

#[test]
fn eval_no_match() {
    let config = test_config();
    let mut tracker = CooldownTracker::default();
    let event = InfraEvent {
        event_type: EventType::Manual,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: None,
        payload: HashMap::new(),
    };
    let results = evaluate_event(&event, &config, &mut tracker);
    let active: Vec<_> = results
        .iter()
        .filter(|r| !r.disabled && !r.cooldown_blocked)
        .collect();
    assert!(active.is_empty());
}

#[test]
fn eval_disabled_rulebook() {
    let config = test_config();
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&file_changed_event(), &config, &mut tracker);
    let disabled: Vec<_> = results.iter().filter(|r| r.disabled).collect();
    // disabled-repair should show as disabled
    assert!(disabled.iter().any(|r| r.rulebook == "disabled-repair"));
}

#[test]
fn eval_cron_event() {
    let config = test_config();
    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&cron_event(), &config, &mut tracker);
    let names: Vec<&str> = results
        .iter()
        .filter(|r| !r.disabled && !r.cooldown_blocked)
        .map(|r| r.rulebook.as_str())
        .collect();
    assert!(names.contains(&"alert-on-cron"));
}

// ============================================================================
// FJ-3106: fired_actions
// ============================================================================

#[test]
fn fired_actions_match() {
    let config = test_config();
    let mut tracker = CooldownTracker::default();
    let actions = fired_actions(&file_changed_event(), &config, &mut tracker);
    assert!(!actions.is_empty());
    let (name, acts) = &actions[0];
    assert_eq!(name, "config-repair");
    assert_eq!(acts[0].action_type(), "apply");
}

#[test]
fn fired_actions_no_match() {
    let config = test_config();
    let mut tracker = CooldownTracker::default();
    let event = InfraEvent {
        event_type: EventType::WebhookReceived,
        timestamp: "t".into(),
        machine: None,
        payload: HashMap::new(),
    };
    let actions = fired_actions(&event, &config, &mut tracker);
    assert!(actions.is_empty());
}

// ============================================================================
// FJ-3106: matching_rulebooks
// ============================================================================

#[test]
fn matching_file_event() {
    let config = test_config();
    let matched = matching_rulebooks(&file_changed_event(), &config);
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].name, "config-repair");
}

#[test]
fn matching_cron_event() {
    let config = test_config();
    let matched = matching_rulebooks(&cron_event(), &config);
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].name, "alert-on-cron");
}

#[test]
fn matching_disabled_excluded() {
    let config = test_config();
    let matched = matching_rulebooks(&file_changed_event(), &config);
    assert!(!matched.iter().any(|r| r.name == "disabled-repair"));
}

// ============================================================================
// FJ-3106: runtime_summary
// ============================================================================

#[test]
fn summary_counts() {
    let config = test_config();
    let tracker = CooldownTracker::default();
    let summary = runtime_summary(&config, &tracker);
    assert_eq!(summary.total_rulebooks, 3);
    assert_eq!(summary.enabled, 2);
    assert_eq!(summary.disabled, 1);
    assert_eq!(summary.in_cooldown, 0);
}

#[test]
fn summary_empty_config() {
    let config = RulebookConfig { rulebooks: vec![] };
    let tracker = CooldownTracker::default();
    let summary = runtime_summary(&config, &tracker);
    assert_eq!(summary.total_rulebooks, 0);
    assert_eq!(summary.enabled, 0);
}

// ============================================================================
// RulebookAction::action_type
// ============================================================================

#[test]
fn action_type_apply() {
    let action = RulebookAction {
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
    assert_eq!(action.action_type(), "apply");
}

#[test]
fn action_type_script() {
    let action = RulebookAction {
        apply: None,
        destroy: None,
        script: Some("echo test".into()),
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
            channel: "slack".into(),
            message: "hi".into(),
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
// EventType Display
// ============================================================================

#[test]
fn event_type_display() {
    assert_eq!(EventType::FileChanged.to_string(), "file_changed");
    assert_eq!(EventType::CronFired.to_string(), "cron_fired");
    assert_eq!(EventType::Manual.to_string(), "manual");
}

// ============================================================================
// Event pattern matching
// ============================================================================

#[test]
fn pattern_match_with_payload() {
    use forjar::core::types::event_matches_pattern;
    let event = file_changed_event();
    let pattern = EventPattern {
        event_type: EventType::FileChanged,
        match_fields: {
            let mut m = HashMap::new();
            m.insert("path".into(), "/etc/nginx/nginx.conf".into());
            m
        },
    };
    assert!(event_matches_pattern(&event, &pattern));
}

#[test]
fn pattern_no_match_wrong_type() {
    use forjar::core::types::event_matches_pattern;
    let event = file_changed_event();
    let pattern = EventPattern {
        event_type: EventType::Manual,
        match_fields: HashMap::new(),
    };
    assert!(!event_matches_pattern(&event, &pattern));
}

#[test]
fn pattern_no_match_wrong_payload() {
    use forjar::core::types::event_matches_pattern;
    let event = file_changed_event();
    let pattern = EventPattern {
        event_type: EventType::FileChanged,
        match_fields: {
            let mut m = HashMap::new();
            m.insert("path".into(), "/etc/other.conf".into());
            m
        },
    };
    assert!(!event_matches_pattern(&event, &pattern));
}
