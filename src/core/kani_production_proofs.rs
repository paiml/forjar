//! FJ-2201: Kani proofs on REAL production functions.
//!
//! Unlike the abstract-model harnesses in `kani_proofs.rs`, these call
//! actual production functions with bounded nondeterministic inputs.
//! No simplified models — real code paths exercised.
//!
//! Run with: `cargo kani --harness <name>`
//!
//! ## Harnesses
//!
//! | Harness | Production Function | Property |
//! |---------|-------------------|----------|
//! | `proof_mutation_grade_monotonic` | `MutationScore::grade()` | Higher score → higher/equal grade |
//! | `proof_mutation_grade_valid` | `MutationScore::grade()` | Returns only {A,B,C,F} |
//! | `proof_mutation_score_pct_bounded` | `MutationScore::score_pct()` | Result in [0,100] |
//! | `proof_convergence_pass_rate_bounded` | `ConvergenceSummary::pass_rate()` | Result in [0,100] |
//! | `proof_applicable_operators_valid` | `applicable_operators()` | Operator applicability invariant |

/// MutationScore::grade() is monotonic: higher score_pct → higher/equal grade.
///
/// Calls the real `MutationScore::grade()` and `score_pct()` production functions.
#[cfg(kani)]
#[kani::proof]
fn proof_mutation_grade_monotonic() {
    use super::types::MutationScore;

    let total: usize = kani::any();
    kani::assume(total > 0 && total <= 100);
    let detected_a: usize = kani::any();
    let detected_b: usize = kani::any();
    kani::assume(detected_a <= total);
    kani::assume(detected_b <= total);
    kani::assume(detected_a <= detected_b);

    let score_a = MutationScore {
        total,
        detected: detected_a,
        survived: total - detected_a,
        errored: 0,
    };
    let score_b = MutationScore {
        total,
        detected: detected_b,
        survived: total - detected_b,
        errored: 0,
    };

    let grade_a = score_a.grade();
    let grade_b = score_b.grade();

    let rank = |g: char| match g {
        'A' => 3,
        'B' => 2,
        'C' => 1,
        _ => 0,
    };
    assert!(
        rank(grade_b) >= rank(grade_a),
        "grade must be monotonic with score"
    );
}

/// MutationScore::grade() always returns one of {A, B, C, F}.
///
/// Calls the real `grade()` function on arbitrary valid inputs.
#[cfg(kani)]
#[kani::proof]
fn proof_mutation_grade_valid() {
    use super::types::MutationScore;

    let total: usize = kani::any();
    kani::assume(total <= 200);
    let detected: usize = kani::any();
    kani::assume(detected <= total);

    let score = MutationScore {
        total,
        detected,
        survived: total - detected,
        errored: 0,
    };
    let grade = score.grade();
    assert!(
        grade == 'A' || grade == 'B' || grade == 'C' || grade == 'F',
        "grade must be A, B, C, or F"
    );
}

/// MutationScore::score_pct() is bounded [0, 100].
///
/// Calls the real `score_pct()` function.
#[cfg(kani)]
#[kani::proof]
fn proof_mutation_score_pct_bounded() {
    use super::types::MutationScore;

    let total: usize = kani::any();
    kani::assume(total <= 100);
    let detected: usize = kani::any();
    kani::assume(detected <= total);

    let score = MutationScore {
        total,
        detected,
        survived: total - detected,
        errored: 0,
    };
    let pct = score.score_pct();
    assert!(pct >= 0.0, "score_pct must be >= 0");
    assert!(pct <= 100.0, "score_pct must be <= 100");
}

/// ConvergenceSummary::pass_rate() is bounded [0, 100].
///
/// Calls the real `pass_rate()` production function.
#[cfg(kani)]
#[kani::proof]
fn proof_convergence_pass_rate_bounded() {
    use super::store::convergence_runner::ConvergenceSummary;

    let total: usize = kani::any();
    kani::assume(total <= 100);
    let passed: usize = kani::any();
    kani::assume(passed <= total);

    let summary = ConvergenceSummary {
        total,
        passed,
        convergence_failures: 0,
        idempotency_failures: 0,
        preservation_failures: 0,
    };
    let rate = summary.pass_rate();
    assert!(rate >= 0.0, "pass_rate must be >= 0");
    assert!(rate <= 100.0, "pass_rate must be <= 100");
}

/// applicable_operators returns only operators valid for the resource type.
///
/// Calls the real `applicable_operators()` and `applicable_types()` functions.
#[cfg(kani)]
#[kani::proof]
fn proof_applicable_operators_valid() {
    use super::store::mutation_runner::applicable_operators;

    let rtype_idx: u8 = kani::any();
    kani::assume(rtype_idx < 4);
    let rtype = match rtype_idx {
        0 => "file",
        1 => "service",
        2 => "package",
        _ => "mount",
    };

    let ops = applicable_operators(rtype);
    for op in &ops {
        assert!(
            op.applicable_types().contains(&rtype),
            "operator must be applicable to the resource type"
        );
    }
}
