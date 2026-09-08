# Implementation receipt — PMAT-214 — a task whose command exits non-zero latched, and --refresh could not free it; now it re-checks a failed entry and writes down what it measured

verdict: DONE — `--refresh` re-runs the completion_check for an entry the lock records as FAILED and PERSISTS the promotion, so one run is a way back: measured end to end, `1 FAILED` then `--refresh` then a plain apply green with the always-failing command run exactly once in total. Closes #487's first half; its second half is recorded as still open.

## Identity

| field | value |
|---|---|
| ticket | PMAT-214 (kind: code) |
| issue | #487 |
| branch | PMAT-214-refresh-unlatches, merged into release-1.27.0-fleet-p0 |
| worker | `paiml-impl-worker` (opus), resumed once at its turn limit; every claim re-run by the orchestrator |

## The latch

A `type: task` may legitimately have a command that always fails. It is the pattern forjar itself makes necessary: registering a GitHub Actions runner needs an ephemeral token that cannot live in a config, so the resource's job is to refuse loudly and name the make target rather than report converged on a box with no runner.

That works only while forjar SKIPS the resource. The generated script is command-then-check, so the moment the lock records a failure, every later apply re-runs a command that cannot succeed, never reaches the check, and re-records the failure. With `policy.failure: stop_on_first` it takes its dependents with it; on gx10 it hid three other resources.

## The first fix was a half fix, and the measurement caught it

The worker made `--refresh` re-check failed entries, and flagged in its own receipt that the promotion lived in `plan_locks` only. That flag was correct and it mattered. Re-run end to end against the real binary:

```
apply             ->  0 converged, 0 unchanged, 1 FAILED
lock              ->  status: failed
apply --refresh   ->  0 converged, 1 unchanged, 0 failed
lock              ->  status: failed        <-- nothing written down
apply             ->  0 converged, 0 unchanged, 1 FAILED
```

`--refresh` was green and the latch was intact. #487 asks for a way BACK; that sequence is still "there is no documented way back", one command later. After the second commit:

```
apply             ->  1 FAILED
lock              ->  status: failed
apply --refresh   ->  0 converged, 1 unchanged
lock              ->  status: converged
apply             ->  0 converged, 1 unchanged
the command ran ONCE in total
```

## What crosses, and what does not

Only PROMOTIONS: an entry present in both the lock and the refreshed view that the lock recorded as not converged and the refreshed view records as converged. That transition can only come from `unlatch_failed`, which already demands the resource be in scope, be declared for that machine, and have had its check run on the host and exit 0 — the same evidence seeding demands.

Evictions do not cross: an eviction is how the planner is told to re-apply a resource, not a decision to forget an entry. Seeds do not cross: an entry that never existed keeps the behaviour it has always had.

## Falsification

`tests/falsification_refresh_unlatches_a_failed_task.rs`, five cases through the real binary against a state dir inside the test's own tempdir. Two are negative and pin the boundary: a FAILING completion_check still runs the command and still reports failure, so this is not a way to launder a broken resource into converged.

Two mutations, both killed. Commenting out the `unlatch_failed` call fails `refresh_rechecks_a_resource_the_lock_records_as_failed` with forjar's own `0 converged, 0 unchanged, 1 failed` — the operator's yoga output verbatim. Commenting out the `persist_unlatched` call fails only the durability case, with "--refresh measured the resource converged and did not write it down", and leaves the other four green, so it is that one call the case measures.

## Gaps

- #487 offered two fixes and said either alone is sufficient; option (1) is taken. Option (2), `drift` evaluating resources the lock records as not converged rather than skipping them, is NOT done. The operator's own measurement of it — `skipped 10: ... not converged in the lock 3` — still reproduces, and it means drift stays blind to precisely the resources the lock believes are broken. Worth its own ticket.
- The operator's manifest workaround (make the command idempotent) remains defensible on its own merits and is not obsoleted by this fix.

IMPL-PMAT-214-RECEIPT-END
