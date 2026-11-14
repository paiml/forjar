# Lanes — fleet ops delivery train (PMAT-890 / forjar#899)

Two-round `LANES=3` review by `agy` (`Gemini 4 Carbon (thinking=high)`), distinct from the Claude PR author model:

- **Round 1 (`@cf60cd9c`, 3/3 FAIL)**:
  - **Lane 1 (Selector & `plan`/`apply` Parity)**: `FAIL` — `src/cli/plan_compute.rs:33` passed `selectors.tag` straight into `planner::plan` without `depends_on` closure while `apply -t` closed over `depends_on` (`src/cli/apply_selection/closure.rs:324`).
  - **Lane 2 (Failure Modes & CI Gate D)**: `FAIL` — `src/cli/plan.rs:162` `missing_state_refusal` caused `scripts/dogfood/docs.sh` to fail at `README.md:197` (`forjar plan -f forjar.yaml`).
  - **Lane 3 (Sealed-Plan Contract & Audit Completeness)**: `FAIL` — `src/cli/plan.rs:101` mutated `config` via `--param` prior to sealing `--out`, causing `PLAN_HASH_MISMATCH` in `apply --plan-file`, and `docs/audits/impl-PMAT-890-receipt.md` recorded `PARTIAL`.
- **Round 2 (`@bc4e97f6`, 3/3 PASS)**:
  - **Lane 1**: `PASS` — verified `plan_compute.rs:33` + `plan_selector.rs:92` `-t` closure parity, `plan --param --out` refusal (`src/cli/plan.rs:150`), and `README.md:197` Gate D fix.
  - **Lane 2**: `PASS` — verified `state_visibility.rs:80`, `drift_lock_names.rs:12`, and `#688` `ResourceAction::Failed` serialization/legacy normalization (`machine_wave_record.rs:210`, `run_log_types.rs:215`).
  - **Lane 3**: `PASS` — verified `falsification_688_failed_resource_action_failed` (`2/2`) + `tests_plan_review_fixes` (`3/3`) green and surgical revert of `src/core/executor/machine_wave_record.rs` on `<host>` (`<SCRATCH>`) exits `rc=101`.
