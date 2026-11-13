# Judges — Per-run PID file staging `.forjar-new.$$` & I8 script corpus coverage (PMAT-638 / forjar#906)

Round count and heads: Round 1 `1e5c9be8` (`3/3 PASS`); see `PMAT-638-lanes.md`. Every claim `C1..C4` and `R1..R2` met 3 independent refuters across all 3 lanes.

## CONFIRMED

1. [code] C1 — `src/resources/file.rs:182` stages regular-file writes to `'<path>.forjar-new.'$$` with `trap 'rm -f "$s"' EXIT`, applies owner/mode at `src/resources/file.rs:216` before `mv -f "$s" "$p"`, and updates `src/core/shell_escape.rs:200`.
   - evidence: `src/resources/tests_file_staging.rs:10` (`test_638_staging_name_is_per_run`) passed on `<host>`, and reverting `.$$` in `src/resources/file.rs:182` (`! cmp -s` verified) drove it RED (`FJ_906_REVERT_RC=101`).
2. [test] C2 — `src/transport/tests_generated_scripts_lint.rs:80` extends the I8 generated-script lint corpus with `file/state-file-content` and `file/state-file-source`.
   - evidence: `cargo test --lib transport::tests_generated_scripts_lint` passed `4/4` on `<host>`, proving generated per-PID staging scripts pass the I8 shell gate.
3. [code] C3 — `src/resources/package_check.rs:199` appends declared `completion_check` assertions across `apt`, `cargo`, `uv`, and `brew` check scripts.
   - evidence: `src/resources/tests_package_d.rs:90` (`12/12` tests) passed on `<host>` alongside `cargo fmt --all -- --check` (`rc=0`) and `cargo clippy --all-targets -- -D warnings` (`rc=0`).
4. [doc] C4 — `CHANGELOG.md:8`, `docs/roadmaps/roadmap.yaml:7110`, and `docs/audits/impl-PMAT-638-receipt.md:17` document both the concurrent-apply fix and the residual PID/SIGKILL boundary.
   - evidence: All Gate A audit, changelog, and version integration checks recorded in `docs/audits/impl-PMAT-638-receipt.md:75` passed (`rc=0`).

## REFUTED

1. [doc] R1 — An earlier docstring on `src/resources/file.rs:182` claimed `.forjar-new.$$` closed the staging race altogether, overstating protection against local symlink/PID prediction and `SIGKILL` before `mv -f`.
   - corrected: Commit `5b3bccad` removed the overclaim in `src/resources/file.rs:182` and `docs/audits/impl-PMAT-638-receipt.md:17` explicitly records guessable PIDs and `SIGKILL` orphans as residual scope (item 1 of `#638`).
2. [test] R2 — Pre-rebase CI failure on `dogfood-guards` (`no test target named falsification_coverage_gate_mutation_scope`) and 10 failing host integration binaries appeared to implicate `#906`.
   - corrected: Merging `origin/main` (`905fd5af`) resolved the stale workflow target list, and control execution in `docs/audits/impl-PMAT-638-receipt.md:40` proved the exact same 10 host-dependent targets fail on plain `origin/main` (`dbd1ec3e`).
