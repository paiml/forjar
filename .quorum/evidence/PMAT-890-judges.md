# Judges — fleet ops delivery train (PMAT-890 / forjar#899)

Round count and heads: Round 1 `cf60cd9c` (`3/3 FAIL`) -> Round 2 `bc4e97f6` (`3/3 PASS`); see `PMAT-890-lanes.md`. Every claim `C1..C4` and `R1..R2` was independently attacked across all 3 lanes.

## CONFIRMED

1. [code] C1 — `src/cli/apply_selection/closure.rs:324`, `src/cli/plan_compute.rs:33`, and `src/cli/plan_selector.rs:92` close `-t`/`--tag` over `depends_on` prerequisites and reject unmatched tags identically across `plan -t`, `apply -t`, and `apply --dry-run -t`.
   - evidence: `src/cli/apply.rs:120` clears `cfg.tag_filter = None` after closure selection so untagged prerequisites are not dropped downstream, and `src/cli/tests_plan_review_fixes.rs:18` plus `src/cli/tests_apply_selection_tag.rs:12` pass (`26/26` selector/review tests green on `<host>`).
2. [test] C2 — `src/core/executor/machine_wave_record.rs:210` records failed resources with `ResourceAction::Failed`, while `src/core/types/run_log_types.rs:215` normalizes legacy `Converged + failed: true` JSONL records on read.
   - evidence: `tests/falsification_688_failed_resource_action_failed.rs:28` passed `2/2` (`failed_task_is_recorded_as_action_failed` and `legacy_converged_with_failed_flag_reads_as_failed`) on `<host>`, and reverting `src/core/executor/machine_wave_record.rs` to `origin/main` in `<SCRATCH>` turned it RED (`FJ_899_REVERT_RC=101`).
3. [code] C3 — `src/cli/plan.rs:101` applies `--param k=v` overrides during planning (`src/cli/tests_plan_param.rs:15`) and `src/cli/plan.rs:150` refuses `--param` combined with `--out` before writing any plan artifact.
   - evidence: `src/cli/tests_plan_review_fixes.rs:58` verifies `plan --param k=v --out p.json` exits with an explicit refusal naming `apply --plan-file` rather than writing a sealed plan guaranteed to fail `PLAN_HASH_MISMATCH`.
4. [doc] C4 — `src/cli/state_visibility.rs:80` refuses missing `--state-dir` directories explicitly, `src/cli/drift_lock_names.rs:12` reports per-machine lock paths in drift output, and `README.md:197` + `CHANGELOG.md:8` satisfy Gate D and release-note traceability.
   - evidence: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `state_visibility` (`5/5`), and `README.md:197` (`forjar plan -f forjar.yaml --state-dir state`) all passed cleanly on `<host>`.

## REFUTED

1. [code] R1 — Round 1 (`cf60cd9c`) claimed `-t` dependency closure parity while leaving `src/cli/plan_compute.rs:33` passing `selectors.tag` as a raw planner filter without `depends_on` closure.
   - corrected: Round 2 (`bc4e97f6`) updated `src/cli/plan_compute.rs:33` and `src/cli/plan_selector.rs:92` to close `selectors.tag` through `resolver::goal_closure` and reject unknown tags, locked down by `src/cli/tests_plan_review_fixes.rs:18` (verified RED `rc=101` when reverted).
2. [code] R2 — Round 1 (`cf60cd9c`) permitted `forjar plan --param k=v --out plan.json` (`src/cli/plan.rs:101`) despite `apply --plan-file` hashing un-overridden `forjar.yaml`, and broke Gate D (`scripts/dogfood/docs.sh`) at `README.md:197`.
   - corrected: Round 2 (`bc4e97f6`) added the explicit `--param --out` refusal in `src/cli/plan.rs:150` (`src/cli/tests_plan_review_fixes.rs:58`) and updated `README.md:197` to pass `--state-dir state`, restoring Gate D to `rc=0`.
