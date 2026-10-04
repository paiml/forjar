# Lanes — the nightly's Windows leg builds OpenSSL with Strawberry perl (PMAT-614)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

This is a workflow PR, so the cop's ruling was three agy lanes at the final
head. Every lane ran through `quorum-review.sh --executor agy`, one model per
lane, and was handed the ticket, the claim and the full diff against
origin/main. No lane builds. The author is claude-opus-5-5, and no lane ran a
Claude model.

| round | head | lane 1 | lane 2 | lane 3 |
|---|---|---|---|---|
| 1 | f78aef43 | agy gemini-3.1-pro-high FAIL conv-a860e26f (R1) | agy gemini-3.1-pro-low FAIL conv-00cc8d7d (R1) | agy gemini-3.8-flash-high FAIL conv-e3f25343 (R1, R2) |
| 2 | 215f2e8a | agy gemini-3.1-pro-high PASS conv-ff084b7c | agy gemini-3.1-pro-low PASS conv-e86bb334 | agy gemini-3.8-flash-high PASS conv-95ee054f |

Round 1 refuted the branch unanimously on the roadmap: its first commit carried
pmat's YAML round-trip of `docs/roadmaps/roadmap.yaml`. Commit 215f2e8a restored
main's file verbatim and appended PMAT-614 as text. Round 2 judged that head,
and its three verdicts are the ones this receipt's `quorum.lanes` counts. The
round-2 "findings" were confirmations of C1, C4 and C5, with no defects.
