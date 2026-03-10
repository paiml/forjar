//! FJ-045/046/1280/3107: SAT solver, minimal changeset, state reconstruction,
//! and rulebook event log falsification.
//!
//! Demonstrates Popperian rejection criteria for:
//! - SAT-based dependency verification (satisfiable/unsatisfiable)
//! - Minimal changeset computation with dependency propagation
//! - Event-sourced state reconstruction from JSONL
//! - Rulebook event log append/read cycle
//!
//! Usage: cargo run --example sat_changeset_falsification

use forjar::core::planner::minimal_changeset::{compute_minimal_changeset, verify_minimality};
use forjar::core::planner::sat_deps::{build_sat_problem, solve, SatProblem, SatResult};
use forjar::core::state::reconstruct;
use forjar::core::state::rulebook_log;
use forjar::core::types::*;
use std::collections::{BTreeMap, HashMap};

fn main() {
    println!("Forjar SAT / Changeset / Reconstruct / Rulebook Falsification");
    println!("{}", "=".repeat(60));

    // ── FJ-045: SAT solver ──
    println!("\n[FJ-045] SAT Dependency Solver:");

    // Linear chain: A → B → C
    let resources = vec!["A".into(), "B".into(), "C".into()];
    let deps = vec![("B".into(), "A".into()), ("C".into(), "B".into())];
    let problem = build_sat_problem(&resources, &deps);
    let result = solve(&problem);
    let sat_ok = matches!(result, SatResult::Satisfiable { .. });
    println!(
        "  Linear chain satisfiable: {} {}",
        if sat_ok { "yes" } else { "no" },
        if sat_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(sat_ok);

    if let SatResult::Satisfiable { assignment } = &result {
        println!(
            "  Assignment: {}",
            assignment
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Contradictory: A AND !A
    let mut var_names = BTreeMap::new();
    var_names.insert(1, "conflicting-pkg".into());
    let contra = SatProblem {
        num_vars: 1,
        clauses: vec![vec![1], vec![-1]],
        var_names,
    };
    let contra_result = solve(&contra);
    let unsat_ok = matches!(contra_result, SatResult::Unsatisfiable { .. });
    println!(
        "  Contradiction detected: {} {}",
        if unsat_ok { "yes" } else { "no" },
        if unsat_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(unsat_ok);

    // ── FJ-046: Minimal changeset ──
    println!("\n[FJ-046] Minimal Changeset:");

    let resources = vec![
        ("nginx".into(), "web".into(), "h-nginx-new".into()),
        ("mysql".into(), "db".into(), "h-mysql".into()),
        ("app".into(), "web".into(), "h-app".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("nginx@web".into(), "h-nginx-old".into());
    locks.insert("mysql@db".into(), "h-mysql".into());
    locks.insert("app@web".into(), "h-app".into());
    // app depends on nginx
    let deps = vec![("app".into(), "nginx".into())];

    let changeset = compute_minimal_changeset(&resources, &locks, &deps);
    let cs_ok = changeset.changes_needed == 2 && changeset.changes_skipped == 1;
    println!(
        "  Dependency propagation: {} {}",
        if cs_ok { "correct" } else { "wrong" },
        if cs_ok { "✓" } else { "✗ FALSIFIED" }
    );
    println!(
        "  Needed: {}, Skipped: {}, Total: {}",
        changeset.changes_needed, changeset.changes_skipped, changeset.total_resources
    );
    assert!(cs_ok);

    let min_ok = verify_minimality(&changeset);
    println!(
        "  Provably minimal: {} {}",
        if min_ok { "yes" } else { "no" },
        if min_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(min_ok);

    // ── FJ-1280: State reconstruction ──
    println!("\n[FJ-1280] State Reconstruction:");

    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    // Write event log
    let machine_dir = state_dir.join("web");
    std::fs::create_dir_all(&machine_dir).unwrap();
    let events = [
        serde_json::json!({
            "ts": "2026-03-09T10:00:00Z",
            "event": "resource_converged",
            "machine": "web",
            "resource": "nginx",
            "duration_seconds": 1.5,
            "hash": "h-nginx"
        }),
        serde_json::json!({
            "ts": "2026-03-09T10:01:00Z",
            "event": "resource_converged",
            "machine": "web",
            "resource": "app",
            "duration_seconds": 0.5,
            "hash": "h-app"
        }),
        serde_json::json!({
            "ts": "2026-03-09T12:00:00Z",
            "event": "drift_detected",
            "machine": "web",
            "resource": "nginx",
            "expected_hash": "h-nginx",
            "actual_hash": "h-nginx-drifted"
        }),
    ];
    let content: String = events.iter().map(|e| e.to_string() + "\n").collect();
    std::fs::write(machine_dir.join("events.jsonl"), content).unwrap();

    // Reconstruct before drift
    let lock_pre = reconstruct::reconstruct_at(&state_dir, "web", "2026-03-09T11:00:00Z").unwrap();
    let pre_ok = lock_pre.resources["nginx"].status == ResourceStatus::Converged
        && lock_pre.resources.len() == 2;
    println!(
        "  Pre-drift reconstruction: {} {}",
        if pre_ok { "correct" } else { "wrong" },
        if pre_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(pre_ok);

    // Reconstruct after drift
    let lock_post = reconstruct::reconstruct_at(&state_dir, "web", "2026-03-09T23:59:59Z").unwrap();
    let post_ok = lock_post.resources["nginx"].status == ResourceStatus::Drifted
        && lock_post.resources["nginx"].hash == "h-nginx-drifted";
    println!(
        "  Post-drift reconstruction: {} {}",
        if post_ok { "correct" } else { "wrong" },
        if post_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(post_ok);

    // ── FJ-3107: Rulebook event log ──
    println!("\n[FJ-3107] Rulebook Event Log:");

    let log_dir = tempfile::tempdir().unwrap();
    let event = InfraEvent {
        event_type: EventType::FileChanged,
        timestamp: "2026-03-09T12:00:00Z".into(),
        machine: Some("web-01".into()),
        payload: HashMap::new(),
    };
    let entry = rulebook_log::make_entry(&event, "config-repair", "apply", true, None);
    rulebook_log::append_entry(log_dir.path(), &entry).unwrap();

    let entry2 = rulebook_log::make_entry(
        &event,
        "notify-slack",
        "notify",
        false,
        Some("webhook timeout".into()),
    );
    rulebook_log::append_entry(log_dir.path(), &entry2).unwrap();

    let entries = rulebook_log::read_entries(log_dir.path()).unwrap();
    let log_ok = entries.len() == 2 && entries[0].success && !entries[1].success;
    println!(
        "  Append/read roundtrip: {} {}",
        if log_ok { "pass" } else { "fail" },
        if log_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(log_ok);
    println!(
        "  Entries: {} (success: {}, failed: {})",
        entries.len(),
        entries.iter().filter(|e| e.success).count(),
        entries.iter().filter(|e| !e.success).count()
    );

    println!("\n{}", "=".repeat(60));
    println!("All SAT/changeset/reconstruct/rulebook criteria survived.");
}
