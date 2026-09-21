# Judges — quorum.yml's private CARGO_HOME (PMAT-588)

Round count and heads: see `PMAT-588-lanes.md`.

## CONFIRMED

1. [diff] C1 — The quorum.yml receipt job now sets a job-private CARGO_HOME outside the workspace, matching the four sibling workflows, and its rust-cache step is kept rather than deleted, so the job still caches while no longer sharing the fleet's cargo home.
   - evidence: the negative control at `tests/falsification_rust_cache_needs_a_private_cargo_home.rs:118` copies the real workflows, removes that one CARGO_HOME line, and requires the lint to refuse job receipt by name, asserted at `tests/falsification_rust_cache_needs_a_private_cargo_home.rs:169`.
2. [test] C2 — The lint classifies each job separately and exonerates only a non-shared CARGO_HOME at workflow, job, or cache-step scope, with a self-test of twelve cases, eight of them one per shape the review surfaced or the author found while fixing it.
   - evidence: `tests/falsification_rust_cache_needs_a_private_cargo_home.rs:72` runs the self-test and requires at least twelve cases; three of the eight shape fixtures give the wrong verdict under the pre-fix file-level lint, measured by running both.
3. [wiring] C3 — The lint is wired three ways: the make target runs the self-test before the scan, the lint target depends on it, and lint.yml gains a step running the same target, so the scan cannot pass trivially on a tree that already complies.
   - evidence: confirmed by every lane in rounds 1, 2 and 4; round 1's pro-low lane quoted the Makefile ordering explicitly.
4. [test] C4 — The Rust falsification test runs the lint instead of re-implementing its rule, so the two consumers of the rule cannot disagree again the way the round-1 review showed they did, and deleting or unwiring the lint turns cargo test red.
   - evidence: the harness at `tests/falsification_rust_cache_needs_a_private_cargo_home.rs:37` executes the script and fails if it is missing; the tree test at `tests/falsification_rust_cache_needs_a_private_cargo_home.rs:85` asserts anti-vacuity floors on the lint's own counts.
5. [scope] C5 — The branch only adds a check and an env var: no permissions block, secret, token, publish step or required-check name changes, and no step is removed, skipped or softened in any workflow it touches.
   - evidence: all four rounds' lanes reported no forbidden change; lint.yml gains one step and quorum.yml gains one env block and a comment.

## REFUTED

1. [lint] R1 — The first lint asked whether a private CARGO_HOME appeared anywhere in the file, so a workflow with two self-hosted caching jobs, only one of them private, passed as a whole although the second job still pruned the shared home.
   - corrected: the lint decides per job; the fixture with two self-hosted jobs and one private CARGO_HOME now fails, and the test at `tests/falsification_rust_cache_needs_a_private_cargo_home.rs:72` requires that fixture set to run.
2. [lint] R2 — The first lint judged any workflow containing a self-hosted job as self-hosted throughout, so a hosted job's cache step was reported as a fleet violation merely because it shared a file with a clean self-hosted job.
   - corrected: runs-on is read per job; a hosted cache beside a clean self-hosted job now passes, one of the eight shape fixtures.
3. [test] R3 — The Rust copy of the rule treated any line containing the CARGO_HOME key as a declaration, so a comment naming a private path exonerated a job whose real environment was the shared cargo home.
   - corrected: the second parser is gone; `tests/falsification_rust_cache_needs_a_private_cargo_home.rs:21` records why, and the lint strips comments before classifying, with a comment-only fixture that must fail.
4. [test] R4 — The criterion said the falsification test fails when quorum.yml is restored to main's version, but the test itself only asserted the patched tree, so the failure was a manual demonstration rather than something cargo test checks.
   - corrected: `tests/falsification_rust_cache_needs_a_private_cargo_home.rs:118` performs that restoration on a temporary copy of the real workflows and requires the lint to fail naming job receipt, asserted at `tests/falsification_rust_cache_needs_a_private_cargo_home.rs:160` and `tests/falsification_rust_cache_needs_a_private_cargo_home.rs:169`.
