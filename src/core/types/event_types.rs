//! FJ-3100: Event-driven automation types.
//!
//! Defines event sources, rulebooks, pattern matching, and action types
//! for reactive infrastructure convergence.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An infrastructure event that can trigger rulebook actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfraEvent {
    /// Event type identifier.
    pub event_type: EventType,
    /// Timestamp (ISO 8601).
    pub timestamp: String,
    /// Machine where the event originated.
    #[serde(default)]
    pub machine: Option<String>,
    /// Event-specific payload fields.
    #[serde(default)]
    pub payload: HashMap<String, String>,
}

/// Supported event types for the event-driven engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// File system change detected (via inotify/fanotify).
    FileChanged,
    /// Process exited (tracked via waitpid).
    ProcessExit,
    /// Cron schedule fired.
    CronFired,
    /// HTTP webhook received.
    WebhookReceived,
    /// Metric threshold crossed.
    MetricThreshold,
    /// Manual trigger (`forjar trigger <rulebook>`).
    Manual,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::FileChanged => "file_changed",
            Self::ProcessExit => "process_exit",
            Self::CronFired => "cron_fired",
            Self::WebhookReceived => "webhook_received",
            Self::MetricThreshold => "metric_threshold",
            Self::Manual => "manual",
        };
        write!(f, "{s}")
    }
}

/// A rulebook entry — maps events to actions.
///
/// # Examples
///
/// ```
/// use forjar::core::types::Rulebook;
///
/// let yaml = r#"
/// name: config-repair
/// events:
///   - type: file_changed
///     match:
///       path: /etc/nginx/nginx.conf
/// actions:
///   - apply:
///       file: forjar.yaml
///       tags: [config]
/// cooldown_secs: 30
/// "#;
/// let rb: Rulebook = serde_yaml_ng::from_str(yaml).unwrap();
/// assert_eq!(rb.name, "config-repair");
/// assert_eq!(rb.events.len(), 1);
/// assert_eq!(rb.cooldown_secs, 30);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rulebook {
    /// Rulebook name (unique identifier).
    pub name: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Event patterns that trigger this rulebook.
    pub events: Vec<EventPattern>,

    /// Conditions that must be true (template expressions).
    #[serde(default)]
    pub conditions: Vec<String>,

    /// Actions to execute when triggered.
    pub actions: Vec<RulebookAction>,

    /// Minimum seconds between activations (deduplication).
    #[serde(default = "default_cooldown")]
    pub cooldown_secs: u64,

    /// Maximum retry attempts for failed actions.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Whether this rulebook is enabled.
    #[serde(default = "crate::core::types::default_true")]
    pub enabled: bool,
}

fn default_cooldown() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}

/// An event pattern that triggers a rulebook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPattern {
    /// Event type to match.
    #[serde(rename = "type")]
    pub event_type: EventType,

    /// Key-value match conditions on event payload.
    #[serde(default, rename = "match")]
    pub match_fields: HashMap<String, String>,
}

/// An action to execute when a rulebook triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookAction {
    /// Apply a subset of resources.
    #[serde(default)]
    pub apply: Option<ApplyAction>,

    /// Destroy resources.
    #[serde(default)]
    pub destroy: Option<DestroyAction>,

    /// Run a shell script.
    #[serde(default)]
    pub script: Option<String>,

    /// Send a notification.
    #[serde(default)]
    pub notify: Option<NotifyAction>,
}

impl RulebookAction {
    /// Which action type is configured.
    pub fn action_type(&self) -> &str {
        if self.apply.is_some() {
            "apply"
        } else if self.destroy.is_some() {
            "destroy"
        } else if self.script.is_some() {
            "script"
        } else if self.notify.is_some() {
            "notify"
        } else {
            "unknown"
        }
    }
}

/// Apply action: run forjar apply on a subset of resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyAction {
    /// Config file to apply.
    pub file: String,
    /// Optional resource subset (by ID).
    #[serde(default)]
    pub subset: Vec<String>,
    /// Optional tag filter.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional target machine.
    #[serde(default)]
    pub machine: Option<String>,
}

/// Destroy action: remove resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestroyAction {
    /// Config file.
    pub file: String,
    /// Resources to destroy.
    #[serde(default)]
    pub resources: Vec<String>,
}

/// Notify action: send notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyAction {
    /// Notification channel (webhook URL, slack channel, etc.).
    pub channel: String,
    /// Message template.
    pub message: String,
}

/// Cooldown tracker for deduplication.
#[derive(Debug, Clone, Default)]
pub struct CooldownTracker {
    last_fired: HashMap<String, std::time::Instant>,
}

impl CooldownTracker {
    /// Check if a rulebook can fire (cooldown expired).
    pub fn can_fire(&self, rulebook_name: &str, cooldown_secs: u64) -> bool {
        match self.last_fired.get(rulebook_name) {
            None => true,
            Some(last) => last.elapsed().as_secs() >= cooldown_secs,
        }
    }

    /// Record that a rulebook fired.
    pub fn record_fire(&mut self, rulebook_name: &str) {
        self.last_fired
            .insert(rulebook_name.to_string(), std::time::Instant::now());
    }
}

/// Check if an event matches a pattern.
pub fn event_matches_pattern(event: &InfraEvent, pattern: &EventPattern) -> bool {
    if event.event_type != pattern.event_type {
        return false;
    }
    // All match fields must be present in event payload with matching values
    for (key, expected) in &pattern.match_fields {
        match event.payload.get(key) {
            Some(actual) if actual == expected => {}
            _ => return false,
        }
    }
    true
}

/// Check if an event matches any pattern in a rulebook.
pub fn event_matches_rulebook(event: &InfraEvent, rulebook: &Rulebook) -> bool {
    if !rulebook.enabled {
        return false;
    }
    rulebook
        .events
        .iter()
        .any(|p| event_matches_pattern(event, p))
}

/// Result of evaluating a rulebook config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookConfig {
    /// Parsed rulebooks.
    pub rulebooks: Vec<Rulebook>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
