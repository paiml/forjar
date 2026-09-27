# Lanes — a mount's ownership options are checked state (PMAT-642)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

Every lane ran through `quorum-review.sh --executor agy` in the configured canary
shape, one model per lane. Each lane was handed the ticket, the claims and the
full diff against origin/main, and no lane builds. The author is claude-opus-5-5.
Lane 2 is configured as claude-opus-4-6-thinking; the fallback chain skipped it
as author-family and it was judged by gemini-3.1-pro-high in its own
conversation. Lane 3 is claude-haiku-4-5, which is a different model id from the
author's: a degraded same-family lane, recorded and not hidden.

| round | head | lane 1 | lane 2 | lane 3 |
|---|---|---|---|---|
| 1 | 6c1cabf2 | agy gemini-3.1-pro-high FAIL (R1) | agy gemini-3.1-pro-high (fallback) FAIL (R1) | claude-haiku-4-5 PASS |
| 2 | 0b5882e0 | agy gemini-3.1-pro-high PASS | agy gemini-3.1-pro-high (fallback) PASS | claude-haiku-4-5 PASS |

Round 1 refuted the first head 2/3 on R1: an omitted key matched any declared
value. Commit 0b5882e0 compares an omitted key as its kernel default and adds a
test for it. Round 2 judged that head 3/3, and its verdicts are the ones this
receipt's `quorum.lanes` counts.
