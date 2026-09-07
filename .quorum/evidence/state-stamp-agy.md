# Independent review — agy /teamwork-preview — PMAT-161

A 25-minute attempt timed out with no verdict; the 35-minute retry (conv-8e786043, 1245 s) reviewed the diff at 41c0eac2 without seeing the claim lanes, and a plan-mode fallback on the same hunting prompt (conv-a4f3a84b, 717 s) ran beside it. A plan-stage teamwork run (conv-aea0ebd6) had grilled the design before phase 1 and forced the machine-ownership guard, per-stack outputs and per-stack status attribution into the ticket.

## Verdict (do-not-implement-as-written)

The branch successfully implements multi-stack state locks, safe 1.0 schema migrations, upfront multi-stack restore refusal, and preserves byte-identical single-stack status output. However, it contains two critical logic flaws that will corrupt state: a path-collapse bug in rename retirement (misidentifying config files due to flawed `ParentDir` stripping), and a machine-dropping defect during scoped applies (failing to union unselected machines and bypassing ownership guards).

A standard automated claim lane would have run existing tests, seen green passes, and issued a false-positive approval. The agent team actively uncovered the path-collapse bug via component stripping analysis, exposed the scoped apply machine dropping via written POC tests, and validated architectural invariants to ensure state safety.

The full deliverable report is available at: `/tmp/lanes-161q1-344044/DOCUMENT_REVIEW_REPORT.md`

## Findings, as returned

- T1 [cited] src/cli/undo.rs:305 — `refuse_multi_stack_restore` strictly precedes all staging, pruning, and generation snapshots, preventing cross-stack corruption during restores.
- T2 [cited] src/core/state/stamp/mod.rs:202 — `stamp::migrate` correctly initializes legacy 1.0 schema entries with `file: None` and provides a one-apply grace period without spurious warnings.
- T3 [cited] src/core/state/stamp/mod.rs:262 — CRITICAL: `same_config_file` incorrectly strips `ParentDir` components, causing relative paths like `../forjar.yaml` to collapse into a wildcard matching any relative `*/forjar.yaml`, triggering wrong-file rename retirements.
- T4 [cited] src/core/state/stamp/rename.rs:58 — HIGH-SEVERITY: Scoped apply (`apply --machine`) drops untargeted machines from the state stamp because `next_stamp` does not union `previous.machines`, allowing sibling stacks to bypass the ownership guard.
- T5 [cited] src/cli/status_core.rs:131 — Multi-stack summary blocks are strictly gated by `stacks.len() > 1`, ensuring single-stack directories remain byte-identical to pre-PMAT-161 output.
- T6 [cited] docs/roadmaps/roadmap.yaml:1882 — The code correctly implements the widened criteria (upfront restore refusal), but the roadmap backlog retains stale, contradictory pre-pivot text about per-stack undo.

## Plan-mode fallback (FAIL)

I have reviewed the branch `PMAT-161-state-stamp-per-name`. While the multi-stack refusal logic correctly protects `undo_prune` and `undo --resume`, I found that `--rollback-on-failure` executes the entire apply process (writing to the state dir and destroying resources) before it fails and reaches the refusal in `rollback_to_generation`. Additionally, generation snapshot GC silently deletes snapshots belonging to other stacks in a shared state dir. Older forjars will also silently erase the `stacks` map when saving a `1.1` lock, and `retire_renamed` leaves duplicate renames behind by using `.find()`. The machine ownership guard, migration logic, and status outputs are implemented correctly.

Findings:
- U1 [cited] src/cli/apply.rs:174 — --rollback-on-failure writes to the state dir before the refusal
- U2 [cited] src/cli/apply_snapshot.rs:118 — generation snapshots (gc_old_snapshots) deletes snapshots from other stacks in a shared dir
- U3 [asserted] src/core/types/state_types.rs:1 — Older forjars will silently erase the stacks map because they lack check_schema and deserialize 1.1 as 1.0
- U4 [cited] src/core/state/stamp/rename.rs:33 — retire_renamed uses .find() which only retires the first matching entry, leaving duplicates behind

## What became of it

Every defect the two lanes raised is fixed in the diff (D1–D5, D7, D9, D10 in the claims dossier: 00af3c08, be1309f5, 1c6d0d64, 22a359f7, 6c5d9416, 507b6103) or rejected with its rationale (D6, the documented downgrade limit). The teamwork lane's executed reproducer for the path-suffix collision is the finding that most changed the branch; its --rollback-on-failure ordering finding is the one three lanes reached independently.
