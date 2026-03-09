//! FJ-1387/3108/3106/044: Compliance, rulebook engine, runtime, migration.
//!
//! Demonstrates:
//! - CIS/NIST/SOC2/HIPAA compliance benchmarks
//! - Compliance pack parsing and evaluation
//! - Rulebook validation and event-driven runtime
//! - Docker → pepita migration
//!
//! Usage: cargo run --example compliance_rules_migrate

use forjar::core::compliance::{count_by_severity, evaluate_benchmark, supported_benchmarks};
use forjar::core::compliance_pack::{evaluate_pack, parse_pack};
use forjar::core::mcdc::{build_decision, generate_mcdc_and, generate_mcdc_or};
use forjar::core::migrate::docker_to_pepita;
use forjar::core::policy_coverage::{compute_coverage, format_coverage};
use forjar::core::rules_engine::validate_rulebook_yaml;
use forjar::core::rules_runtime::{evaluate_event, matching_rulebooks, runtime_summary};
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

fn main() {
    println!("Forjar: Compliance, Rules, Migration & MC/DC");
    println!("{}", "=".repeat(50));

    // ── FJ-1387: Compliance Benchmarks ──
    println!("\n[FJ-1387] Compliance Benchmarks:");
    println!("  Supported: {:?}", supported_benchmarks());

    let mut resources = IndexMap::new();
    resources.insert(
        "web-config".into(),
        Resource {
            resource_type: ResourceType::File,
            mode: Some("0644".into()),
            path: Some("/etc/nginx/nginx.conf".into()),
            ..Default::default()
        },
    );
    resources.insert(
        "db-svc".into(),
        Resource {
            resource_type: ResourceType::Service,
            owner: Some("root".into()),
            ..Default::default()
        },
    );
    let config = ForjarConfig {
        name: "demo".into(),
        resources,
        ..Default::default()
    };

    for benchmark in supported_benchmarks() {
        let findings = evaluate_benchmark(benchmark, &config);
        let (c, h, m, l) = count_by_severity(&findings);
        println!(
            "  {benchmark:12}: {} findings (C:{c} H:{h} M:{m} L:{l})",
            findings.len()
        );
    }

    // ── FJ-3205: Compliance Pack ──
    println!("\n[FJ-3205] Compliance Pack Evaluation:");
    let pack = parse_pack(
        r#"
name: security-baseline
version: "1.0"
framework: CIS
rules:
  - id: SEC-001
    title: Root ownership required
    type: assert
    resource_type: file
    field: owner
    expected: root
  - id: SEC-002
    title: No world-writable modes
    type: deny
    resource_type: file
    field: mode
    pattern: "777"
"#,
    )
    .unwrap();
    let mut pack_resources = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), "file".into());
    fields.insert("owner".into(), "www-data".into());
    fields.insert("mode".into(), "0644".into());
    pack_resources.insert("web-config".into(), fields);

    let result = evaluate_pack(&pack, &pack_resources);
    println!(
        "  Pack '{}': {}/{} passed ({:.0}%)",
        result.pack_name,
        result.passed_count(),
        result.results.len(),
        result.pass_rate()
    );

    // ── FJ-3208: Policy Coverage ──
    println!("\n[FJ-3208] Policy Coverage:");
    let cov = compute_coverage(&config);
    println!("{}", format_coverage(&cov));

    // ── FJ-3108: Rulebook Validation ──
    println!("\n[FJ-3108] Rulebook Validation:");
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
  - name: health-alert
    events:
      - type: metric_threshold
    actions:
      - notify:
          channel: "https://hooks.slack.com/xxx"
          message: "CPU threshold exceeded"
    cooldown_secs: 300
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    println!("  Validated 2 rulebooks: {} issues", issues.len());

    // ── FJ-3106: Runtime Evaluation ──
    println!("\n[FJ-3106] Runtime Event Evaluation:");
    let rb_config: RulebookConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let event = InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web-01".into()),
        payload: {
            let mut p = HashMap::new();
            p.insert("path".into(), "/etc/nginx/nginx.conf".into());
            p
        },
    };
    let matched = matching_rulebooks(&event, &rb_config);
    println!("  Matching rulebooks for file_changed: {}", matched.len());

    let mut tracker = CooldownTracker::default();
    let results = evaluate_event(&event, &rb_config, &mut tracker);
    for r in &results {
        println!(
            "  {} → {} actions (blocked: {}, disabled: {})",
            r.rulebook,
            r.actions.len(),
            r.cooldown_blocked,
            r.disabled
        );
    }
    let summary = runtime_summary(&rb_config, &tracker);
    println!(
        "  Summary: {} total, {} enabled, {} in cooldown",
        summary.total_rulebooks, summary.enabled, summary.in_cooldown
    );

    // ── FJ-044: Docker → Pepita Migration ──
    println!("\n[FJ-044] Docker → Pepita Migration:");
    let docker = Resource {
        resource_type: ResourceType::Docker,
        image: Some("nginx:latest".into()),
        state: Some("running".into()),
        ports: vec!["8080:80".into()],
        environment: vec!["NODE_ENV=production".into()],
        restart: Some("unless-stopped".into()),
        ..Default::default()
    };
    let migration = docker_to_pepita("web-container", &docker);
    println!(
        "  Type: {:?} → {:?}",
        ResourceType::Docker,
        migration.resource.resource_type
    );
    println!(
        "  State: running → {}",
        migration.resource.state.as_deref().unwrap_or("?")
    );
    println!("  netns: {} (from ports)", migration.resource.netns);
    println!("  Warnings:");
    for w in &migration.warnings {
        println!("    - {w}");
    }

    // ── FJ-051: MC/DC Analysis ──
    println!("\n[FJ-051] MC/DC Test Generation:");
    let and_decision = build_decision(
        "file_exists && hash_matches && owner_correct",
        &["file_exists", "hash_matches", "owner_correct"],
    );
    let and_report = generate_mcdc_and(&and_decision);
    println!(
        "  AND decision: {} conditions, {} pairs, min tests: {}",
        and_report.num_conditions,
        and_report.pairs.len(),
        and_report.min_tests_needed
    );

    let or_decision = build_decision("drift || timeout", &["drift", "timeout"]);
    let or_report = generate_mcdc_or(&or_decision);
    println!(
        "  OR  decision: {} conditions, {} pairs, min tests: {}",
        or_report.num_conditions,
        or_report.pairs.len(),
        or_report.min_tests_needed
    );

    println!("\n{}", "=".repeat(50));
    println!("All compliance/rules/migration/mcdc criteria survived.");
}
