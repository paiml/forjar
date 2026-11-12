# Lanes — `build.rs` runtime `CARGO_MANIFEST_DIR` & tool-job guard (PMAT-194 / forjar#909)

Two-round `LANES=3` review by `agy` (`Gemini 4 Carbon (thinking=high)`), distinct from the Claude author model:

- **Round 1 (`@6f655eac`, 3/3 FAIL)**:
  - **Lane 1 (Detector & Build-Script Correctness)**: `FAIL` — caught multi-`-p` false negative (`-p forjar-contracts -p forjar`) and `--package=<pkg>` false positive in `tests/falsification_tool_jobs_run_where_the_tools_are.rs:289`.
  - **Lane 2 (Failure Modes & Workspace Flag Edge Cases)**: `FAIL` — `--workspace` combined with `-p` and multi-package unions bypassed `every_unfiltered_lib_test_job_runs_where_rsync_is`.
  - **Lane 3 (Falsification Coverage & Contract Audit)**: `FAIL` — required explicit unit assertions for `-p forjar-contracts -p forjar` and `--package=forjar-contracts`.
- **Round 2 (`@8905e5a2`, 3/3 PASS)**:
  - **Lane 1**: `PASS` — verified `build.rs:14,22` runtime `std::env::var("CARGO_MANIFEST_DIR")` + `rerun-if-env-changed`.
  - **Lane 2**: `PASS` — verified `tests/falsification_tool_jobs_run_where_the_tools_are.rs:285` tracks `saw_pkg`, `saw_root`, and `--workspace` across the full argv slice with unit tests for multi-`-p` and `--package=`.
  - **Lane 3**: `PASS` — verified `falsification_build_rs_manifest_dir_runtime` (`3/3`) passes at `8905e5a2` and all 3 tests fail (`FJ_909_REVERT_RC=101`) when `build.rs` is reverted to `origin/main` on `<host>` (`<SCRATCH>`).
