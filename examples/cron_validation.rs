//! FJ-2501: Cron schedule validation example.
//!
//! Demonstrates format validation of cron schedule fields,
//! including range checking, special keywords, and error reporting.
//!
//! ```bash
//! cargo run --example cron_validation
//! ```

use forjar::core::parser::{parse_config, validate_config};

fn main() {
    demo_valid_cron();
    demo_invalid_cron();
    demo_deny_paths();
}

fn demo_valid_cron() {
    println!("=== FJ-2501: Cron Schedule Validation ===\n");

    let valid = r#"
version: "1.0"
name: cron-demo
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  daily-backup:
    type: cron
    machine: m
    name: daily-backup
    command: /usr/bin/backup --full
    schedule: "0 2 * * 1-5"
  hourly-check:
    type: cron
    machine: m
    name: hourly-check
    command: /usr/bin/healthcheck
    schedule: "*/15 * * * *"
  weekly-report:
    type: cron
    machine: m
    name: weekly-report
    command: /usr/bin/report
    schedule: "@weekly"
"#;

    let config = parse_config(valid).expect("valid YAML");
    let errors = validate_config(&config);
    println!("Valid schedules:");
    println!("  '0 2 * * 1-5'   (weekdays at 2am)");
    println!("  '*/15 * * * *'  (every 15 minutes)");
    println!("  '@weekly'       (cron keyword)");
    println!("  Errors: {} (expected 0)\n", errors.len());
}

fn demo_invalid_cron() {
    let invalid = r#"
version: "1.0"
name: bad-cron
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  bad-job:
    type: cron
    machine: m
    name: bad-job
    command: /usr/bin/bad
    schedule: "99 25 32 13 8"
"#;

    let config = parse_config(invalid).expect("valid YAML");
    let errors = validate_config(&config);
    println!("Invalid schedule '99 25 32 13 8':");
    for e in &errors {
        println!("  {e}");
    }
}

fn demo_deny_paths() {
    println!("\n=== FJ-2300: Deny Paths Policy ===\n");

    let deny = r#"
version: "1.0"
name: deny-demo
machines:
  m:
    hostname: m
    addr: 127.0.0.1
policy:
  deny_paths:
    - /etc/shadow
    - /root/**
resources:
  allowed:
    type: file
    machine: m
    path: /opt/app/config.yaml
    content: "ok"
  forbidden:
    type: file
    machine: m
    path: /etc/shadow
    content: "nope"
"#;

    let config = parse_config(deny).expect("valid YAML");
    let errors = validate_config(&config);
    println!("deny_paths: [\"/etc/shadow\", \"/root/**\"]");
    for e in &errors {
        println!("  {e}");
    }
}
