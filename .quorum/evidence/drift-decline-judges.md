# PMAT-564 — adjudicated claims

One round of three sandboxed agy quorum lanes: 1 FAIL, 2 PASS, not agreed.
Seven confirmations and four refutations, every one re-measured on this host
before it was acted on. The decline held; the branch's claim that EVERY
detector honoured the scope did not, and one point two lanes agreed on was
wrong.

## CONFIRMED

1. [n-from-config] That the declared count N comes from the config's
   resources targeting the scanned machines and never from the lock's
   `in_scope` (all three lanes).
   - evidence: `src/cli/drift.rs:413` builds N from `cfg.resources`,
     filtered to non-recipe resources whose `machine` names a scanned
     machine; a `machine: [a, b]` resource with only `a` scanned counts once;
     `-m` narrows the scanned set and therefore N. The lock is never read
     for it, which is the whole point: the lock's `in_scope` on yoga was 103
     and the manifest's count was 10.

2. [unmeasured-outranks] That an unmeasured resource keeps forjar#549's exit
   4 rather than being declined (lanes 1 and 2 measured; lane 3 read).
   - evidence: `src/cli/drift.rs:433` returns before declining when
     `total_unmeasured > 0`; mutation M3 removes that guard and two
     FALSIFY-549 cases go red with exit 2 in place of 4 — which is exactly
     the first cut of this branch, caught by the suite before any lane saw it.

3. [no-exit-0] That no run with inspected == 0 and declared > 0 exits 0
   outside `--dry-run` (all three lanes).
   - evidence: the lockless path with `--no-task-checks` declines
     (`tests/falsification_drift_without_a_lock_measures_the_host.rs:218`),
     a guard the lock never heard of declines
     (`tests/falsification_drift_is_not_blind_to_task_guards.rs:236`), `--json`
     declines with the same code
     (`tests/falsification_drift_declines_on_empty_scope.rs:233`). `--dry-run`
     previews and returns before the scan, which every lane named and none
     called a defect: a preview has no verdict to give.

4. [no-config-unchanged] That the no-config path still grades every lock
   entry (all three lanes).
   - evidence: `detect_drift_impl` in `src/tripwire/drift/file.rs` calls the
     scoped detector with `declared_only = false`, so the guard at
     `src/tripwire/drift/file.rs:246` is inert on the no-config path.

5. [message] That `no lock holds them` is printed only when every skip reason
   for the declared resources is absence from the lock, and mixed reasons
   are named (all three lanes).
   - evidence: `src/cli/drift.rs:448`; the opted-out case prints
     `skipped: --no-task-checks` and the test asserts the flag is named.

6. [control] That a partially locked manifest is graded, not declined, and
   its tampered file is a reject (lanes 2 and 3 measured).
   - evidence: `tests/falsification_drift_declines_on_empty_scope.rs:196`;
     mutation M4 (decline whenever declared > 0) kills exactly this case and
     the fully-inspected control.

7. [falsifiers] That each contract falsifier's mutation turns its cited test
   red (lanes 1 and 3 reasoned; run here).
   - evidence: M1 through M5 in the pmat digest, each restored from HEAD.

## REFUTED

1. [every-detector] That "a locked resource the config does not declare is
   skipped as `in the lock, not in the config` by every detector" (this
   author, `CHANGELOG.md:10` as first written, and the contract's
   `never graded` invariant).
   - corrected: lane 1, the only lane that read
     `src/tripwire/drift/image.rs`, found the image detector walking the
     lock with no config check at `src/tripwire/drift/image.rs:36`, and
     `census.inspected` overwriting a skip. The guard is there now, with
     `an_undeclared_locked_image_is_skipped_as_not_in_config` in `src/tripwire/drift/tests_image_drift.rs` failing without it (M5),
     and the contract names every detector and the census rule that makes
     the guard necessary.

2. [pre-expansion] That N counts `count:`/`for_each:` resources
   pre-expansion (lanes 1 and 3 — two resamples of one model, agreeing).
   - corrected: `load_drift_config` calls `parse_and_validate`, which
     expands recipes, `count:` and `for_each:` before the config reaches
     `cmd_drift`, so `src/cli/drift.rs:413` counts expanded resources. Lane
     2 said so; two lanes sharing a model id were wrong together, which is
     why `partial_reasons` records the duplicate.

3. [first-cut-precedence] That the first cut of the decline was correct
   (this author, before running the neighbouring suites).
   - corrected: it declined whenever inspected == 0, and
     `falsification_drift_unmeasured_is_not_drift` went red on two cases —
     an unreachable host was "nothing to measure" instead of "could not
     measure". The unmeasured guard at `src/cli/drift.rs:433` was added and
     the two cases went green untouched; three older cases that asserted
     exit 0 over `inspected 0 of N` were inverted to the decline they had
     always described.

4. [no-writes] That every lane left its clone byte-identical (the brief's
   NO WRITES instruction).
   - corrected: lane 1 wrote a three-line `test_image_skip.rs` into its own
     clone; `agy-lane.sh` kept the clone and said so. Isolation held — the
     shared checkout was clean at `bd1664a8` — and the clone was removed
     after the finding it produced was acted on. Named because a lane that
     writes where it was told only to read is a finding about the lane, not
     a detail.
