# PMAT-542 — the lanes

One round, three sandboxed agy quorum lanes, review-only (`writes=false`),
dispatched in one message with `--not-before` pinned to the dispatch instant.
31 findings.

| lane | brief | verdict |
|---|---|---|
| 1 (conv-14b4704d) | is the selection sound — find a path it calls harmless that can move a gate | FAIL |
| 2 (conv-295ed1ee) | the workflow as GitHub will actually run it | FAIL |
| 3 (conv-a84090fe) | the tests and the documents, checked number by number | FAIL |

The review repository was a FULL standalone clone this time, not a
`git clone --shared`. The previous round's lane 3 could run no git command at
all because a shared clone's `objects/info/alternates` points outside the
sandbox; every lane here ran `git log`, `git show` and `git diff` without
trouble, and lanes 1 and 2 independently measured the same hole.

The orchestrator's own subagent hit its 30-turn limit before writing a receipt —
the tenth time in this session. All three lane JSONs were already on disk with
`exit 0` and were read directly; nothing was taken from a delegate summary that
was never written.
