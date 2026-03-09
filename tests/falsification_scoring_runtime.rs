//! ForjarScore v2: Runtime dimension scoring falsification.
//!
//! Popperian rejection criteria for:
//! - COR (Correctness): validate/plan/apply/converge/lock/warning scoring
//! - IDM (Idempotency): second apply/zero changes/hash stable/class/changed count
//! - PRF (Performance): budget points, idempotent points, efficiency points
//! - Grade boundary conditions: A/B/C/D/F thresholds
//! - Runtime grade composition (static/runtime grade string)
//! - Legacy composite blending
//!
//! Usage: cargo test --test falsification_scoring_runtime

use forjar::core::scoring::{compute, RuntimeData, ScoringInput};
use forjar::core::types::ForjarConfig;

// ============================================================================
// Helpers
// ============================================================================

fn empty_config() -> ForjarConfig {
    ForjarConfig {
        version: "1.0".into(),
        name: "test".into(),
        ..Default::default()
    }
}

fn runtime_input(runtime: RuntimeData) -> ScoringInput {
    ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 5000,
        runtime: Some(runtime),
        raw_yaml: Some(String::new()),
    }
}

fn perfect_runtime() -> RuntimeData {
    RuntimeData {
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
    }
}

fn zero_runtime() -> RuntimeData {
    RuntimeData {
        validate_pass: false,
        plan_pass: false,
        first_apply_pass: false,
        second_apply_pass: false,
        zero_changes_on_reapply: false,
        hash_stable: false,
        all_resources_converged: false,
        state_lock_written: false,
        warning_count: 0,
        changed_on_reapply: 0,
        first_apply_ms: 0,
        second_apply_ms: 0,
    }
}

fn find_dim<'a>(
    result: &'a forjar::core::scoring::ScoringResult,
    code: &str,
) -> &'a forjar::core::scoring::DimensionScore {
    result.dimensions.iter().find(|d| d.code == code).unwrap()
}

// ============================================================================
// COR — Correctness
// ============================================================================

#[test]
fn cor_perfect_runtime() {
    let config = empty_config();
    let input = runtime_input(perfect_runtime());
    let result = compute(&config, &input);
    let cor = find_dim(&result, "COR");
    // validate(15) + plan(15) + apply(40) + converged(15) + lock(10) - warnings(0) = 95
    assert_eq!(cor.score, 95);
}

#[test]
fn cor_zero_runtime() {
    let config = empty_config();
    let input = runtime_input(zero_runtime());
    let result = compute(&config, &input);
    let cor = find_dim(&result, "COR");
    assert_eq!(cor.score, 0);
}

#[test]
fn cor_only_validate_and_plan() {
    let config = empty_config();
    let mut rt = zero_runtime();
    rt.validate_pass = true;
    rt.plan_pass = true;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let cor = find_dim(&result, "COR");
    assert_eq!(cor.score, 30); // 15 + 15
}

#[test]
fn cor_warnings_deduct() {
    let config = empty_config();
    let mut rt = perfect_runtime();
    rt.warning_count = 5;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let cor = find_dim(&result, "COR");
    // 95 - (5*2) = 85
    assert_eq!(cor.score, 85);
}

#[test]
fn cor_warnings_capped_at_5() {
    let config = empty_config();
    let mut rt = perfect_runtime();
    rt.warning_count = 100;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let cor = find_dim(&result, "COR");
    // 95 - (5*2) = 85 (capped at 5 warnings)
    assert_eq!(cor.score, 85);
}

#[test]
fn cor_no_runtime_zero() {
    let config = empty_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 5000,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    let cor = find_dim(&result, "COR");
    assert_eq!(cor.score, 0);
}

// ============================================================================
// IDM — Idempotency
// ============================================================================

#[test]
fn idm_perfect_strong() {
    let config = empty_config();
    let input = runtime_input(perfect_runtime());
    let result = compute(&config, &input);
    let idm = find_dim(&result, "IDM");
    // second_apply(25) + zero_changes(25) + hash_stable(20) + strong(20) - changed(0) = 90
    assert_eq!(idm.score, 90);
}

#[test]
fn idm_weak_class() {
    let config = empty_config();
    let rt = perfect_runtime();
    let mut input = runtime_input(rt);
    input.idempotency = "weak".into();
    let result = compute(&config, &input);
    let idm = find_dim(&result, "IDM");
    // 25 + 25 + 20 + 10(weak) = 80
    assert_eq!(idm.score, 80);
}

#[test]
fn idm_eventual_class() {
    let config = empty_config();
    let rt = perfect_runtime();
    let mut input = runtime_input(rt);
    input.idempotency = "eventual".into();
    let result = compute(&config, &input);
    let idm = find_dim(&result, "IDM");
    // 25 + 25 + 20 + 0(eventual) = 70
    assert_eq!(idm.score, 70);
}

#[test]
fn idm_changed_on_reapply_deduction() {
    let config = empty_config();
    let mut rt = perfect_runtime();
    rt.changed_on_reapply = 3;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let idm = find_dim(&result, "IDM");
    // 90 - (3*10) = 60
    assert_eq!(idm.score, 60);
}

#[test]
fn idm_changed_on_reapply_capped() {
    let config = empty_config();
    let mut rt = perfect_runtime();
    rt.changed_on_reapply = 100;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let idm = find_dim(&result, "IDM");
    // 90 - (5*10) = 40 (capped at 5)
    assert_eq!(idm.score, 40);
}

#[test]
fn idm_zero_runtime() {
    let config = empty_config();
    let input = runtime_input(zero_runtime());
    let result = compute(&config, &input);
    let idm = find_dim(&result, "IDM");
    // strong class still gives 20 even with zero runtime
    assert_eq!(idm.score, 20);
}

// ============================================================================
// PRF — Performance
// ============================================================================

#[test]
fn prf_fast_within_budget() {
    let config = empty_config();
    let mut rt = perfect_runtime();
    rt.first_apply_ms = 1000; // 20% of 5000 budget
    rt.second_apply_ms = 50; // fast idempotent, ratio 5%
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let prf = find_dim(&result, "PRF");
    // budget: 1000/5000 = 20% → 50pts
    // idempotent: 50ms → 30pts
    // efficiency: 50*100/1000 = 5% → 20pts
    // total: 100 (capped)
    assert_eq!(prf.score, 100);
}

#[test]
fn prf_over_budget() {
    let config = empty_config();
    let mut rt = perfect_runtime();
    rt.first_apply_ms = 10000; // 200% of 5000 budget
    rt.second_apply_ms = 50;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let prf = find_dim(&result, "PRF");
    // budget: 10000/5000 = 200% → 0pts
    // idempotent: 50ms → 30pts
    // efficiency: 50*100/10000 = 0% → 20pts
    // total: 50
    assert_eq!(prf.score, 50);
}

#[test]
fn prf_no_budget_zero() {
    let config = empty_config();
    let rt = perfect_runtime();
    let mut input = runtime_input(rt);
    input.budget_ms = 0;
    let result = compute(&config, &input);
    let prf = find_dim(&result, "PRF");
    assert_eq!(prf.score, 0);
}

#[test]
fn prf_slow_idempotent() {
    let config = empty_config();
    let mut rt = perfect_runtime();
    rt.first_apply_ms = 2000;
    rt.second_apply_ms = 15000; // >10s idempotent
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let prf = find_dim(&result, "PRF");
    // budget: 2000/5000 = 40% → 50pts
    // idempotent: 15000ms → 0pts
    // efficiency: 15000*100/2000 = 750% → 0pts
    // total: 50
    assert_eq!(prf.score, 50);
}

#[test]
fn prf_budget_breakpoints() {
    let config = empty_config();
    // Test 51-75% bracket: should give 40pts
    let mut rt = perfect_runtime();
    rt.first_apply_ms = 3000; // 60% of 5000
    rt.second_apply_ms = 100;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let prf = find_dim(&result, "PRF");
    // budget: 3000/5000 = 60% → 40pts
    // idempotent: 100ms → 30pts
    // efficiency: 100*100/3000 = 3% → 20pts
    // total: 90
    assert_eq!(prf.score, 90);
}

#[test]
fn prf_budget_76_100_bracket() {
    let config = empty_config();
    let mut rt = perfect_runtime();
    rt.first_apply_ms = 4500; // 90% of 5000
    rt.second_apply_ms = 100;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let prf = find_dim(&result, "PRF");
    // budget: 4500/5000 = 90% → 30pts
    // idempotent: 100ms → 30pts
    // efficiency: 100*100/4500 = 2% → 20pts
    // total: 80
    assert_eq!(prf.score, 80);
}

#[test]
fn prf_budget_101_150_bracket() {
    let config = empty_config();
    let mut rt = perfect_runtime();
    rt.first_apply_ms = 6000; // 120% of 5000
    rt.second_apply_ms = 100;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let prf = find_dim(&result, "PRF");
    // budget: 6000/5000 = 120% → 15pts
    // idempotent: 100ms → 30pts
    // efficiency: 100*100/6000 = 1% → 20pts
    // total: 65
    assert_eq!(prf.score, 65);
}

#[test]
fn prf_idempotent_breakpoints() {
    let config = empty_config();
    // 2001-5000ms bracket
    let mut rt = perfect_runtime();
    rt.first_apply_ms = 1000;
    rt.second_apply_ms = 3000;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let prf = find_dim(&result, "PRF");
    // budget: 1000/5000 = 20% → 50pts
    // idempotent: 3000ms → 25pts
    // efficiency: 3000*100/1000 = 300% → 0pts
    // total: 75
    assert_eq!(prf.score, 75);
}

#[test]
fn prf_idempotent_5001_10000() {
    let config = empty_config();
    let mut rt = perfect_runtime();
    rt.first_apply_ms = 1000;
    rt.second_apply_ms = 7000;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    let prf = find_dim(&result, "PRF");
    // budget: 50, idempotent: 15, efficiency: 0
    assert_eq!(prf.score, 65);
}

// ============================================================================
// Grade boundaries
// ============================================================================

#[test]
fn grade_a_requires_90_and_min_80() {
    let config = empty_config();
    let mut rt = perfect_runtime();
    rt.first_apply_ms = 1000;
    rt.second_apply_ms = 50;
    let input = runtime_input(rt);
    let result = compute(&config, &input);
    // With empty config, static dims are low → static grade is F/D
    // But runtime with perfect data: COR=95, IDM=90, PRF=100 → composite ~94, min=90 → A
    assert!(result.runtime_grade.is_some());
    let rg = result.runtime_grade.unwrap();
    assert_eq!(rg, 'A');
}

#[test]
fn grade_format_static_runtime() {
    let config = empty_config();
    let input = runtime_input(perfect_runtime());
    let result = compute(&config, &input);
    // Grade format is "static/runtime"
    assert!(result.grade.contains('/'));
    let parts: Vec<&str> = result.grade.split('/').collect();
    assert_eq!(parts.len(), 2);
}

#[test]
fn grade_pending_without_runtime() {
    let config = empty_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    assert!(result.grade.ends_with("/pending"));
}

#[test]
fn grade_blocked_status() {
    let config = empty_config();
    let input = ScoringInput {
        status: "blocked".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    assert!(result.grade.ends_with("/blocked"));
    assert!(result.hard_fail);
    assert!(result
        .hard_fail_reason
        .as_ref()
        .unwrap()
        .contains("blocked"));
}

// ============================================================================
// Dimension weights (v2)
// ============================================================================

#[test]
fn cor_weight_is_35_pct() {
    let config = empty_config();
    let input = runtime_input(perfect_runtime());
    let result = compute(&config, &input);
    let cor = find_dim(&result, "COR");
    assert!((cor.weight - 0.35).abs() < 0.001);
}

#[test]
fn idm_weight_is_35_pct() {
    let config = empty_config();
    let input = runtime_input(perfect_runtime());
    let result = compute(&config, &input);
    let idm = find_dim(&result, "IDM");
    assert!((idm.weight - 0.35).abs() < 0.001);
}

#[test]
fn prf_weight_is_30_pct() {
    let config = empty_config();
    let input = runtime_input(perfect_runtime());
    let result = compute(&config, &input);
    let prf = find_dim(&result, "PRF");
    assert!((prf.weight - 0.30).abs() < 0.001);
}

// ============================================================================
// Legacy composite
// ============================================================================

#[test]
fn legacy_composite_blends_all_8() {
    let config = empty_config();
    let input = runtime_input(perfect_runtime());
    let result = compute(&config, &input);
    // 8 dimensions: COR, IDM, PRF, SAF, OBS, DOC, RES, CMP
    assert_eq!(result.dimensions.len(), 8);
    // Composite is weighted average of all 8
    assert!(result.composite > 0);
}

#[test]
fn runtime_composite_present_with_runtime() {
    let config = empty_config();
    let input = runtime_input(perfect_runtime());
    let result = compute(&config, &input);
    assert!(result.runtime_composite.is_some());
    assert!(result.runtime_composite.unwrap() > 0);
}

#[test]
fn runtime_composite_absent_without_runtime() {
    let config = empty_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    assert!(result.runtime_composite.is_none());
    assert!(result.runtime_grade.is_none());
}
