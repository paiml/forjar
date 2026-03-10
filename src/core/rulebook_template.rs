//! FJ-3103: Template expansion for rulebook actions.
//!
//! Expands `{{ event.* }}` and `{{ machine.* }}` variables in
//! rulebook action fields before execution.

use crate::core::types::{ApplyAction, InfraEvent, NotifyAction, RulebookAction};
use std::collections::HashMap;

/// Expand template variables in a rulebook action.
///
/// Replaces `{{ event.type }}`, `{{ event.machine }}`, and any
/// `{{ event.<payload_key> }}` patterns with values from the event.
/// Variables that cannot be resolved are left as-is.
pub fn expand_action(action: &RulebookAction, event: &InfraEvent) -> RulebookAction {
    let vars = build_vars(event);
    let mut expanded = action.clone();

    // Expand script templates
    if let Some(ref script) = expanded.script {
        expanded.script = Some(expand_string(script, &vars));
    }

    // Expand notify message templates
    if let Some(ref notify) = expanded.notify {
        expanded.notify = Some(NotifyAction {
            channel: expand_string(&notify.channel, &vars),
            message: expand_string(&notify.message, &vars),
        });
    }

    // Expand apply fields
    if let Some(ref apply) = expanded.apply {
        expanded.apply = Some(ApplyAction {
            file: expand_string(&apply.file, &vars),
            subset: apply.subset.clone(),
            tags: apply.tags.clone(),
            machine: apply.machine.as_ref().map(|m| expand_string(m, &vars)),
        });
    }

    expanded
}

/// Build template variable map from event context.
fn build_vars(event: &InfraEvent) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    vars.insert("event.type".into(), event.event_type.to_string());
    vars.insert("event.timestamp".into(), event.timestamp.clone());

    if let Some(ref machine) = event.machine {
        vars.insert("event.machine".into(), machine.clone());
    }

    for (k, v) in &event.payload {
        vars.insert(format!("event.{k}"), v.clone());
    }

    vars
}

/// Expand `{{ var }}` patterns in a string.
///
/// Handles both `{{ var }}` (with spaces) and `{{var}}` (no spaces).
/// Unknown variables are left unexpanded.
fn expand_string(input: &str, vars: &HashMap<String, String>) -> String {
    let mut result = input.to_string();
    for (key, value) in vars {
        // With spaces: {{ key }}
        let pattern = format!("{{{{ {key} }}}}");
        result = result.replace(&pattern, value);
        // Without spaces: {{key}}
        let pattern_no_space = format!("{{{{{key}}}}}");
        result = result.replace(&pattern_no_space, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::EventType;

    fn sample_event() -> InfraEvent {
        let mut payload = HashMap::new();
        payload.insert("action".into(), "deploy".into());
        payload.insert("env".into(), "prod".into());
        payload.insert("region".into(), "us-east-1".into());
        InfraEvent {
            event_type: EventType::WebhookReceived,
            timestamp: "2026-03-10T00:00:00Z".into(),
            machine: Some("web-01".into()),
            payload,
        }
    }

    fn event_no_machine() -> InfraEvent {
        InfraEvent {
            event_type: EventType::CronFired,
            timestamp: "2026-03-10T12:00:00Z".into(),
            machine: None,
            payload: HashMap::new(),
        }
    }

    // -- expand_string tests --

    #[test]
    fn expand_string_with_spaces() {
        let vars = HashMap::from([("event.type".into(), "webhook_received".into())]);
        let result = expand_string("got {{ event.type }} event", &vars);
        assert_eq!(result, "got webhook_received event");
    }

    #[test]
    fn expand_string_no_spaces() {
        let vars = HashMap::from([("event.type".into(), "webhook_received".into())]);
        let result = expand_string("got {{event.type}} event", &vars);
        assert_eq!(result, "got webhook_received event");
    }

    #[test]
    fn expand_string_no_match() {
        let vars = HashMap::new();
        let result = expand_string("no {{ unknown.var }} here", &vars);
        assert_eq!(result, "no {{ unknown.var }} here");
    }

    #[test]
    fn expand_string_multiple_vars() {
        let vars = HashMap::from([("a".into(), "A".into()), ("b".into(), "B".into())]);
        let result = expand_string("{{ a }} and {{ b }}", &vars);
        assert_eq!(result, "A and B");
    }

    #[test]
    fn expand_string_empty_input() {
        let vars = HashMap::from([("x".into(), "X".into())]);
        let result = expand_string("", &vars);
        assert_eq!(result, "");
    }

    // -- build_vars tests --

    #[test]
    fn build_vars_includes_type() {
        let event = sample_event();
        let vars = build_vars(&event);
        assert_eq!(vars.get("event.type").unwrap(), "webhook_received");
    }

    #[test]
    fn build_vars_includes_timestamp() {
        let event = sample_event();
        let vars = build_vars(&event);
        assert_eq!(vars.get("event.timestamp").unwrap(), "2026-03-10T00:00:00Z");
    }

    #[test]
    fn build_vars_includes_machine() {
        let event = sample_event();
        let vars = build_vars(&event);
        assert_eq!(vars.get("event.machine").unwrap(), "web-01");
    }

    #[test]
    fn build_vars_no_machine() {
        let event = event_no_machine();
        let vars = build_vars(&event);
        assert!(!vars.contains_key("event.machine"));
    }

    #[test]
    fn build_vars_includes_payload() {
        let event = sample_event();
        let vars = build_vars(&event);
        assert_eq!(vars.get("event.action").unwrap(), "deploy");
        assert_eq!(vars.get("event.env").unwrap(), "prod");
        assert_eq!(vars.get("event.region").unwrap(), "us-east-1");
    }

    // -- expand_action tests --

    #[test]
    fn expand_action_script() {
        let action = RulebookAction {
            script: Some("deploy {{ event.env }} on {{ event.machine }}".into()),
            apply: None,
            destroy: None,
            notify: None,
        };
        let event = sample_event();
        let expanded = expand_action(&action, &event);
        assert_eq!(expanded.script.unwrap(), "deploy prod on web-01");
    }

    #[test]
    fn expand_action_notify() {
        let action = RulebookAction {
            script: None,
            apply: None,
            destroy: None,
            notify: Some(NotifyAction {
                channel: "#ops-{{ event.env }}".into(),
                message: "Event {{ event.type }} on {{ event.machine }}".into(),
            }),
        };
        let event = sample_event();
        let expanded = expand_action(&action, &event);
        let notify = expanded.notify.unwrap();
        assert_eq!(notify.channel, "#ops-prod");
        assert_eq!(notify.message, "Event webhook_received on web-01");
    }

    #[test]
    fn expand_action_apply_file() {
        let action = RulebookAction {
            script: None,
            apply: Some(ApplyAction {
                file: "{{ event.env }}.yaml".into(),
                subset: vec![],
                tags: vec!["config".into()],
                machine: Some("{{ event.machine }}".into()),
            }),
            destroy: None,
            notify: None,
        };
        let event = sample_event();
        let expanded = expand_action(&action, &event);
        let apply = expanded.apply.unwrap();
        assert_eq!(apply.file, "prod.yaml");
        assert_eq!(apply.machine.unwrap(), "web-01");
        assert_eq!(apply.tags, vec!["config"]);
    }

    #[test]
    fn expand_action_noop_no_templates() {
        let action = RulebookAction {
            script: Some("echo hello".into()),
            apply: None,
            destroy: None,
            notify: None,
        };
        let event = sample_event();
        let expanded = expand_action(&action, &event);
        assert_eq!(expanded.script.unwrap(), "echo hello");
    }

    #[test]
    fn expand_action_partial_expansion() {
        let action = RulebookAction {
            script: Some("{{ event.env }} and {{ unknown.var }}".into()),
            apply: None,
            destroy: None,
            notify: None,
        };
        let event = sample_event();
        let expanded = expand_action(&action, &event);
        assert_eq!(expanded.script.unwrap(), "prod and {{ unknown.var }}");
    }

    #[test]
    fn expand_action_no_fields_set() {
        let action = RulebookAction {
            script: None,
            apply: None,
            destroy: None,
            notify: None,
        };
        let event = sample_event();
        let expanded = expand_action(&action, &event);
        assert!(expanded.script.is_none());
        assert!(expanded.apply.is_none());
        assert!(expanded.notify.is_none());
        assert!(expanded.destroy.is_none());
    }

    #[test]
    fn expand_action_apply_no_machine() {
        let action = RulebookAction {
            script: None,
            apply: Some(ApplyAction {
                file: "{{ event.type }}.yaml".into(),
                subset: vec!["nginx".into()],
                tags: vec![],
                machine: None,
            }),
            destroy: None,
            notify: None,
        };
        let event = sample_event();
        let expanded = expand_action(&action, &event);
        let apply = expanded.apply.unwrap();
        assert_eq!(apply.file, "webhook_received.yaml");
        assert!(apply.machine.is_none());
        assert_eq!(apply.subset, vec!["nginx"]);
    }

    #[test]
    fn expand_action_payload_keys() {
        let action = RulebookAction {
            script: Some("region={{ event.region }}".into()),
            apply: None,
            destroy: None,
            notify: None,
        };
        let event = sample_event();
        let expanded = expand_action(&action, &event);
        assert_eq!(expanded.script.unwrap(), "region=us-east-1");
    }

    #[test]
    fn expand_action_timestamp() {
        let action = RulebookAction {
            script: Some("at {{ event.timestamp }}".into()),
            apply: None,
            destroy: None,
            notify: None,
        };
        let event = sample_event();
        let expanded = expand_action(&action, &event);
        assert_eq!(expanded.script.unwrap(), "at 2026-03-10T00:00:00Z");
    }

    #[test]
    fn expand_string_mixed_spacing() {
        let vars = HashMap::from([("x".into(), "42".into())]);
        // Both forms in one string
        let result = expand_string("{{ x }} and {{x}}", &vars);
        assert_eq!(result, "42 and 42");
    }
}
