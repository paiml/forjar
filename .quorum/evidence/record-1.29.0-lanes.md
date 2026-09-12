# PMAT-533 — the lanes

One round, three sandboxed agy quorum lanes, width 3, `writes=false`, on the
record diff with `--not-before` pinned to the dispatch instant.

| lane | conversation | exit | duration | verdict |
|---|---|---|---|---|
| 1 | conv-35894adf | 0 | 378s | FAIL |
| 2 | conv-b9b58ad0 | 0 | 337s | FAIL |
| 3 | conv-b02acaec | 0 | 334s | FAIL |

## The instruction, for the third time

Three rounds in this release run have carried the same instruction above their
numbered claims: **quote any sentence a reader could check and find false.**
Three times it has produced every finding that mattered — eight sentences in
the cut, two false records in the booking, five here.

The numbered claims are useful for pointing lanes at surfaces. The instruction
is what finds the defects, because a record fails by drifting a word or a count
away from what the tree says, and no claim list anticipates which word.

## Two lanes were right and one was righter

All three refuted the Gate R quote and the step count. On PMAT-520, two lanes
ran one command — comparing #527's merge commit against
`git rev-parse v1.29.0^{commit}` — and found the record asserting something git
contradicts in a single line. That correction is what opened PMAT-535, which is
a defect in a rule three gates share.

## What they could not check

Two lanes reported `docs.rs` UNCHECKED (one got a 404 from its sandbox) and two
could not run `make dogfood-published`. Both were measured here instead:
`doc_status: true`, and the target exits 0 with gates C and D green against what
crates.io serves. A lane saying "I could not check X" is worth more than a lane
confirming it, and all three said it.

## The no-write rule

The brief opened with it and named the three earlier rounds in this session
whose lanes wrote into the repository root. The tree was clean afterwards.
