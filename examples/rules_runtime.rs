//! Example: Rulebook runtime evaluator (FJ-3106)
//!
//! Demonstrates event-driven automation with cooldown deduplication:
//! events trigger rulebooks, cooldowns prevent rapid-fire triggering.
//!
//! ```bash
//! cargo run --example rules_runtime
//! ```

use forjar::core::rules_runtime;
use forjar::core::types::{
    ApplyAction, CooldownTracker, EventPattern, EventType, InfraEvent, Rulebook, RulebookAction,
    RulebookConfig,
};
use std::collections::HashMap;

fn main() {
    println!("=== Rulebook Runtime Evaluator (FJ-3106) ===\n");

    // Build a rulebook config
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
                        machine: None,
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
                    script: Some("echo 'alert triggered'".into()),
                    notify: None,
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
        payload: HashMap::new(),
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

    println!("\nDone.");
}
