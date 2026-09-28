# Lanes — forjar 1.33.0-rc.1 release PR (PMAT-651)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

Every lane ran through `quorum-review.sh --executor agy --shape default` with
lane models gemini-3.1-pro-high, claude-sonnet-5 and claude-haiku-4-5, the
diff against base f281f5ad, and no build. The author is claude-opus-5-5, so no
lane runs the author's model id. The Claude lanes are a degraded same-family
review with distinct model ids, recorded and not hidden. From round 2 on, the
diff touched no tier-1 path, so policy withheld the gemini lane and the
fallback chain judged it with claude-sonnet-5 in its own conversation.

| round | head | ticket | lane 1 | lane 2 | lane 3 |
|---|---|---|---|---|---|
| 1 | 02acddfd | PMAT-648 | agy gemini-3.1-pro-high FAIL (R1) | claude-sonnet-5 FAIL (R1, R2) | claude-haiku-4-5 PASS |
| 2 | 2ab9f59d | PMAT-648 | claude-sonnet-5 (gemini withheld) PASS | claude-sonnet-5 FAIL (R3) | claude-haiku-4-5 NO-VERDICT |
| 3 | d1c5b54a | PMAT-651 | claude-sonnet-5 (gemini withheld) PASS | claude-sonnet-5 PASS | claude-haiku-4-5 PASS |

Round 1 refuted the first head on R1 (lazy detach under automount, fixed in
d07790c8 with a falsifying test) and R2 (the rc-tag workflow change, reverted
in 2ab9f59d). Round 2 refuted scope on R3: the #647 hunk sat under the PMAT-648
ticket. d1c5b54a adds the release ticket PMAT-651 (forjar#651) that names both
fixes, and round 3 judged that head 3/3. Its verdicts are the ones this
receipt's `quorum.lanes` counts.
