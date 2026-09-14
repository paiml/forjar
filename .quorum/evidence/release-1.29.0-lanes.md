# PMAT-520 — the lanes

One round, three sandboxed agy quorum lanes, width 3, `writes=false`, on the
diff at `3875e2de` against `origin/main`, `--not-before` pinned to the dispatch
instant.

| lane | conversation | exit | duration | verdict |
|---|---|---|---|---|
| 1 | conv-b7a1263d | 0 | 367s | FAIL |
| 2 | conv-925ff699 | 0 | 478s | FAIL |
| 3 | conv-6f895692 | 0 | 389s | FAIL |

## The instruction that earned the round

Six numbered claims were put to them, and one standing instruction above the
six: **find a sentence in the CHANGELOG, the crux audit or the dogfood receipt
that a reader could check and find false.**

Four of the five findings came from that instruction rather than from the
numbered claims. A release record is prose, and prose fails in a way a claim
list does not anticipate: not by being unsupported, but by drifting a word or a
count away from what the tree says. The count was wrong by one PR. The README
was a minor behind. A crux row named the wrong object. The receipt generalised
a precise gate line into a false sentence.

None of those is a design defect and none would have failed a gate. All four
would have been in the permanent record of what 1.29.0 was.

## The briefing errors, recorded

The brief told the lanes the receipt "will name" the crux document as the
reverted hunk, without saying WHICH receipt. All three looked in the dogfood
receipt, which by its own skill's definition carries no falsification block,
and all three reported its absence as a finding. That is a brief defect, not a
lane error, and the fix is in the brief rather than in the receipt.

## The no-write rule

The brief opened with it and named the two lanes in an earlier round this
session that wrote log files into the repository root. The repository was
verified clean afterwards: only the orchestrator's own untracked log, and HEAD
still on the commit the round was dispatched on.
