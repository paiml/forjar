# Quorum evidence — PMAT-217 — adjudicated claims

## CONFIRMED

1. [gate] CORPUS-NOT-NEIGHBOURS — the test needs aprender's corpus, so the corpus feature is the right gate and a directory check never was.
- evidence: the rule that now pins this decision is at tests/falsification_contracts_corpus_tests_are_feature_gated.rs:209, a file this branch ADDS. The assertion it guards queries `softmax` and requires a non-empty result, and the index it queries is built from forjar's own `contracts/`. All three lanes reached this independently. The measurement that settles it is behavioural: with `--features aprender-corpus` and no corpus present the test fails alongside its 38 peers, exactly 39 failures, which is what "this test needs the corpus" looks like from outside.

2. [gate] WORKSPACE-GREEN — the red that started this is gone, from the path that was red.
- evidence: the shim whose canned count gate F reads sits at tests/falsification_coverage_gate_mutation_scope.rs:68 at the merge base, printing `43 ignored`; that line is why the first attempt turned gate F red inside its own suite. `cargo test --workspace` exits 0 with 311 test binaries green, run in `~/src/forjar` where a sibling aprender is visible. Before the fix the same command in the same directory reported one failure, and `origin/main` at 54e36f13 reproduced it with no local changes.

3. [narrowness] NO PRODUCTION CODE — every file in the diff traces to the gated set growing by one.
- evidence: the only Rust this branch adds is the rule file, whose two cases begin at tests/falsification_contracts_corpus_tests_are_feature_gated.rs:103 and tests/falsification_contracts_corpus_tests_are_feature_gated.rs:209. The six files are the test, the two recorded figures in `scripts/dogfood/coverage.sh`, the shim's canned count, the new rules, and the living prose in `Cargo.toml` and `VENDORED.md`. `git diff --stat origin/main...HEAD` names no file under `src/`. No lane answered this question, so it is judged here rather than assumed.

## REFUTED

4. [completeness] THE FIGURE LIVES IN FIVE PLACES — the first version moved two of them.
- corrected: tests/falsification_coverage_gate_mutation_scope.rs:68, which resolves at the merge base, carried a canned "43 ignored" and gate F went red inside its own falsification suite; the new invariant is what caught it. Review then found two more, in `Cargo.toml` and `VENDORED.md`. The invariant now covers the tree, both recorded figures, the shim and the living prose, and mutating any one of them fails it.

5. [vacuity] DEAD PARSER — the suite shipped an attribute parser whose output was discarded.
- corrected: the helper that replaced it is tests/falsification_contracts_corpus_tests_are_feature_gated.rs:74, and it returns bodies only. All three lanes found the dead parser, and it was a leftover from a rule that had already been replaced. Removed rather than wired up, because the rule reads bodies and bodies are all it needs.

6. [vacuity] EVADABLE ONE CALL DEEP — the rule could be escaped by moving the check into a helper, and one file was escaping it that way already.
- corrected: the scan is file-wide and the exemption is a named constant at tests/falsification_contracts_corpus_tests_are_feature_gated.rs:215, so `cross_project_tests.rs` is exempt by name with its reason, plus a case that fails if that file stops reaching for the sibling. Measured: a helper-hidden version of the defect in a non-exempt file is caught now and was not before. The rule still cannot catch a determined rewrite, and its docstring now says so instead of implying otherwise.
