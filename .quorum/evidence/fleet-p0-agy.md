# Quorum evidence — 1.27.0 fleet P0 — agy lanes

Three review lanes through `quorum-review.sh --width 3 --executor agy`, each in its own standalone clone at 9ad01216, schema-enforced. Verdicts 3 FAIL, artifact `/run/user/1000/paiml-implement/agy/PMAT-212/quorum-PMAT-212.json`.

Two of the three findings reproduced when the orchestrator re-ran them and are fixed in `733804f4`. The third is answered in `fleet-p0-lanes.md`.

The two implementation phases were worked by `paiml-impl-worker` subagents in their own worktrees (PMAT-213 and PMAT-214), and every acceptance command they reported was re-run by the orchestrator before it was believed. That mattered once: the PMAT-214 worker reported its phase complete and flagged in its own receipt that the promotion lived in `plan_locks` only. Re-running the operator's sequence end to end showed the lock still recorded `status: failed` and a plain apply still latched — the fix was green and the defect was intact. The worker was right to flag it and right not to decide it alone; the decision to persist was the orchestrator's.

A CRUX lane (`--mode plan`) supplied the comparison in `docs/audits/crux-1.27.0.md`.
