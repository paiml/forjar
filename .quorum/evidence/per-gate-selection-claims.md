# PMAT-542 — the claims put to the round

The branch adds a per-gate selection to `scripts/ci/changed-class.sh` and splits
CI's `dogfood` job into `dogfood-surface` (build + gate C + gate D, on the
selection) and `dogfood-guards` (guard tests, on the class).

1. The selection is sound: gate C reads only the built binary and
   `docs/audits/surface_audit.csv`, gate D only the binary, `README.md` and that
   same CSV, so `tests/`, `scripts/` outside `scripts/dogfood/`, `docs/` and
   `.quorum/` cannot move either.
2. The allow-list fails CLOSED: a path nobody classified selects every gate.
3. The workflow wiring works as GitHub will actually run it — composite outputs
   propagate, the `if:` expressions are valid, and an unmeasured class or
   selection is refused rather than skipped.
4. Renaming the job breaks no required check and drops no step.
5. Each of the new cases dies if the change is reverted.
6. The receipt and the log describe what the commands produced, and the numbers
   in them are what the tools printed.

Each lane was given a different brief — the selection's soundness; the workflow
as GitHub runs it; the tests and the documents — and all three ended with the
standing instruction: *quote any sentence a reader could check and find false,
say what is actually true, and cite where you measured it.*
