# Quorum evidence — #390-A / #390-B — adjudicated claims

Canonical format: `N. [lane] claim` with an indented `- evidence:` or `- corrected:` subline.

## CONFIRMED — 4 claims survived refutation

1. [probe] (explains-symptom) The parallel wave path wrote no run log at all, destroying a failed task's transcript rather than hiding it.
   - evidence: `machine_wave.rs` never called `run_capture`. The mechanical cause was structural: the script was built and dropped inside the spawn closure, so nothing outside the thread had the text `run_capture` needs. Fixed by having `execute_wave_io` carry it out — see src/core/executor/machine_wave.rs:16 (the `WaveResult` type) and src/core/executor/machine_wave.rs:203 (the `capture_exec_output` call). Pinned by tests/falsification_390ab_parallel_wave_parity.rs:97.

2. [probe] (explains-symptom) The parallel path skipped post-apply verification, so identical configs could report converged and failed.
   - evidence: The success arm went straight to `record_success`, so FJ-2731 and FJ-2732 silently did not run. The population is every plain `type: task`, because `resources::task::check_script` falls through to `verdict::always_diverged("task=pending")` with no completion_check and no output_artifacts. Fixed at src/core/executor/machine_wave.rs:229. Pinned by a parity assertion — the same config run both ways must agree — at tests/falsification_390ab_parallel_wave_parity.rs:151.

3. [design] (explains-symptom) The fix requires a signature change that ripples, and that is why the defect survived: the script was consumed by `and_then` inside the closure.
   - evidence: `execute_wave_io` returned `(usize, f64, Result<ExecOutput, String>)`. Carrying the script out required a fourth element and a matching change at the caller in machine_b.rs, plus the pre-hook early-return arm. Introduced as a named type at src/core/executor/machine_wave.rs:16 rather than a bare tuple so the meaning of the fourth element is documented where it is declared.

4. [pmat] (unrelated-defect) The new suite is not vacuous and the repo-wide population did not grow.
   - evidence: `analyze_vacuous_tests` finds no entry for tests/falsification_390ab_parallel_wave_parity.rs:97 or its siblings. Repo-wide tautologies unchanged from the 1.24.0 baseline.

## REFUTED — 2 claims killed

1. [probe] refuted 1/1 — The first version of this test suite proved the fix worked.
   - corrected: It proved nothing. All four tests passed against a build with BOTH fixes reverted. Cause: `machine.rs:183` selects the wave path with `use_parallel && machine_changes.len() > 1`, and every fixture declared ONE resource — so each test silently ran the SEQUENTIAL path regardless of policy. Fixed by declaring two resources per fixture and adding `the_wave_path_is_actually_taken` at tests/falsification_390ab_parallel_wave_parity.rs:97 to guard the guard. With that corrected, 3 of 5 tests go red against the pre-fix code.

2. [probe] refuted 1/1 — This branch broke `cli::tests_build_image::cmd_build_far_produces_valid_archive`.
   - corrected: It did not. That test appeared once in a full-suite run and passes in isolation on BOTH this branch and main; the file has no reference to the wave or parallel paths, and a second full-suite run on this branch was clean at 13404 passed / 0 failed. A parallelism flake, recorded rather than attributed to this change.
