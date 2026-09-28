# Lanes — forjar 1.33.0 release PR (PMAT-652)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

Every lane ran through `quorum-review.sh --executor agy --shape default` with
lane models gemini-3.1-pro-high, claude-sonnet-5 and claude-haiku-4-5, the
diff against base 2fd44833 (v1.33.0-rc.1), ticket PMAT-652, and no build. The
author is claude-opus-5-5, so no lane runs the author's model id. The two
Claude lanes are a degraded same-family review with distinct model ids,
recorded and not hidden. The diff touches a workflow (tier-1), so the gemini
lane ran in both rounds.

| round | head | ticket | lane 1 | lane 2 | lane 3 |
|---|---|---|---|---|---|
| 1 | 9aee0703 | PMAT-652 | agy gemini-3.1-pro-high FAIL (R1) | claude-sonnet-5 PASS | claude-haiku-4-5 PASS |
| 2 | a5374809 | PMAT-652 | agy gemini-3.1-pro-high PASS | claude-sonnet-5 PASS | claude-haiku-4-5 PASS |

Round 1 refuted scope on R1: the workflow hunk carried a comment line beside
the one line the ticket allows. a5374809 removes it and round 2 judged the
result 3/3 PASS.
