# Judges — forjar#683 follow-up: the falsifier within the pre-commit limits

Lanes and findings: see `PMAT-683-tgc-lanes.md`. Line numbers are at the head.

## CONFIRMED

1. [limit] C1 — `toolchain_gaps` is under the hook's limits, and so is every helper it calls.
   - evidence: `pmat analyze complexity --file tests/falsification_683_cross_mounts_under_work_root.rs` reports no function over Cognitive 25 or Cyclomatic 30; `toolchain_gaps` at `tests/falsification_683_cross_mounts_under_work_root.rs:297` is 11 / 5, `first_gap` at `tests/falsification_683_cross_mounts_under_work_root.rs:277` is 12 / 6. The commit went through the pre-commit hook: "Complexity check... ✅".
2. [equivalence] C2 — The scan returns the same first step the old loop broke on, and the env predicate is the old one.
   - evidence: the install branch and the gap branch are at `tests/falsification_683_cross_mounts_under_work_root.rs:281` and `tests/falsification_683_cross_mounts_under_work_root.rs:283`, the early return at `tests/falsification_683_cross_mounts_under_work_root.rs:284`; the env test at `tests/falsification_683_cross_mounts_under_work_root.rs:291` is called at `tests/falsification_683_cross_mounts_under_work_root.rs:300`. Two claude-sonnet-5 lanes checked the equivalence line by line, both PASS.
3. [test] C3 — The falsifier still fails when the check is broken.
   - evidence: `cargo test --locked --test falsification_683_cross_mounts_under_work_root`: 5 passed. With `return Some(step);` at `tests/falsification_683_cross_mounts_under_work_root.rs:284` replaced by `let _ = step;`: 4 passed, 1 failed (`a_leg_with_no_toolchain_install_is_caught`, `tests/falsification_683_cross_mounts_under_work_root.rs:351`). With release.yml at 1d3e2d45: 4 passed, 1 failed (`every_job_that_sets_rustup_home_installs_a_toolchain_first`, `tests/falsification_683_cross_mounts_under_work_root.rs:318`), naming all four Linux legs. Restored: 5 passed. Clippy `-D warnings` and `cargo fmt --check` clean.
4. [scope] C4 — Only the test file and the roadmap entry change.
   - evidence: `git diff --stat origin/main...HEAD` lists `tests/falsification_683_cross_mounts_under_work_root.rs` and `docs/roadmaps/roadmap.yaml` (plus this evidence and the receipt).

## REFUTED

1. [scope] R1 — The branch does not deliver #683, because it moves no RUSTUP_HOME. Raised by claude-haiku-4-5 in round 1.
   - corrected: the move merged in #690 (ec477d93). This branch is the follow-up named in the roadmap's fourth acceptance criterion for PMAT-683 (`docs/roadmaps/roadmap.yaml`, added by a96b7107). Round 2 ran with that criterion committed and was 3/3 PASS.
