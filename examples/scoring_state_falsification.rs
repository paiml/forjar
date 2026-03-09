//! FJ-2800/013/005: Scoring, state management, and codegen falsification.
//!
//! Demonstrates Popperian rejection criteria for:
//! - ForjarScore v2 static + runtime scoring
//! - State lock file save/load roundtrip with BLAKE3 integrity
//! - Codegen dispatch (check/apply/state_query scripts)
//! - Process locking
//! - CIS Ubuntu 22.04 compliance pack
//!
//! Usage: cargo run --example scoring_state_falsification

use forjar::core::cis_ubuntu_pack::{cis_ubuntu_2204_pack, severity_summary};
use forjar::core::codegen::{apply_script, check_script, state_query_script};
use forjar::core::scoring::{compute, format_score_report, score_bar, RuntimeData, ScoringInput};
use forjar::core::state;
use forjar::core::state::integrity;
use forjar::core::types::{ForjarConfig, Resource, ResourceType};

fn main() {
    println!("Forjar Scoring / State / Codegen Falsification");
    println!("{}", "=".repeat(60));

    // ── ForjarScore v2: static-only scoring ──
    println!("\n[FJ-2800] ForjarScore v2:");

    let config = ForjarConfig {
        version: "1.0".into(),
        name: "test-recipe".into(),
        ..Default::default()
    };
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    let static_ok = result.static_composite > 0 && result.runtime_grade.is_none();
    println!(
        "  Static-only scoring works: {} {}",
        if static_ok { "yes" } else { "no" },
        if static_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(static_ok);
    println!("  Static grade: {}", result.static_grade);
    println!("  Score bar: {}", score_bar(result.static_composite));

    // With runtime data
    let input_rt = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 5000,
        runtime: Some(RuntimeData {
            validate_pass: true,
            plan_pass: true,
            first_apply_pass: true,
            second_apply_pass: true,
            zero_changes_on_reapply: true,
            hash_stable: true,
            all_resources_converged: true,
            state_lock_written: true,
            warning_count: 0,
            changed_on_reapply: 0,
            first_apply_ms: 100,
            second_apply_ms: 50,
        }),
        raw_yaml: Some(String::new()),
    };
    let result_rt = compute(&config, &input_rt);
    let rt_ok = result_rt.runtime_grade.is_some();
    println!(
        "  Runtime grade present: {} {}",
        if rt_ok { "yes" } else { "no" },
        if rt_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(rt_ok);
    println!("{}", format_score_report(&result_rt));

    // ── State lock roundtrip ──
    println!("[FJ-013] State Lock Roundtrip:");

    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let lock = state::new_lock("web", "web-01");
    state::save_lock(&state_dir, &lock).unwrap();
    let loaded = state::load_lock(&state_dir, "web").unwrap();
    let lock_ok = loaded.is_some() && loaded.as_ref().unwrap().machine == "web";
    println!(
        "  Save/load roundtrip: {} {}",
        if lock_ok { "pass" } else { "fail" },
        if lock_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(lock_ok);

    // BLAKE3 integrity
    let integrity_results = integrity::verify_state_integrity(&state_dir);
    let int_ok = !integrity::has_errors(&integrity_results);
    println!(
        "  BLAKE3 integrity: {} {}",
        if int_ok { "pass" } else { "fail" },
        if int_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(int_ok);

    // Process locking
    state::acquire_process_lock(&state_dir).unwrap();
    state::release_process_lock(&state_dir);
    let pl_ok = !state_dir.join(".forjar.lock").exists();
    println!(
        "  Process lock acquire/release: {} {}",
        if pl_ok { "pass" } else { "fail" },
        if pl_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(pl_ok);

    // ── Codegen dispatch ──
    println!("\n[FJ-005] Codegen Dispatch:");

    let mut pkg = Resource::default();
    pkg.resource_type = ResourceType::Package;
    pkg.packages = vec!["nginx".into()];

    let check = check_script(&pkg).unwrap();
    let apply = apply_script(&pkg).unwrap();
    let query = state_query_script(&pkg).unwrap();
    let cg_ok = !check.is_empty() && !apply.is_empty() && !query.is_empty();
    println!(
        "  Package scripts generated: {} {}",
        if cg_ok { "yes" } else { "no" },
        if cg_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(cg_ok);

    let mut recipe = Resource::default();
    recipe.resource_type = ResourceType::Recipe;
    let recipe_ok = check_script(&recipe).is_err();
    println!(
        "  Recipe type rejects codegen: {} {}",
        if recipe_ok { "yes" } else { "no" },
        if recipe_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(recipe_ok);

    // ── CIS Ubuntu Pack ──
    println!("\n[FJ-3206] CIS Ubuntu 22.04 Pack:");

    let pack = cis_ubuntu_2204_pack();
    let (errors, warnings, info) = severity_summary(&pack);
    let cis_ok = pack.rules.len() == 24 && errors >= 12 && warnings >= 8 && info >= 1;
    println!(
        "  24 rules, severity distribution: {} {}",
        if cis_ok { "correct" } else { "wrong" },
        if cis_ok { "✓" } else { "✗ FALSIFIED" }
    );
    println!(
        "  Errors: {}, Warnings: {}, Info: {}",
        errors, warnings, info
    );
    assert!(cis_ok);

    println!("\n{}", "=".repeat(60));
    println!("All scoring/state/codegen criteria survived.");
}
