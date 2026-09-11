# PMAT-522 — the lanes, and the round that was nearly skipped

One round, three sandboxed agy quorum lanes, width 3, `writes=false`, on the
diff at `8ef9f844` against `origin/main`, `--not-before` pinned to the dispatch
instant.

| lane | conversation | exit | duration | verdict |
|---|---|---|---|---|
| 1 | conv-01fd646a | 0 | 494s | FAIL |
| 2 | conv-65ec47aa | 0 | 339s | FAIL |
| 3 | conv-daac3694 | 0 | 317s | FAIL |

## The round was nearly skipped, and the gate refused that

The first receipt for this ticket carried ONE "lane": a pointer to a document
explaining why no round had been dispatched. The reasoning was that the defect
was already measured by the operator, on the machine it happened to, with
numbers no lane could produce from a diff; and that three lanes each spawn
processes on a box recovered by hand from a fork storm twenty minutes earlier.

**The quorum gate refused it**: `quorum had 1 evidence lanes, floor is 3`. It
was right to. That reasoning is an argument for skipping a rule, made by the
person the rule constrains, about a change that person had just written — and
the round, once it ran, found three defects that argument would have shipped:

1. the process cap FAILED OPEN when `ps` could not answer;
2. the CB-2115 ceiling had been lowered to a number the committed tree does not
   measure, so gate B was RED on this very branch;
3. the cap's own test asserted on the script's TEXT rather than its behaviour.

The brief carried one instruction no other round in this window needed: **do
not run `pmat comply ratchet` and do not create a `.pmat-ratchet.toml`**, with
the safe way to exercise the guard named instead. All three lanes respected it;
load stayed at 5.

## What the lanes did anyway

Two of them wrote `pmat_output.json`, `stdout.log` and `stderr.log` into the
repository root, despite the no-write rule opening the brief. Removed by the
orchestrator and recorded here rather than left for someone to find. This is
the second incident of lane writes in this repository's history and the brief
already opened with the rule, so the rule is not the missing part.
