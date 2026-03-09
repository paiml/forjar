//! FJ-3108: Rulebook validation example.
//!
//! Demonstrates validating rulebook YAML files for correctness:
//! event patterns, action completeness, cooldown bounds, and
//! event type coverage analysis.

use forjar::core::rules_engine::*;
use forjar::core::types::RulebookConfig;

fn main() {
    println!("=== FJ-3108: Rulebook Validation ===\n");

    // 1. Valid rulebook
    println!("--- Valid Rulebook ---");
    let valid = r#"
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
  - name: crash-recovery
    events:
      - type: process_exit
        match:
          process: myapp
          exit_code: "137"
    actions:
      - script: "systemctl restart myapp"
      - notify:
          channel: "https://hooks.slack.com/services/xxx"
          message: "myapp crashed on {{machine}}"
    cooldown_secs: 300
    max_retries: 3
"#;

    let issues = validate_rulebook_yaml(valid).unwrap();
    let summary = ValidationSummary::new(2, issues);
    println!(
        "  Rulebooks: {}, Errors: {}, Warnings: {}, Passed: {}",
        summary.rulebook_count,
        summary.error_count(),
        summary.warning_count(),
        summary.passed()
    );

    // 2. Rulebook with issues
    println!("\n--- Rulebook with Issues ---");
    let bad = r#"
rulebooks:
  - name: empty-events
    events: []
    actions:
      - script: "echo ok"
  - name: empty-events
    events:
      - type: manual
    actions: []
  - name: rapid-fire
    events:
      - type: cron_fired
    actions:
      - apply:
          file: ""
    cooldown_secs: 0
    max_retries: 50
"#;

    let issues = validate_rulebook_yaml(bad).unwrap();
    let summary = ValidationSummary::new(3, issues);
    println!(
        "  Rulebooks: {}, Errors: {}, Warnings: {}",
        summary.rulebook_count,
        summary.error_count(),
        summary.warning_count()
    );
    for issue in &summary.issues {
        println!(
            "  [{}] {}: {}",
            issue.severity, issue.rulebook, issue.message
        );
    }
    println!("  Passed: {}", summary.passed());

    // 3. Event type coverage
    println!("\n--- Event Type Coverage ---");
    let config: RulebookConfig = serde_yaml_ng::from_str(valid).unwrap();
    let coverage = event_type_coverage(&config);
    for (et, count) in &coverage {
        let bar = if *count > 0 { "+" } else { "-" };
        println!("  [{bar}] {et}: {count} rulebook(s)");
    }

    println!("\n--- Summary ---");
    println!("Validates: event patterns, action types, cooldown, retries");
    println!("Coverage: shows which event types are handled");
}
