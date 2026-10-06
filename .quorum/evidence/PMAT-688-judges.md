# Judges — forjar#688: a failed resource is recorded as `converged`

Lanes and their findings: see `PMAT-688-lanes.md`. Line numbers are at head f17a11c5.

## CONFIRMED

1. [cause+fix] C1 — The tag writes the variant name, so the only honest `action` for a failure is a variant of its own.
   - evidence: `src/core/types/run_log_types.rs:90` keeps `tag = "action"`; `src/core/types/run_log_types.rs:105` is the new `Failed` variant; `src/core/types/run_log_types.rs:75` counts it into `summary.failed`; `src/cli/logs.rs:256` labels it FAILED. Measured on main 61d5a703: a task exiting 1 wrote `{action: converged, exit_code: 1, failed: true}` (`tests/falsification_688_failed_run_meta.rs:73` red).
2. [compat] C2 — Old rows go through `RecordedRunStatus`, which holds every shape ever written.
   - evidence: `src/core/types/run_log_types.rs:125` declares it, `src/core/types/run_log_types.rs:147` maps `converged` + `failed: true` (line 154) to `Failed`; `tests/falsification_688_failed_run_meta.rs:121` reads an old row and re-serializes it as `action: failed`, red on main.
3. [writer] C3 — `fail()` is the one function all three ran-and-failed paths call, and it now writes the row.
   - evidence: `src/core/executor/machine_wave_record.rs:254` is `fn fail`; the write is at `src/core/executor/machine_wave_record.rs:281`. `tests/falsification_688_failed_run_meta.rs:104` (exit 0, check 1) read a Null row on main and `failed` with the fix.
4. [skip] C4 — The JIDOKA dependency skip writes `Skipped { reason }`, the variant the type documents for exactly this and that nothing wrote before.
   - evidence: `src/core/executor/machine_b.rs:178`; `tests/falsification_688_failed_run_meta.rs:141` under `continue_independent` read a Null row with machine_b.rs at c19c6151 and `skipped`, reason naming `guard`, with f17a11c5.
5. [falsified] C5 — The test reads the bytes apply wrote, not a value forjar serialized for it.
   - evidence: `tests/falsification_688_failed_run_meta.rs:43` runs `CARGO_BIN_EXE_forjar`, `tests/falsification_688_failed_run_meta.rs:42` returns untyped YAML. Measured locally: src files at 61d5a703 gave "1 passed; 3 failed" of the first four; machine_b.rs at c19c6151 gave "4 passed; 1 failed"; at f17a11c5 "5 passed". ci.yml runs `--test falsification_688_failed_run_meta`.

## REFUTED

1. [lead] R1 — Only the writer is wrong; it could have written `failed` with the old type.
   - corrected: the old enum had three variants, Noop, Converged and Skipped, and the tag writes the variant name, so no writer could put `action: failed` on disk. The fix had to be in the type (`src/core/types/run_log_types.rs:105`).
2. [lead] R2 — Removing `failed` from `Converged` makes run dirs written before this change unreadable.
   - corrected: they read through `RecordedRunStatus` (`src/core/types/run_log_types.rs:125`), and `tests/falsification_688_failed_run_meta.rs:121` deserializes one and gets `Failed`.
