//! FJ-3100: Event-driven automation — rulebook matching example.
//!
//! Demonstrates event pattern matching, rulebook evaluation,
//! cooldown tracking, and YAML rulebook configuration.

fn main() {
    use forjar::core::types::{
        event_matches_pattern, event_matches_rulebook, CooldownTracker, EventPattern, EventType,
        InfraEvent, Rulebook, RulebookConfig,
    };
    use std::collections::HashMap;

    println!("=== FJ-3100: Event-Driven Automation ===\n");

    // 1. Create an infrastructure event
    let mut payload = HashMap::new();
    payload.insert("path".to_string(), "/etc/nginx/nginx.conf".to_string());
    let event = InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web-01".into()),
        payload,
    };
    println!("Event: {} on {:?}", event.event_type, event.machine);
    println!("  payload: {:?}", event.payload);

    // 2. Pattern matching
    println!("\n--- Pattern Matching ---");

    let pattern_match = EventPattern {
        event_type: EventType::FileChanged,
        match_fields: {
            let mut m = HashMap::new();
            m.insert("path".into(), "/etc/nginx/nginx.conf".into());
            m
        },
    };
    println!(
        "Pattern (file_changed + path=/etc/nginx/nginx.conf): {}",
        event_matches_pattern(&event, &pattern_match)
    );

    let pattern_wrong_type = EventPattern {
        event_type: EventType::CronFired,
        match_fields: HashMap::new(),
    };
    println!(
        "Pattern (cron_fired): {}",
        event_matches_pattern(&event, &pattern_wrong_type)
    );

    let pattern_wrong_path = EventPattern {
        event_type: EventType::FileChanged,
        match_fields: {
            let mut m = HashMap::new();
            m.insert("path".into(), "/etc/hosts".into());
            m
        },
    };
    println!(
        "Pattern (file_changed + path=/etc/hosts): {}",
        event_matches_pattern(&event, &pattern_wrong_path)
    );

    // 3. Parse a rulebook from YAML
    println!("\n--- Rulebook YAML ---");
    let yaml = r#"
name: config-repair
description: "Auto-repair nginx config drift"
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
    println!("Rulebook: {} (enabled={})", rb.name, rb.enabled);
    println!("  events: {}", rb.events.len());
    println!("  actions: {}", rb.actions.len());
    println!(
        "  cooldown: {}s, retries: {}",
        rb.cooldown_secs, rb.max_retries
    );
    println!("  matches event: {}", event_matches_rulebook(&event, &rb));

    // 4. Cooldown tracking
    println!("\n--- Cooldown Tracker ---");
    let mut tracker = CooldownTracker::default();
    println!(
        "Can fire (first time): {}",
        tracker.can_fire("config-repair", 30)
    );
    tracker.record_fire("config-repair");
    println!(
        "Can fire (just fired, 30s cooldown): {}",
        tracker.can_fire("config-repair", 30)
    );
    println!(
        "Can fire (0s cooldown): {}",
        tracker.can_fire("config-repair", 0)
    );
    println!(
        "Can fire (other rule): {}",
        tracker.can_fire("other-rule", 30)
    );

    // 5. Multi-rulebook config
    println!("\n--- Multi-Rulebook Config ---");
    let config_yaml = r#"
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
  - name: deploy-notify
    events:
      - type: manual
    actions:
      - notify:
          channel: "https://hooks.slack.com/abc"
          message: "Deployment triggered"
  - name: cleanup-cron
    events:
      - type: cron_fired
    actions:
      - script: "forjar apply -f cleanup.yaml"
"#;
    let config: RulebookConfig = serde_yaml_ng::from_str(config_yaml).unwrap();
    println!("Loaded {} rulebooks:", config.rulebooks.len());
    for rb in &config.rulebooks {
        println!(
            "  {} — {} event(s), {} action(s) [{}]",
            rb.name,
            rb.events.len(),
            rb.actions.len(),
            rb.actions[0].action_type()
        );
    }

    // 6. All event types
    println!("\n--- Event Types ---");
    for et in [
        EventType::FileChanged,
        EventType::ProcessExit,
        EventType::CronFired,
        EventType::WebhookReceived,
        EventType::MetricThreshold,
        EventType::Manual,
    ] {
        println!("  {et}");
    }
}
