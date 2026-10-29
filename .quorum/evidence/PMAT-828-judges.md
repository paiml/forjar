# Judges — data source fault isolation (PMAT-828)

Round count and heads: see `PMAT-828-lanes.md`.

## CONFIRMED

1. [diff] C1 — One failing `data:` source no longer aborts resolution: every source is tried, healthy values are injected as `__data__<key>` and the failures come back as a list, which is what lets the unrelated resources be planned at all.
   - evidence: `src/core/resolver/data.rs:207` loops every source and collects `Err` into `failures`; `src/core/resolver/tests_data.rs:402` asserts a failure is recorded without an error and `src/core/resolver/tests_data.rs:416` that healthy sources still resolve.
2. [diff] C2 — Only the resources that reference the failed key are dropped, plus what depends on them: the boundary check keeps `data.foo` from matching `data.foobar` or `data.foo-bar`, and the `depends_on` cascade loops to a fixed point.
   - evidence: `src/core/resolver/data.rs:229` is the reference matcher with the boundary guards at `src/core/resolver/data.rs:237` and `src/core/resolver/data.rs:241`; `src/core/resolver/tests_data.rs:446` pins the hyphenated key.
3. [diff] C3 — The failure is never silent: `plan` and `apply` finish the unrelated work and then return an error naming each failed source, on the dry-run, events, `--out` and normal paths.
   - evidence: `src/cli/data_isolation.rs:32` builds the error from the recorded messages; `src/cli/apply.rs:186`, `src/cli/apply.rs:221` and `src/cli/apply.rs:272` return it, and `src/cli/plan.rs:142` and `src/cli/plan.rs:169` do the same for plan.
4. [test] C4 — Other callers keep the strict behaviour, so `validate`, `check` and friends still stop at the first bad source; only plan and apply isolate.
   - evidence: `src/core/resolver/tests_data.rs:439` asserts the strict resolver still returns the command failure for the same config.
5. [test] C5 — Through the real binary an unrelated file is written, the dependent and its `depends_on` child are not, the run exits non-zero, and a healthy source writes the dependent, so the fix cannot be "skip every templated resource".
   - evidence: `tests/falsification_data_source_fault_isolation.rs:68` asserts the three file outcomes and the exit; `tests/falsification_data_source_fault_isolation.rs:95` is the negative control; RED on origin/main, where the unrelated file is not written.
6. [test] C6 — A skipped resource is skipped, not forgotten: after a good apply, a later outage leaves the converged file and its lock entry in place, because nothing in apply prunes entries absent from the run.
   - evidence: `tests/falsification_data_source_fault_isolation.rs:114` applies healthy, applies broken, then checks the file and parses the lock for the `dependent` key.

## REFUTED

1. [doc] R1 — The test header said every assertion was on written files plus the exit status, while one asserts the output names the source, and it said `plan` was covered by the in-crate test although that test only covers `--out`; both sentences were false as written.
   - corrected: `tests/falsification_data_source_fault_isolation.rs:6` now lists the output assertion and names `plan --out` as what the in-crate test covers; the Opus lane found it in round 1 and again in round 2.
2. [diff] R2 — The reference matcher treated `-` as a boundary, so a failed `data.foo` would also have skipped a resource using the healthy `data.foo-bar`, dropping resources that never touched the failure.
   - corrected: `src/core/resolver/data.rs:241` now counts `-` as a key character on both sides (`src/core/resolver/data.rs:237`), pinned by `src/core/resolver/tests_data.rs:446`.
