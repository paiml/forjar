//! Tests for FJ-1329: Reproducibility score.

use super::purity::PurityLevel;
use super::repro_score::{compute_score, grade, ReproInput};

#[test]
fn test_fj1329_perfect_score() {
    let inputs = vec![
        ReproInput {
            name: "nginx".to_string(),
            purity: PurityLevel::Pure,
            has_store: true,
            has_lock_pin: true,
        },
        ReproInput {
            name: "curl".to_string(),
            purity: PurityLevel::Pure,
            has_store: true,
            has_lock_pin: true,
        },
    ];
    let score = compute_score(&inputs);
    assert!((score.composite - 100.0).abs() < 0.01);
    assert!((score.purity_score - 100.0).abs() < 0.01);
    assert!((score.store_score - 100.0).abs() < 0.01);
    assert!((score.lock_score - 100.0).abs() < 0.01);
}

#[test]
fn test_fj1329_zero_score() {
    let inputs = vec![ReproInput {
        name: "installer".to_string(),
        purity: PurityLevel::Impure,
        has_store: false,
        has_lock_pin: false,
    }];
    let score = compute_score(&inputs);
    assert!((score.composite - 0.0).abs() < 0.01);
}

#[test]
fn test_fj1329_mixed_resources() {
    let inputs = vec![
        ReproInput {
            name: "nginx".to_string(),
            purity: PurityLevel::Pinned,
            has_store: true,
            has_lock_pin: true,
        },
        ReproInput {
            name: "script".to_string(),
            purity: PurityLevel::Impure,
            has_store: false,
            has_lock_pin: false,
        },
    ];
    let score = compute_score(&inputs);
    // purity: (75+0)/2 = 37.5, store: 50%, lock: 50%
    // composite: 37.5*0.5 + 50*0.3 + 50*0.2 = 18.75 + 15 + 10 = 43.75
    assert!((score.composite - 43.75).abs() < 0.01);
    assert!((score.purity_score - 37.5).abs() < 0.01);
    assert!((score.store_score - 50.0).abs() < 0.01);
    assert!((score.lock_score - 50.0).abs() < 0.01);
}

#[test]
fn test_fj1329_empty_inputs() {
    let score = compute_score(&[]);
    assert!((score.composite - 100.0).abs() < 0.01);
    assert!(score.resources.is_empty());
}

#[test]
fn test_fj1329_resource_breakdown() {
    let inputs = vec![
        ReproInput {
            name: "a".to_string(),
            purity: PurityLevel::Pure,
            has_store: true,
            has_lock_pin: true,
        },
        ReproInput {
            name: "b".to_string(),
            purity: PurityLevel::Constrained,
            has_store: false,
            has_lock_pin: false,
        },
    ];
    let score = compute_score(&inputs);
    assert_eq!(score.resources.len(), 2);
    assert_eq!(score.resources[0].name, "a");
    assert_eq!(score.resources[0].purity, PurityLevel::Pure);
    assert!(score.resources[0].score > score.resources[1].score);
}

#[test]
fn test_fj1329_grade_a() {
    assert_eq!(grade(100.0), "A");
    assert_eq!(grade(90.0), "A");
}

#[test]
fn test_fj1329_grade_b() {
    assert_eq!(grade(89.9), "B");
    assert_eq!(grade(75.0), "B");
}

#[test]
fn test_fj1329_grade_c() {
    assert_eq!(grade(74.9), "C");
    assert_eq!(grade(50.0), "C");
}

#[test]
fn test_fj1329_grade_d() {
    assert_eq!(grade(49.9), "D");
    assert_eq!(grade(25.0), "D");
}

#[test]
fn test_fj1329_grade_f() {
    assert_eq!(grade(24.9), "F");
    assert_eq!(grade(0.0), "F");
}

#[test]
fn test_fj1329_all_pinned_with_store_and_lock() {
    let inputs = vec![ReproInput {
        name: "pkg".to_string(),
        purity: PurityLevel::Pinned,
        has_store: true,
        has_lock_pin: true,
    }];
    let score = compute_score(&inputs);
    // purity: 75, store: 100, lock: 100
    // composite: 75*0.5 + 100*0.3 + 100*0.2 = 37.5 + 30 + 20 = 87.5
    assert!((score.composite - 87.5).abs() < 0.01);
    assert_eq!(grade(score.composite), "B");
}

#[test]
fn test_fj1329_constrained_scores() {
    let inputs = vec![ReproInput {
        name: "nginx".to_string(),
        purity: PurityLevel::Constrained,
        has_store: false,
        has_lock_pin: false,
    }];
    let score = compute_score(&inputs);
    // purity: 25, store: 0, lock: 0
    // composite: 25*0.5 = 12.5
    assert!((score.composite - 12.5).abs() < 0.01);
}

#[test]
fn test_fj1329_score_range() {
    for purity in &[
        PurityLevel::Pure,
        PurityLevel::Pinned,
        PurityLevel::Constrained,
        PurityLevel::Impure,
    ] {
        for has_store in &[true, false] {
            for has_lock in &[true, false] {
                let inputs = vec![ReproInput {
                    name: "r".to_string(),
                    purity: *purity,
                    has_store: *has_store,
                    has_lock_pin: *has_lock,
                }];
                let score = compute_score(&inputs);
                assert!(
                    (0.0..=100.0).contains(&score.composite),
                    "score {:.1} out of range for {:?}/store={}/lock={}",
                    score.composite,
                    purity,
                    has_store,
                    has_lock
                );
            }
        }
    }
}
