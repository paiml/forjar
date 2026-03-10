//! Example: Rulebook runtime evaluator with template expansion (FJ-3103, FJ-3106)
//!
//! Demonstrates event-driven automation with cooldown deduplication:
//! events trigger rulebooks, cooldowns prevent rapid-fire triggering.
//! Also shows how `rulebook_template::expand_action` resolves
//! `{{ event.* }}` variables in action fields before execution.
//!
//! ```bash
//! cargo run --example rules_runtime
//! ```

use forjar::core::rulebook_template::expand_action;
use forjar::core::rules_runtime;
use forjar::core::types::{
    ApplyAction, CooldownTracker, EventPattern, EventType, InfraEvent, NotifyAction, Rulebook,
    RulebookAction, RulebookConfig,
};
use std::collections::HashMap;

fn main() {
    println!("=== Rulebook Runtime + Template Expansion (FJ-3103 / FJ-3106) ===\n");

    // Build a rulebook config with template variables in actions
    let config = RulebookConfig {
        rulebooks: vec![
            Rulebook {
                name: "config-repair".into(),
                description: Some("Re-apply when config files change".into()),
                events: vec![EventPattern {
                    event_type: EventType::FileChanged,
                    match_fields: {
                        let mut m = HashMap::new();
                        m.insert("path".into(), "/etc/nginx/nginx.conf".into());
                        m
                    },
                }],
                conditions: Vec::new(),
                actions: vec![RulebookAction {
                    apply: Some(ApplyAction {
                        file: "forjar.yaml".into(),
                        subset: vec!["nginx-config".into()],
                        tags: Vec::new(),
                        machine: Some("{{ event.machine }}".into()),
                    }),
                    destroy: None,
                    script: None,
                    notify: None,
                }],
                cooldown_secs: 30,
                max_retries: 3,
                enabled: true,
            },
            Rulebook {
                name: "alert-threshold".into(),
                description: Some("Notify on metric threshold".into()),
                events: vec![EventPattern {
                    event_type: EventType::MetricThreshold,
                    match_fields: HashMap::new(),
                }],
                conditions: Vec::new(),
                actions: vec![RulebookAction {
                    apply: None,
                    destroy: None,
                    script: Some(
                        "echo 'alert on {{ event.machine }} at {{ event.timestamp }}'".into(),
                    ),
                    notify: Some(NotifyAction {
                        channel: "#ops-{{ event.env }}".into(),
                        message: "Threshold breach on {{ event.machine }}: {{ event.metric }}"
                            .into(),
                    }),
                }],
                cooldown_secs: 60,
                max_retries: 1,
                enabled: true,
            },
        ],
    };

    let mut tracker = CooldownTracker::default();

    // Show initial summary
    let summary = rules_runtime::runtime_summary(&config, &tracker);
    println!("1. Runtime Summary:");
    println!("  Total rulebooks: {}", summary.total_rulebooks);
    println!("  Enabled: {}", summary.enabled);
    println!("  Disabled: {}", summary.disabled);
    println!("  In cooldown: {}", summary.in_cooldown);

    // Event 1: matching file change
    println!("\n2. Event: nginx.conf changed");
    let event1 = InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web-1".into()),
        payload: {
            let mut m = HashMap::new();
            m.insert("path".into(), "/etc/nginx/nginx.conf".into());
            m
        },
    };

    let results = rules_runtime::evaluate_event(&event1, &config, &mut tracker);
    for r in &results {
        println!(
            "  Rulebook: {} | blocked={} | actions={}",
            r.rulebook,
            r.cooldown_blocked,
            r.actions.len()
        );
    }

    // Event 2: same event again (should be blocked by cooldown)
    println!("\n3. Same event again (cooldown active):");
    let results = rules_runtime::evaluate_event(&event1, &config, &mut tracker);
    for r in &results {
        println!(
            "  Rulebook: {} | blocked={} | actions={}",
            r.rulebook,
            r.cooldown_blocked,
            r.actions.len()
        );
    }

    // Event 3: different type (metric threshold)
    println!("\n4. Event: metric threshold crossed");
    let event2 = InfraEvent {
        event_type: EventType::MetricThreshold,
        timestamp: "2026-03-09T12:00:05Z".into(),
        machine: Some("web-1".into()),
        payload: {
            let mut m = HashMap::new();
            m.insert("metric".into(), "cpu_percent".into());
            m.insert("env".into(), "prod".into());
            m
        },
    };
    let fired = rules_runtime::fired_actions(&event2, &config, &mut tracker);
    for (name, actions) in &fired {
        println!("  Fired: {} ({} actions)", name, actions.len());
        for a in actions {
            println!("    Type: {}", a.action_type());
        }
    }

    // Updated summary (with cooldowns active)
    println!("\n5. Updated Summary:");
    let summary = rules_runtime::runtime_summary(&config, &tracker);
    println!("  In cooldown: {}", summary.in_cooldown);

    // Event 4: unmatched event
    println!("\n6. Unmatched event (manual trigger):");
    let event3 = InfraEvent {
        event_type: EventType::Manual,
        timestamp: "2026-03-09T12:00:10Z".into(),
        machine: None,
        payload: HashMap::new(),
    };
    let matched = rules_runtime::matching_rulebooks(&event3, &config);
    println!("  Matching rulebooks: {}", matched.len());

    // 7. Template expansion demonstration
    println!("\n7. Template Expansion (rulebook_template::expand_action):");

    // Expand a script action with event context
    let script_action = RulebookAction {
        script: Some(
            "deploy {{ event.env }} on {{ event.machine }} at {{ event.timestamp }}".into(),
        ),
        apply: None,
        destroy: None,
        notify: None,
    };
    let deploy_event = InfraEvent {
        event_type: EventType::WebhookReceived,
        timestamp: "2026-03-10T09:30:00Z".into(),
        machine: Some("app-server-01".into()),
        payload: {
            let mut m = HashMap::new();
            m.insert("env".into(), "production".into());
            m.insert("region".into(), "us-east-1".into());
            m
        },
    };
    let expanded = expand_action(&script_action, &deploy_event);
    println!("  Template: {:?}", script_action.script.as_deref().unwrap());
    println!("  Expanded: {:?}", expanded.script.as_deref().unwrap());

    // Expand a notify action
    let notify_action = RulebookAction {
        script: None,
        apply: None,
        destroy: None,
        notify: Some(NotifyAction {
            channel: "#deploy-{{ event.region }}".into(),
            message: "[{{ event.type }}] {{ event.env }} deploy on {{ event.machine }}".into(),
        }),
    };
    let expanded = expand_action(&notify_action, &deploy_event);
    let n = expanded.notify.as_ref().unwrap();
    println!("\n  Notify template:");
    println!(
        "    channel: #deploy-{{{{ event.region }}}} -> {}",
        n.channel
    );
    println!("    message: [{{{{ event.type }}}}] ... -> {}", n.message);

    // Expand an apply action with machine targeting
    let apply_action = RulebookAction {
        script: None,
        apply: Some(ApplyAction {
            file: "{{ event.env }}.yaml".into(),
            subset: vec!["web".into()],
            tags: vec!["config".into()],
            machine: Some("{{ event.machine }}".into()),
        }),
        destroy: None,
        notify: None,
    };
    let expanded = expand_action(&apply_action, &deploy_event);
    let a = expanded.apply.as_ref().unwrap();
    println!("\n  Apply template:");
    println!("    file: {{{{ event.env }}}}.yaml -> {}", a.file);
    println!(
        "    machine: {{{{ event.machine }}}} -> {}",
        a.machine.as_deref().unwrap()
    );

    // Partial expansion: unknown variables stay as-is
    let partial = RulebookAction {
        script: Some("{{ event.env }} + {{ unknown.var }}".into()),
        apply: None,
        destroy: None,
        notify: None,
    };
    let expanded = expand_action(&partial, &deploy_event);
    println!(
        "\n  Partial expansion: {:?} -> {:?}",
        partial.script.as_deref().unwrap(),
        expanded.script.as_deref().unwrap()
    );

    println!("\nDone.");
}
