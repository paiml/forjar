# Judges — `build.rs` runtime `CARGO_MANIFEST_DIR` & tool-job guard (PMAT-194 / forjar#909)

Round count and heads: Round 1 `6f655eac` (`3/3 FAIL`) -> Round 2 `8905e5a2` (`3/3 PASS`); see `PMAT-194-lanes.md`. Every claim `C1..C4` and `R1..R2` met 3 independent refuters across all 3 lanes.

## CONFIRMED

1. [code] C1 — `build.rs:14` reads `CARGO_MANIFEST_DIR` at runtime via `std::env::var("CARGO_MANIFEST_DIR")` rather than compile-time `env!("CARGO_MANIFEST_DIR")`, eliminating embedded build-host paths from the compiled build-script binary.
   - evidence: `tests/falsification_build_rs_manifest_dir_runtime.rs:15` (`build_rs_has_no_compile_time_manifest_dir` and `build_rs_reads_manifest_dir_at_run_time`) passed on `<host>`, and reverting `build.rs` to `origin/main` failed all 3 tests (`FJ_909_REVERT_RC=101`).
2. [test] C2 — `build.rs:22` emits `cargo::rerun-if-env-changed=CARGO_MANIFEST_DIR` so relocated checkouts re-execute `build.rs` instead of reusing stale `OUT_DIR` artifacts.
   - evidence: `tests/falsification_build_rs_manifest_dir_runtime.rs:38` (`build_rs_reruns_when_the_manifest_dir_changes`) passed at `8905e5a2` and failed on `origin/main`'s `build.rs`.
3. [test] C3 — `tests/falsification_tool_jobs_run_where_the_tools_are.rs:285` inspects the complete Cargo argument list across `-p <pkg>`, `--package <pkg>`, `--package=<pkg>`, and `--workspace`, flagging any invocation whose package union includes root `forjar`.
   - evidence: `cargo test --test falsification_tool_jobs_run_where_the_tools_are` passed cleanly on `<host>`, including the new multi-`-p` and `--package=` unit assertions.
4. [doc] C4 — `CHANGELOG.md:8` and `docs/roadmaps/roadmap.yaml` record `PMAT-194` with clean formatting and lints.
   - evidence: `cargo fmt --all -- --check` (`rc=0`) and `cargo clippy --all-targets -- -D warnings` (`rc=0`) passed on `<host>`.

## REFUTED

1. [code] R1 — Round 1 (`6f655eac`) early-returned `false` on the first non-root `-p <pkg>` in `tests/falsification_tool_jobs_run_where_the_tools_are.rs:289`, allowing `cargo test -p forjar-contracts -p forjar` to evade the `rsync` runner guard.
   - corrected: Round 2 (`8905e5a2`) scans all package arguments in `tests/falsification_tool_jobs_run_where_the_tools_are.rs:285` (`!saw_pkg || saw_root || saw_workspace`) and locks down `-p forjar-contracts -p forjar` with a unit test.
2. [code] R2 — Round 1 (`6f655eac`) ignored `--package=<pkg>` (`=` syntax) in `tests/falsification_tool_jobs_run_where_the_tools_are.rs:289`, misclassifying `--package=forjar-contracts` as unfiltered.
   - corrected: Round 2 (`8905e5a2`) added `tok.strip_prefix("--package=")` handling in `tests/falsification_tool_jobs_run_where_the_tools_are.rs:295` with explicit unit test coverage.
