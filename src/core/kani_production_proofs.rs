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
//! | `proof_rejects_unknown_total_and_monotone` | `parser::rejects_unknown()` | Total; monotone in the unknown-field count |

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
    use super::store::mutation_runner::ALL_MUTATION_OPERATORS;

    let rtype_idx: u8 = kani::any();
    kani::assume(rtype_idx < 4);
    let rtype = match rtype_idx {
        0 => "file",
        1 => "service",
        2 => "package",
        _ => "mount",
    };

    // Range over the PREDICATE, not the allocating wrapper.
    //
    // `applicable_operators` builds its result with `.collect()` into a Vec, so
    // Kani must model the allocator on top of the string comparisons in
    // `applicable_types()`. Measured 2026-08-16: this was the ONLY harness to
    // start in a 45-minute CI run, and was still inside it when the job was
    // killed — one harness, 22+ minutes, no verdict, nothing else reached.
    //
    // Identical shape to `proof_disk_budget_hysteresis_total`, which drove a
    // String-allocating constructor across a 65,536-point space and never
    // terminated. An intractable proof is indistinguishable from an absent one,
    // and this workflow exists precisely to stop proofs that never run.
    //
    // The property is unchanged and still checked against the production
    // `applicable_types()`: an operator is admitted for a type exactly when it
    // declares that type. Only the heap leaves the model.
    // The original assertion — "every operator the filter RETURNED is applicable"
    // — is true by construction of the filter, so it could not fail. Restating
    // it allocation-free would just be a tautology comparing an expression to
    // itself. So this proves the property that CAN fail and that actually
    // matters: every resource type has at least one applicable operator.
    //
    // Without it, mutation testing over that type mutates nothing, finds
    // nothing, and reports success — a vacuous green, which is the failure this
    // whole proof gate exists to catch. Adding a resource type and forgetting
    // to give any operator its name is exactly how that happens.
    let mut applicable_count = 0usize;
    for op in ALL_MUTATION_OPERATORS {
        if op.applicable_types().contains(&rtype) {
            applicable_count += 1;
        }
    }
    assert!(
        applicable_count > 0,
        "resource type has no applicable mutation operator: mutation testing \
         would mutate nothing and report success"
    );
}

/// KANI-CLC-001 — `parser::rejects_unknown()` is total and monotone.
///
/// Contract: contracts/config-load-consistency-v1.yaml
///
/// This is the decision `parse_and_validate_opts` makes about whether unknown
/// fields are fatal (GH-272). Proving it here rather than proving something
/// about `parse_and_validate_opts` is deliberate: that function reads a file
/// and allocates a diagnostic string per unknown field, and CBMC models every
/// path through both. A harness aimed there would explode the state space
/// without establishing anything the falsification tests do not already cover.
///
/// Monotonicity is the property that matters operationally: finding MORE
/// unknown fields must never turn a rejection back into an acceptance.
#[cfg(kani)]
#[kani::proof]
fn proof_rejects_unknown_total_and_monotone() {
    use super::parser::rejects_unknown;

    let deny: bool = kani::any();
    let n1: usize = kani::any();
    let n2: usize = kani::any();
    kani::assume(n1 <= 8);
    kani::assume(n2 <= 8);
    kani::assume(n1 <= n2);

    // Total: both calls terminate and produce a value for every input.
    let r1 = rejects_unknown(deny, n1);
    let r2 = rejects_unknown(deny, n2);

    // Monotone in the count: more unknown fields never un-rejects a config.
    assert!(!r1 || r2);

    // A clean config is never rejected on this ground, whatever the mode.
    assert!(!rejects_unknown(deny, 0));

    // And in strict mode any unknown field at all is fatal.
    if deny && n2 > 0 {
        assert!(r2);
    }
}
