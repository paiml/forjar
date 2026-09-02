//! FJ-409 (E06): Store is not scored
//!
//! WHAT WAS OBSERVABLY WRONG:
//! The `store: true` flag was falsely increasing the reproducibility score and purity level
//! (upgrading it to Pure) even though no actual store behavior was enforced for `apt` packages.
//! Two configs that yielded byte-identical apply scripts were scoring differently (68 vs 38).
//!
//! WHY THESE ASSERTIONS:
//! We assert that parsing a config with `store: true` and one without it results in the
//! exact same reproducibility score, verifying that declared-but-not-enforced flags do not
//! skew the score.

use forjar::core::store::purity::{classify, PurityLevel, PuritySignals};
use forjar::core::store::repro_score::{compute_score, ReproInput};

#[test]
fn test_e06_store_flag_does_not_affect_score() {
    let signals_plain = PuritySignals {
        has_version: true,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![],
    };

    let signals_with_store = PuritySignals {
        has_version: true,
        has_store: true, // This is the only difference
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![],
    };

    let result_plain = classify("ripgrep", &signals_plain);
    let result_store = classify("ripgrep", &signals_with_store);

    // They should have the same purity level
    assert_eq!(
        result_plain.level, result_store.level,
        "Purity level should not change just because store: true is declared"
    );
    assert_eq!(result_store.level, PurityLevel::Pinned);

    let score_plain = compute_score(&[ReproInput {
        name: "ripgrep".to_string(),
        purity: result_plain.level,
        has_store: false,
        has_lock_pin: true,
    }]);

    let score_store = compute_score(&[ReproInput {
        name: "ripgrep".to_string(),
        purity: result_store.level,
        has_store: true,
        has_lock_pin: true,
    }]);

    // The composite score must be completely equal
    assert!(
        (score_plain.composite - score_store.composite).abs() < 0.001,
        "Scores must be equal for configs differing only by store: true"
    );
}
