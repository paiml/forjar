# PMAT-564 — the claims put to the round

The branch makes `forjar drift` decline — exit 2, the count named — when it
inspected none of the resources the manifest declares, and stops every
detector from grading a locked resource the manifest does not declare.
Measured on yoga under 1.30.0: `drift -f forjar-ephemeral.yaml` inspected 0
of that manifest's 10 resources, graded two files from `forjar.yaml`, and
exited 0 (paiml/infra#605, first signature).

Three lanes, read-only, sandboxed, against self-contained clones of the PR
worktree at `bd1664a8`, diffed against the base branch
`PMAT-562-drift-exits-on-drift` (PR #563, reviewed separately), dispatched in
one message with `--not-before` pinned and `out_dir` keyed by ticket AND
session id. Lane models were named in the brief: gemini-3.1-pro-high twice
and gemini-3.7-flash-high once, because gemini-3.8-flash-high had returned
503 on both earlier rounds that day.

The questions, identical to every lane:

1. The declared count N: is it ever the lock's `in_scope`? What does it count
   for `machine: [a, b]` with only `a` scanned, under `-m`, under
   `--all-stacks`, and for `count:`/`for_each:` — pre- or post-expansion?
2. Precedence: drift findings with inspected == 0 — reachable? Unmeasured
   with inspected == 0 must exit 4, not 2.
3. Any way a run with inspected == 0 and declared > 0 still exits 0 — the
   lockless path with `--no-task-checks`, `--dry-run`, `--json`?
4. Does the no-config path still grade every lock entry? Can a later
   detector `inspected` an id an earlier one skipped as not-in-config?
5. The contract and CHANGELOG: quote any false sentence; does each
   falsifier's mutation turn its cited test red?
6. Is `no lock holds them` true every time it is printed?

The acceptance command every lane was told to run covered six suites:
`falsification_drift_declines_on_empty_scope`, `_unmeasured_is_not_drift`,
`_without_a_lock_measures_the_host`, `_is_not_blind_to_task_guards`,
`_scopes_to_the_config_it_was_given`, `falsification_contract_citations_resolve`.
