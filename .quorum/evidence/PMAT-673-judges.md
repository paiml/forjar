# Judges — `stress.yml` timeout envelope & `SEC005` token boundary (PMAT-673 / forjar#914)

Round count: no agy round is counted. The adjudications below are the author's record, each tied to a command or a diff line checkable at the head; the non-author lane verdicts for this head are the Opus lane files listed in `PMAT-673-lanes.md` (not adjudicated at the final rebound head).

## CONFIRMED

1. [test] C1 — `.github/workflows/stress.yml` sets `seq 1 3` with `timeout-minutes: 45`, keeping the modelled worst case (`3 * 9 + 10 = 37` min, 9 min per pass plus a 10 min build) inside the 45-minute ceiling while preserving 9 weekly stress passes across the 3 matrix legs.
   - evidence: `tests/falsification_stress_workflow_fits_its_timeout.rs:18` (`passes_per_leg_fit_the_timeout`) passes 2/2 at the head (Opus lane 2 re-ran it on t2yoga), and reverting `.github/workflows/stress.yml` to `origin/main` failed it immediately (`FJ_914_REVERT_RC=101`).
2. [test] C2 — `.github/workflows/stress.yml` declares `fail-fast: false` on the stress matrix so one failing stress leg never aborts diagnostic collection on the remaining two legs.
   - evidence: `tests/falsification_stress_workflow_fits_its_timeout.rs:34` (`a_red_leg_does_not_cancel_its_siblings`) passes at the head and fails (`0 passed; 2 failed`) when `.github/workflows/stress.yml` was reverted to `origin/main`.
3. [test] C3 — `tests/falsification_sec005_needs_a_token_boundary.rs:10` locks down `SEC005` secret-scanner token boundaries so words ending in `sk` (`disk-watch`, `ask-lane`, `desk-lamp`, in quoted assignments) are accepted while real `sk-...` key literals are rejected.
   - evidence: `cargo test --test falsification_sec005_needs_a_token_boundary` passed `2/2` (`words_ending_in_sk_are_not_api_keys` and `a_real_key_shaped_literal_is_still_flagged`) on `<host>` at the corrected head. The quoted-assignment form is red on bashrs `=6.68.0` and green on `=7.4.2` (Opus 5.5 lane measurement on t2yoga).
4. [doc] C4 — `CHANGELOG.md:8` and `docs/roadmaps/roadmap.yaml` document `PMAT-673`, and `.github/workflows/stress.yml` is a scheduled (weekly cron)/manual workflow rather than a blocking PR gate (`RULES-GATES-1111` clean).
   - evidence: `cargo fmt --all -- --check` (`rc=0`), `cargo clippy --all-targets -- -D warnings` (`rc=0`), and all 9 `dogfood-guards` binaries passed cleanly on `<host>`.

## REFUTED

1. [doc] R1 — `docs/audits/impl-PMAT-673-receipt.md` recorded `verdict: partial` after an overloaded full-suite run on `<host>` hit an `LLVM linker crash` on `--doc`.
   - not resolved: the receipt's `verdict: partial` stands (fmt, clippy, dogfood-guards and both PR test binaries pass; 2 suite targets are red on origin/main by content and need their own tickets).
2. [doc] R2 — `docs/audits/impl-PMAT-673-receipt.md`'s title names only `PMAT-673` (`stress.yml`) whereas the branch (`s-forjar-2/pack696b`) also adds `tests/falsification_sec005_needs_a_token_boundary.rs:10`.
   - corrected: Audited both test targets, confirming `tests/falsification_sec005_needs_a_token_boundary.rs:10` is pure regression test coverage (`0` lines changed under `src/`) and passes `2/2`.
