//! FJ-3500/3103/3106: Environments, cron parsing, rules runtime.
//!
//! Demonstrates:
//! - Environment param/machine resolution and diffing
//! - Promotion gate classification
//! - Cron schedule parsing, matching, and summaries
//! - Rules runtime event evaluation and action dispatch
//!
//! Usage: cargo run --example env_cron_rules

use forjar::core::cron_source::{matches, parse_cron, schedule_summary, CronTime};
use forjar::core::rules_runtime::{
    evaluate_event, fired_actions, matching_rulebooks, runtime_summary,
};
use forjar::core::types::environment::*;
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

fn main() {
    println!("Forjar: Environments, Cron & Rules");
    println!("{}", "=".repeat(50));

    // ── Environment Resolution ──
    println!("\n[FJ-3500] Environment Resolution:");
    let mut base_params = HashMap::new();
    base_params.insert(
        "region".into(),
        serde_yaml_ng::Value::String("us-east-1".into()),
    );
    base_params.insert("tier".into(), serde_yaml_ng::Value::String("dev".into()));

    let mut base_machines = IndexMap::new();
    base_machines.insert("web1".into(), Machine::ssh("web1", "10.0.0.1", "deploy"));
    base_machines.insert("db1".into(), Machine::ssh("db1", "10.0.0.2", "deploy"));

    let mut staging_params = HashMap::new();
    staging_params.insert(
        "tier".into(),
        serde_yaml_ng::Value::String("staging".into()),
    );
    let mut staging_machines = IndexMap::new();
    staging_machines.insert(
        "web1".into(),
        MachineOverride {
            addr: "10.1.0.1".into(),
        },
    );

    let staging = Environment {
        description: Some("Staging".into()),
        params: staging_params,
        machines: staging_machines,
        promotion: None,
    };

    let resolved_params = resolve_env_params(&base_params, &staging);
    for (k, v) in &resolved_params {
        println!("  param {k}: {v:?}");
    }
    let resolved_machines = resolve_env_machines(&base_machines, &staging);
    for (name, m) in &resolved_machines {
        println!("  machine {name}: {}", m.addr);
    }

    // ── Promotion Gates ──
    println!("\n  Gate types:");
    let gates = vec![
        PromotionGate {
            validate: Some(ValidateGateOptions {
                deep: true,
                exhaustive: false,
            }),
            ..Default::default()
        },
        PromotionGate {
            coverage: Some(CoverageGateOptions { min: 90 }),
            ..Default::default()
        },
        PromotionGate {
            script: Some("curl -sf http://localhost/health".into()),
            ..Default::default()
        },
    ];
    for g in &gates {
        println!("    {}", g.gate_type());
    }

    // ── Environment Diff ──
    println!("\n  Diff staging vs prod:");
    let mut prod_params = HashMap::new();
    prod_params.insert(
        "tier".into(),
        serde_yaml_ng::Value::String("production".into()),
    );
    let mut prod_machines = IndexMap::new();
    prod_machines.insert(
        "web1".into(),
        MachineOverride {
            addr: "10.2.0.1".into(),
        },
    );
    prod_machines.insert(
        "db1".into(),
        MachineOverride {
            addr: "10.2.0.2".into(),
        },
    );
    let prod = Environment {
        description: Some("Production".into()),
        params: prod_params,
        machines: prod_machines,
        promotion: None,
    };
    let diff = diff_environments(
        "staging",
        &staging,
        "prod",
        &prod,
        &base_params,
        &base_machines,
    );
    println!("    {} total diffs", diff.total_diffs());
    for pd in &diff.param_diffs {
        println!(
            "    param {}: {:?} → {:?}",
            pd.key, pd.source_value, pd.target_value
        );
    }
    for md in &diff.machine_diffs {
        println!(
            "    machine {}: {:?} → {:?}",
            md.machine, md.source_addr, md.target_addr
        );
    }

    // ── Cron Source ──
    println!("\n[FJ-3103] Cron Parsing:");
    let schedules = [
        ("* * * * *", "every minute"),
        ("0 */6 * * *", "every 6 hours"),
        ("30 9 * * 1-5", "9:30 weekdays"),
        ("0 0 1 * *", "monthly"),
    ];
    for (expr, label) in &schedules {
        let sched = parse_cron(expr).unwrap();
        let time = CronTime {
            minute: 30,
            hour: 9,
            day: 1,
            month: 3,
            weekday: 1,
        };
        let matched = matches(&sched, &time);
        println!(
            "  {label:20} {expr:20} match@09:30={matched:5} summary={}",
            schedule_summary(&sched)
        );
    }

    // ── Rules Runtime ──
    println!("\n[FJ-3106] Rules Runtime:");
    let config = RulebookConfig {
        rulebooks: vec![
            Rulebook {
                name: "config-repair".into(),
                description: Some("Auto-repair on config change".into()),
                events: vec![EventPattern {
                    event_type: EventType::FileChanged,
                    match_fields: {
                        let mut m = HashMap::new();
                        m.insert("path".into(), "/etc/nginx/nginx.conf".into());
                        m
                    },
                }],
                conditions: vec![],
                actions: vec![RulebookAction {
                    apply: Some(ApplyAction {
                        file: "forjar.yaml".into(),
                        subset: vec![],
                        tags: vec!["config".into()],
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
                name: "cron-alert".into(),
                description: None,
                events: vec![EventPattern {
                    event_type: EventType::CronFired,
                    match_fields: HashMap::new(),
                }],
                conditions: vec![],
                actions: vec![RulebookAction {
                    apply: None,
                    destroy: None,
                    script: None,
                    notify: Some(NotifyAction {
                        channel: "slack://ops".into(),
                        message: "Cron triggered".into(),
                    }),
                }],
                cooldown_secs: 60,
                max_retries: 1,
                enabled: true,
            },
        ],
    };

    let mut tracker = CooldownTracker::default();
    let summary = runtime_summary(&config, &tracker);
    println!(
        "  Rulebooks: {} total, {} enabled",
        summary.total_rulebooks, summary.enabled
    );

    let event = InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web1".into()),
        payload: {
            let mut m = HashMap::new();
            m.insert("path".into(), "/etc/nginx/nginx.conf".into());
            m
        },
    };

    let matched = matching_rulebooks(&event, &config);
    println!(
        "  Matched: {:?}",
        matched.iter().map(|r| &r.name).collect::<Vec<_>>()
    );

    let actions = fired_actions(&event, &config, &mut tracker);
    for (name, acts) in &actions {
        for a in acts {
            println!("  Fire: {name} → {}", a.action_type());
        }
    }

    let results = evaluate_event(&event, &config, &mut tracker);
    for r in &results {
        println!(
            "  Eval: {} (cooldown_blocked={})",
            r.rulebook, r.cooldown_blocked
        );
    }

    println!("\n{}", "=".repeat(50));
    println!("All env/cron/rules criteria survived.");
}
