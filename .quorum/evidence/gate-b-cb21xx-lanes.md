# PMAT-521 — the lanes

One round, three sandboxed agy quorum lanes, width 3, `writes=false`, on the
diff at `72d11508` against `origin/main`, `--not-before` pinned to the dispatch
instant.

| lane | conversation | exit | duration | verdict |
|---|---|---|---|---|
| 1 | conv-7e0f92e2 | 0 | 509s | FAIL |
| 2 | conv-7e1df0bf | 0 | 687s | FAIL |
| 3 | conv-bb34c7af | 0 | 443s | FAIL |

All six claims refuted. The brief named the six things to attack in order —
is anything asserted less than before; can the exemption widen by accident;
does the arm fail closed in every unmeasurable direction; are the recorded
numbers true; do the nine cases falsify; is CB-2113 correctly excluded — and
five of the six produced a finding that changed the code.

The brief also told the lanes this host was inside a GitHub secondary rate
limit and to report a GitHub-dependent check as a finding rather than retrying.
All three did, which is why claim 4 reads as unmeasurable rather than as three
lanes silently confirming numbers they could not see.

Lane 2 alone confirmed claims 1, 2 and 3 before refuting 4, 5 and 6; lanes 1
and 3 refuted 1 and 2 as well, on the duplicate-id overwrite and the
exempt-but-unowned hole. The split is recorded as it fell. What decides each
one is that the refutation reproduces, and all six did.

## The no-write rule

The brief opens with it. The repository was verified clean afterwards:
`git status --porcelain` showed only the orchestrator's own untracked receipt
and log, and HEAD was still the commit the round was dispatched on.
