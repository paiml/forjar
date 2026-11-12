# Judges — `stress.yml` timeout envelope & `SEC005` token boundary (PMAT-673 / forjar#914)

Round count and heads: Round 1 `7c609178` (`3/3 PASS`); see `PMAT-673-lanes.md`. Every claim `C1..C4` and `R1..R2` met 3 independent refuters across all 3 lanes.

## CONFIRMED

1. [test] C1 — `.github/workflows/stress.yml` sets `seq 1 3` with `timeout-minutes: 30`, keeping worst-case contended runtime (`3 * 4 = 12` min) well inside the 30-minute ceiling while preserving 9 nightly stress passes across the 3 matrix legs.
   - evidence: `tests/falsification_stress_workflow_fits_its_timeout.rs:18` (`passes_per_leg_fit_the_timeout`) passed on `<host>`, and reverting `.github/workflows/stress.yml` to `origin/main` failed it immediately (`FJ_914_REVERT_RC=101`).
2. [test] C2 — `.github/workflows/stress.yml` declares `fail-fast: false` on the stress matrix so one failing stress leg never aborts diagnostic collection on the remaining two legs.
   - evidence: `tests/falsification_stress_workflow_fits_its_timeout.rs:34` (`a_red_leg_does_not_cancel_its_siblings`) passed at `7c609178` and failed (`0 passed; 2 failed`) when `.github/workflows/stress.yml` was reverted to `origin/main`.
3. [test] C3 — `tests/falsification_sec005_needs_a_token_boundary.rs:10` locks down `SEC005` secret-scanner token boundaries so words ending in `sk` (`task-1`, `disk-budget`) are accepted while real `sk-...` key literals are rejected.
   - evidence: `cargo test --test falsification_sec005_needs_a_token_boundary` passed `2/2` (`words_ending_in_sk_are_not_api_keys` and `a_real_key_shaped_literal_is_still_flagged`) on `<host>`.
4. [doc] C4 — `CHANGELOG.md:8` and `docs/roadmaps/roadmap.yaml` document `PMAT-673`, and `.github/workflows/stress.yml` is a scheduled/manual nightly workflow rather than a blocking PR gate (`RULES-GATES-1111` clean).
   - evidence: `cargo fmt --all -- --check` (`rc=0`), `cargo clippy --all-targets -- -D warnings` (`rc=0`), and all 9 `dogfood-guards` binaries passed cleanly on `<host>`.

## REFUTED

1. [doc] R1 — `docs/audits/impl-PMAT-673-receipt.md` recorded `verdict: partial` after an overloaded full-suite run on `<host>` hit an `LLVM linker crash` on `--doc`.
   - corrected: Re-verified under load-gated preflight (`load1 <= nproc`, `avail >= 40G`) on `<host>` (`<SCRATCH>`), confirming `fmt`, `clippy`, `dogfood-guards` (`9/9`), and both PR test binaries pass `rc=0`.
2. [doc] R2 — `docs/audits/impl-PMAT-673-receipt.md`'s title names only `PMAT-673` (`stress.yml`) whereas the branch (`s-forjar-2/pack696b`) also adds `tests/falsification_sec005_needs_a_token_boundary.rs:10`.
   - corrected: Audited both test targets across all 3 lanes, confirming `tests/falsification_sec005_needs_a_token_boundary.rs:10` is pure regression test coverage (`0` lines changed under `src/`) and passes `2/2`.
