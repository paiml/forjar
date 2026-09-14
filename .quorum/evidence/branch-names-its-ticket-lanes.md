# PMAT-535 — the lanes

One round, three sandboxed agy quorum lanes, review-only (`writes=false`),
dispatched in the same message with `--not-before` pinned to the dispatch
instant. 29 findings.

| lane | brief | verdict |
|---|---|---|
| 1 (conv-63e3c4dc) | the rule itself: what it refuses, what it misses, its placement, its failure modes | do-not-implement-as-written |
| 2 (conv-92638a89) | the trailer reading and the tests | FAIL |
| 3 (conv-b70ff43e) | the receipt and the log as documents a reader will check | FAIL |

**Lane 3 could not run git.** The review repository was a `git clone --shared`,
whose `objects/info/alternates` points into the parent repository — outside the
lane's sandbox. Every `git log`/`git show` returned
`error: object directory … does not exist`, and the lane marked three of its ten
findings UNMEASURED rather than guessing. That is the right behaviour and the
wrong setup: a sandboxed lane needs a full clone, not a shared one. The lane
still found the receipt's false sentence from the files alone.

The orchestrator's own subagent was cut off by a usage limit before it wrote its
receipt. All three lane JSONs had already been written and were read directly;
nothing was inferred from the delegate.
