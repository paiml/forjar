//! FJ-3106: Rulebook runtime evaluator with cooldown + deduplication.
//!
//! Evaluates incoming infrastructure events against configured rulebooks,
//! enforces cooldown periods, tracks retry counts, and produces an
//! ordered list of actions to execute.

use crate::core::types::{
    event_matches_rulebook, CooldownTracker, InfraEvent, Rulebook, RulebookAction, RulebookConfig,
};

/// Result of evaluating an event against all rulebooks.
#[derive(Debug, Clone)]
pub struct EvalResult {
    /// Matched rulebook name.
    pub rulebook: String,
    /// Actions to execute.
    pub actions: Vec<RulebookAction>,
    /// Whether the rulebook was blocked by cooldown.
    pub cooldown_blocked: bool,
    /// Whether the rulebook was disabled.
    pub disabled: bool,
}

/// Evaluate an event against all rulebooks, respecting cooldowns.
pub fn evaluate_event(
    event: &InfraEvent,
    config: &RulebookConfig,
    tracker: &mut CooldownTracker,
) -> Vec<EvalResult> {
    let mut results = Vec::new();

    for rb in &config.rulebooks {
        if !rb.enabled {
            results.push(EvalResult {
                rulebook: rb.name.clone(),
                actions: Vec::new(),
                cooldown_blocked: false,
                disabled: true,
            });
            continue;
        }

        if !event_matches_rulebook(event, rb) {
            continue;
        }

        // Check cooldown
        if !tracker.can_fire(&rb.name, rb.cooldown_secs) {
            results.push(EvalResult {
                rulebook: rb.name.clone(),
                actions: Vec::new(),
                cooldown_blocked: true,
                disabled: false,
            });
            continue;
        }

        // Fire: record and return actions
        tracker.record_fire(&rb.name);
        results.push(EvalResult {
            rulebook: rb.name.clone(),
            actions: rb.actions.clone(),
            cooldown_blocked: false,
            disabled: false,
        });
    }

    results
}

/// Evaluate an event and return only the fired actions (no blocked/disabled).
pub fn fired_actions(
    event: &InfraEvent,
    config: &RulebookConfig,
    tracker: &mut CooldownTracker,
) -> Vec<(String, Vec<RulebookAction>)> {
    evaluate_event(event, config, tracker)
        .into_iter()
        .filter(|r| !r.cooldown_blocked && !r.disabled && !r.actions.is_empty())
        .map(|r| (r.rulebook, r.actions))
        .collect()
}

/// Count how many rulebooks would match an event (ignoring cooldowns).
pub fn matching_rulebooks<'a>(event: &InfraEvent, config: &'a RulebookConfig) -> Vec<&'a Rulebook> {
    config
        .rulebooks
        .iter()
        .filter(|rb| event_matches_rulebook(event, rb))
        .collect()
}

/// Summary of runtime evaluation state.
#[derive(Debug, Clone)]
pub struct RuntimeSummary {
    /// Total rulebooks loaded.
    pub total_rulebooks: usize,
    /// Enabled rulebooks.
    pub enabled: usize,
    /// Disabled rulebooks.
    pub disabled: usize,
    /// Rulebooks currently in cooldown.
    pub in_cooldown: usize,
}

/// Produce a runtime summary for the current state.
pub fn runtime_summary(config: &RulebookConfig, tracker: &CooldownTracker) -> RuntimeSummary {
    let enabled = config.rulebooks.iter().filter(|rb| rb.enabled).count();
    let in_cooldown = config
        .rulebooks
        .iter()
        .filter(|rb| rb.enabled && !tracker.can_fire(&rb.name, rb.cooldown_secs))
        .count();

    RuntimeSummary {
        total_rulebooks: config.rulebooks.len(),
        enabled,
        disabled: config.rulebooks.len() - enabled,
        in_cooldown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{
        ApplyAction, DestroyAction, EventPattern, EventType, NotifyAction, RulebookAction,
    };
    use std::collections::HashMap;

    fn make_event(event_type: EventType) -> InfraEvent {
        InfraEvent {
            event_type,
            timestamp: "2026-03-09T12:00:00Z".into(),
            machine: Some("web-1".into()),
            payload: HashMap::new(),
        }
    }

    fn make_event_with_payload(event_type: EventType, key: &str, val: &str) -> InfraEvent {
        let mut payload = HashMap::new();
        payload.insert(key.to_string(), val.to_string());
        InfraEvent {
            event_type,
            timestamp: "2026-03-09T12:00:00Z".into(),
            machine: Some("web-1".into()),
            payload,
        }
    }

    fn make_rulebook(name: &str, event_type: EventType, cooldown: u64) -> Rulebook {
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

    #[test]
    fn evaluate_matching_event() {
        let config = RulebookConfig {
            rulebooks: vec![make_rulebook("restart-nginx", EventType::FileChanged, 30)],
        };
        let event = make_event(EventType::FileChanged);
        let mut tracker = CooldownTracker::default();

        let results = evaluate_event(&event, &config, &mut tracker);
        assert_eq!(results.len(), 1);
        assert!(!results[0].cooldown_blocked);
        assert!(!results[0].disabled);
        assert_eq!(results[0].actions.len(), 1);
    }

    #[test]
    fn evaluate_no_match() {
        let config = RulebookConfig {
            rulebooks: vec![make_rulebook("restart-nginx", EventType::FileChanged, 30)],
        };
        let event = make_event(EventType::CronFired);
        let mut tracker = CooldownTracker::default();

        let results = evaluate_event(&event, &config, &mut tracker);
        assert!(results.is_empty());
    }

    #[test]
    fn cooldown_blocks_second_fire() {
        let config = RulebookConfig {
            rulebooks: vec![make_rulebook("restart-nginx", EventType::FileChanged, 60)],
        };
        let event = make_event(EventType::FileChanged);
        let mut tracker = CooldownTracker::default();

        // First fire — should succeed
        let r1 = evaluate_event(&event, &config, &mut tracker);
        assert_eq!(r1.len(), 1);
        assert!(!r1[0].cooldown_blocked);

        // Second fire — should be blocked by cooldown
        let r2 = evaluate_event(&event, &config, &mut tracker);
        assert_eq!(r2.len(), 1);
        assert!(r2[0].cooldown_blocked);
    }

    #[test]
    fn zero_cooldown_always_fires() {
        let config = RulebookConfig {
            rulebooks: vec![make_rulebook("rapid-fire", EventType::Manual, 0)],
        };
        let event = make_event(EventType::Manual);
        let mut tracker = CooldownTracker::default();

        let r1 = evaluate_event(&event, &config, &mut tracker);
        assert!(!r1[0].cooldown_blocked);

        let r2 = evaluate_event(&event, &config, &mut tracker);
        assert!(!r2[0].cooldown_blocked);
    }

    #[test]
    fn disabled_rulebook_skipped() {
        let mut rb = make_rulebook("disabled-rb", EventType::FileChanged, 0);
        rb.enabled = false;

        let config = RulebookConfig {
            rulebooks: vec![rb],
        };
        let event = make_event(EventType::FileChanged);
        let mut tracker = CooldownTracker::default();

        let results = evaluate_event(&event, &config, &mut tracker);
        assert_eq!(results.len(), 1);
        assert!(results[0].disabled);
        assert!(results[0].actions.is_empty());
    }

    #[test]
    fn multiple_rulebooks_match() {
        let config = RulebookConfig {
            rulebooks: vec![
                make_rulebook("rb1", EventType::FileChanged, 0),
                make_rulebook("rb2", EventType::FileChanged, 0),
            ],
        };
        let event = make_event(EventType::FileChanged);
        let mut tracker = CooldownTracker::default();

        let results = evaluate_event(&event, &config, &mut tracker);
        assert_eq!(results.len(), 2);
        assert!(!results[0].cooldown_blocked);
        assert!(!results[1].cooldown_blocked);
    }

    #[test]
    fn fired_actions_filters_blocked() {
        let config = RulebookConfig {
            rulebooks: vec![make_rulebook("rb1", EventType::FileChanged, 60)],
        };
        let event = make_event(EventType::FileChanged);
        let mut tracker = CooldownTracker::default();

        // First fire
        let actions = fired_actions(&event, &config, &mut tracker);
        assert_eq!(actions.len(), 1);

        // Second fire — blocked
        let actions = fired_actions(&event, &config, &mut tracker);
        assert!(actions.is_empty());
    }

    #[test]
    fn matching_rulebooks_ignores_cooldown() {
        let config = RulebookConfig {
            rulebooks: vec![make_rulebook("rb", EventType::Manual, 0)],
        };
        let event = make_event(EventType::Manual);
        let matched = matching_rulebooks(&event, &config);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "rb");
    }

    #[test]
    fn runtime_summary_counts() {
        let mut rb_disabled = make_rulebook("disabled", EventType::Manual, 0);
        rb_disabled.enabled = false;

        let config = RulebookConfig {
            rulebooks: vec![
                make_rulebook("active", EventType::FileChanged, 0),
                rb_disabled,
            ],
        };
        let tracker = CooldownTracker::default();

        let summary = runtime_summary(&config, &tracker);
        assert_eq!(summary.total_rulebooks, 2);
        assert_eq!(summary.enabled, 1);
        assert_eq!(summary.disabled, 1);
        assert_eq!(summary.in_cooldown, 0);
    }

    #[test]
    fn event_with_payload_match() {
        let mut rb = make_rulebook("payload-match", EventType::FileChanged, 0);
        rb.events[0]
            .match_fields
            .insert("path".into(), "/etc/nginx/nginx.conf".into());

        let config = RulebookConfig {
            rulebooks: vec![rb],
        };
        let event =
            make_event_with_payload(EventType::FileChanged, "path", "/etc/nginx/nginx.conf");
        let mut tracker = CooldownTracker::default();

        let results = evaluate_event(&event, &config, &mut tracker);
        assert_eq!(results.len(), 1);
        assert!(!results[0].cooldown_blocked);
    }

    #[test]
    fn event_payload_mismatch() {
        let mut rb = make_rulebook("payload-match", EventType::FileChanged, 0);
        rb.events[0]
            .match_fields
            .insert("path".into(), "/etc/nginx/nginx.conf".into());

        let config = RulebookConfig {
            rulebooks: vec![rb],
        };
        let event = make_event_with_payload(EventType::FileChanged, "path", "/etc/other.conf");
        let mut tracker = CooldownTracker::default();

        let results = evaluate_event(&event, &config, &mut tracker);
        assert!(results.is_empty());
    }

    #[test]
    fn notify_action_type() {
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
                    message: "CPU threshold exceeded".into(),
                }),
            }],
            cooldown_secs: 300,
            max_retries: 1,
            enabled: true,
        };

        let config = RulebookConfig {
            rulebooks: vec![rb],
        };
        let event = make_event(EventType::MetricThreshold);
        let mut tracker = CooldownTracker::default();

        let results = evaluate_event(&event, &config, &mut tracker);
        assert_eq!(results.len(), 1);
        assert!(results[0].actions[0].notify.is_some());
        assert_eq!(results[0].actions[0].action_type(), "notify");
    }

    #[test]
    fn destroy_action_type() {
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
                    resources: vec!["temp-cache".into()],
                }),
                script: None,
                notify: None,
            }],
            cooldown_secs: 3600,
            max_retries: 0,
            enabled: true,
        };

        let config = RulebookConfig {
            rulebooks: vec![rb],
        };
        let event = make_event(EventType::CronFired);
        let mut tracker = CooldownTracker::default();

        let results = evaluate_event(&event, &config, &mut tracker);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].actions[0].action_type(), "destroy");
    }
}
