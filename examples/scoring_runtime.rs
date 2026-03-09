//! FJ-2800/2803: Runtime scoring dimensions (COR, IDM, PRF).
//!
//! Demonstrates:
//! - Correctness scoring breakdown (validate, plan, apply, converge, lock)
//! - Idempotency scoring (second apply, zero changes, hash stable, class)
//! - Performance scoring (budget points, idempotent points, efficiency)
//! - Two-tier grade composition (static/runtime)
//!
//! Usage: cargo run --example scoring_runtime

use forjar::core::scoring::{compute, format_score_report, RuntimeData, ScoringInput};
use forjar::core::types::ForjarConfig;

fn main() {
    println!("Forjar: Runtime Scoring Dimensions");
    println!("{}", "=".repeat(50));

    let config = ForjarConfig {
        version: "1.0".into(),
        name: "test-recipe".into(),
        ..Default::default()
    };

    // ── Perfect Runtime ──
    println!("\n[FJ-2800] Perfect Runtime:");
    let perfect = RuntimeData {
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
        first_apply_ms: 1000,
        second_apply_ms: 50,
    };
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 5000,
        runtime: Some(perfect.clone()),
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    println!("{}", format_score_report(&result));
    assert!(result.runtime_grade.is_some());

    // ── Degraded Runtime ──
    println!("[FJ-2803] Degraded Runtime (warnings, slow):");
    let degraded = RuntimeData {
        validate_pass: true,
        plan_pass: true,
        first_apply_pass: true,
        second_apply_pass: true,
        zero_changes_on_reapply: false,
        hash_stable: true,
        all_resources_converged: true,
        state_lock_written: true,
        warning_count: 3,
        changed_on_reapply: 2,
        first_apply_ms: 8000,
        second_apply_ms: 6000,
    };
    let input2 = ScoringInput {
        status: "qualified".into(),
        idempotency: "weak".into(),
        budget_ms: 5000,
        runtime: Some(degraded),
        raw_yaml: Some(String::new()),
    };
    let result2 = compute(&config, &input2);
    println!("{}", format_score_report(&result2));

    // ── Static-Only (Pending) ──
    println!("[FJ-2801] Static-Only (no runtime data):");
    let input3 = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result3 = compute(&config, &input3);
    assert!(result3.grade.ends_with("/pending"));
    println!("  Grade: {}", result3.grade);

    println!("\n{}", "=".repeat(50));
    println!("All scoring runtime criteria survived.");
}
