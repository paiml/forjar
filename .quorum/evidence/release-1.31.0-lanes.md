# PMAT-574 — the lanes, and what each returned

Three rounds were run. Round 1 reviewed head 203d8a65 (before the quorum
receipt existed); rounds 2 and 3 reviewed the final head e65a1fad. No round
agreed, and each round's collapse was a lane returning nothing, never a lane
returning a finding that was then ignored.

| round | lane 1 | lane 2 | lane 3 | outcome |
|---|---|---|---|---|
| 1 | `gemini-3.1-pro-high` FAIL (1 finding) | `gemini-3.8-flash-high` NO-VERDICT | `gpt-oss-120b-medium` NO-VERDICT (503, no capacity) | not agreed |
| 2 | `gemini-3.1-pro-high` PASS | `gemini-3.8-flash-high` NO-VERDICT | `gemini-3.6-flash-high` PASS | not agreed |
| 3 | `gemini-3.1-pro-high` FAIL (6 findings) | `gemini-3.8-flash-medium` NO-VERDICT | `gemini-3.6-flash-high` PASS | not agreed |

`gemini-3.8-flash-*` returned NO-VERDICT in all three rounds — a SUCCESS
envelope carrying no verdict object each time. That is a property of the lane,
not of the branch, and it is why the round width kept collapsing to two.

Round 3's lane 1 was the most valuable review of the three rounds: six findings,
two of which refuted claims the author had written into the cut log and the
dogfood receipt (the after-composition that did not sum, and a false cause for
the 42-vs-43 gap). Both are fixed and recorded in
`.quorum/evidence/release-1.31.0-judges.md`; the other four are refuted there by
re-reading the files the lane cited.

## Round 1 in detail

Round 1, head 203d8a65 against `main` at 9884334a. Three sandboxed `agy` lanes,
review-only, `writes=false`, three distinct model ids, none in the author's
family.

| lane | model | envelope | verdict | findings |
|---|---|---|---|---|
| 1 | `gemini-3.1-pro-high` | SUCCESS | FAIL | 1, cited, at `docs/audits/impl-PMAT-574-receipt.md:1` |
| 2 | `gemini-3.8-flash-high` | SUCCESS | NO-VERDICT | no verdict object in the lane file |
| 3 | `gpt-oss-120b-medium` | ERROR | NO-VERDICT | `UNAVAILABLE (code 503): No capacity available for model gpt-oss-120b-medium` |

Two lanes returned nothing usable, and this file records that rather than
rounding it to "the round was fine". A NO-VERDICT is not a PASS: the receipt
validator's own words for the salvage path are `[UNREVIEWED CLAIM MATERIAL —
never a PASS]`.

## Lane 1's finding, and what happened to each half

> The diff adds the implementation receipt itself to the repository as
> `docs/audits/impl-PMAT-574-receipt.md`. The ticket does not request this file,
> and the receipt's own enumeration of "The diff" explicitly lists the files it
> changed but completely omits this file.

**CONFIRMED, and fixed:** the receipt's `## The diff` section did not name the
receipt doing the listing, nor the quorum receipt and evidence this round
produces. A file list that omits itself is exactly the class of incomplete
record this repository refuses elsewhere. Both are listed now.

**REFUTED:** the file is not an unrequested addition.
`scripts/dogfood/harness.sh` (gate A) fails any merged PR whose ticket has no
`docs/audits/impl-<ticket>-receipt.md` at HEAD ending in
`IMPL-<ticket>-RECEIPT-END` with exactly one `verdict:` line. The receipt is
required by the release gate the lane could not run — which is the standing
limit of a diff-reading lane on a release cut, stated in
`release-1.31.0-agy.md`.

The lane earned its keep either way: it read the receipt as a document with
claims in it, which is what a hostile reader is for.
