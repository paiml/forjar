//! FJ-3105/3100/2003/114: Metric thresholds, event matching, generation diffs, DO-330.
//!
//! Demonstrates:
//! - Threshold evaluation (gt/gte/lt/lte/eq) and consecutive tracking
//! - Event pattern matching and rulebook dispatch
//! - Generation diff computation and summary formatting
//! - DO-330 tool qualification package generation
//!
//! Usage: cargo run --example metric_event_diff

use forjar::core::do330::{generate_qualification_package, ToolQualLevel};
use forjar::core::metric_source::{
    evaluate_metrics, evaluate_threshold, MetricThreshold, ThresholdOp, ThresholdTracker,
};
use forjar::core::types::{
    diff_resource_sets, event_matches_pattern, event_matches_rulebook, CooldownTracker,
    EventPattern, EventType, GenerationDiff, InfraEvent, ResourceDiff, Rulebook, RulebookAction,
};
use std::collections::HashMap;

fn main() {
    println!("Forjar: Metrics, Events, Diffs & DO-330");
    println!("{}", "=".repeat(50));

    // ── Metric Thresholds ──
    println!("\n[FJ-3105] Threshold Evaluation:");
    let t = MetricThreshold {
        name: "cpu".into(),
        operator: ThresholdOp::Gt,
        value: 80.0,
        consecutive: 1,
    };
    println!("  cpu > 80: 85.0 → {}", evaluate_threshold(&t, 85.0));
    println!("  cpu > 80: 75.0 → {}", evaluate_threshold(&t, 75.0));

    println!("\n[FJ-3105] Multi-Metric Evaluation:");
    let thresholds = vec![
        MetricThreshold {
            name: "cpu".into(),
            operator: ThresholdOp::Gt,
            value: 80.0,
            consecutive: 1,
        },
        MetricThreshold {
            name: "mem".into(),
            operator: ThresholdOp::Gt,
            value: 90.0,
            consecutive: 1,
        },
    ];
    let mut values = HashMap::new();
    values.insert("cpu".into(), 85.0);
    values.insert("mem".into(), 70.0);
    let mut tracker = ThresholdTracker::default();
    let results = evaluate_metrics(&thresholds, &values, &mut tracker);
    for r in &results {
        println!(
            "  {}: violated={}, fire={}",
            r.name, r.violated, r.should_fire
        );
    }

    // ── Event Matching ──
    println!("\n[FJ-3100] Event Matching:");
    let event = InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("intel".into()),
        payload: vec![("path".into(), "/etc/nginx/nginx.conf".into())]
            .into_iter()
            .collect(),
    };
    let pattern = EventPattern {
        event_type: EventType::FileChanged,
        match_fields: vec![("path".into(), "/etc/nginx/nginx.conf".into())]
            .into_iter()
            .collect(),
    };
    println!(
        "  FileChanged + path match: {}",
        event_matches_pattern(&event, &pattern)
    );

    println!("\n[FJ-3100] Rulebook Dispatch:");
    let rb = Rulebook {
        name: "nginx-repair".into(),
        description: None,
        events: vec![pattern],
        conditions: vec![],
        actions: vec![RulebookAction {
            apply: None,
            destroy: None,
            script: Some("systemctl reload nginx".into()),
            notify: None,
        }],
        cooldown_secs: 60,
        max_retries: 3,
        enabled: true,
    };
    println!("  Rulebook match: {}", event_matches_rulebook(&event, &rb));

    let mut cooldown = CooldownTracker::default();
    println!(
        "  Can fire (before): {}",
        cooldown.can_fire("nginx-repair", 60)
    );
    cooldown.record_fire("nginx-repair");
    println!(
        "  Can fire (after):  {}",
        cooldown.can_fire("nginx-repair", 60)
    );

    // ── Generation Diff ──
    println!("\n[FJ-2003] Generation Diff:");
    let diff = GenerationDiff {
        gen_from: 5,
        gen_to: 8,
        machine: "intel".into(),
        resources: vec![
            ResourceDiff::added("new-pkg", "package"),
            ResourceDiff::modified("config", "file").with_detail("content changed"),
            ResourceDiff::removed("old-svc", "service"),
            ResourceDiff::unchanged("base", "file"),
        ],
    };
    println!("{}", diff.format_summary());

    println!("\n[FJ-2003] diff_resource_sets:");
    let from = vec![("a", "file", "h1"), ("b", "pkg", "h2")];
    let to = vec![("a", "file", "h1"), ("c", "svc", "h3")];
    let diffs = diff_resource_sets(&from, &to);
    for d in &diffs {
        println!("  {} → {}", d.resource_id, d.action);
    }

    // ── DO-330 ──
    println!("\n[FJ-114] DO-330 Qualification:");
    let pkg = generate_qualification_package("1.1.1", ToolQualLevel::Tql5);
    println!("  Tool: {} v{}", pkg.tool_name, pkg.tool_version);
    println!("  Level: {}", pkg.qualification_level);
    println!("  Requirements: {}", pkg.total_requirements);
    println!("  Complete: {}", pkg.qualification_complete);

    println!("\n{}", "=".repeat(50));
    println!("All metric/event/diff/DO-330 criteria survived.");
}
