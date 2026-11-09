//! forjar#452: the vendored `forjar-contracts` crate's unit tests must run in CI.
//!
//! Since #436 moved the crate in-tree, CI scoped tests to the root package and
//! the crate's suite never executed; 38 tests rotted unseen.

#[test]
fn ci_runs_the_forjar_contracts_unit_tests() {
    let ci = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.github/workflows/ci.yml"
    ))
    .expect("read ci.yml");
    let runs = ci
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .any(|l| l.contains("cargo test") && l.contains("-p forjar-contracts"));
    assert!(
        runs,
        "no non-comment `cargo test ... -p forjar-contracts` step in ci.yml"
    );
}
