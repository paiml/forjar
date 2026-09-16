# PMAT-574 — the lanes, and what each returned

This table is the ONE place this receipt counts rounds and names heads. A
number beside the word "round" anywhere else in this evidence is a QUOTATION of
a string a lane found stale, kept so the finding can be checked, not a claim
about how many rounds ran.

This table is the ONE place this receipt counts rounds and names heads. Each
round ran on the head that existed when it started, because a round that raises
a real finding produces a fix and the fix moves the head; the next round then
reviews a head no earlier round saw. No round was ever re-run to bury a finding:
every round that failed to agree either raised a finding adjudicated in
`.quorum/evidence/release-1.31.0-judges.md`, or collapsed because a lane
returned NOTHING.

The merge rail runs its own round on the FINAL head. By construction that round
cannot be described in a file it is reviewing, which is the honest limit of this
table.

| round | lane 1 | lane 2 | lane 3 | outcome |
|---|---|---|---|---|
| 1 | `gemini-3.1-pro-high` FAIL (1 finding) | `gemini-3.8-flash-high` NO-VERDICT | `gpt-oss-120b-medium` NO-VERDICT (503, no capacity) | not agreed |
| 2 | `gemini-3.1-pro-high` PASS | `gemini-3.8-flash-high` NO-VERDICT | `gemini-3.6-flash-high` PASS | not agreed |
| 3 | `gemini-3.1-pro-high` FAIL (6 findings) | `gemini-3.8-flash-medium` NO-VERDICT | `gemini-3.6-flash-high` PASS | not agreed |
| 4 | `gemini-3.1-pro-high` FAIL (2 findings) | `gemini-3.6-flash-high` PASS | `gemini-3.6-flash-medium` FAIL (line-number claims) | not agreed |
| 5 | `gemini-3.1-pro-high` FAIL (3 findings) | `gemini-3.6-flash-high` NO-VERDICT | `gemini-3.6-flash-medium` PASS | not agreed |
| 6 | `gemini-3.1-pro-high` PASS | `gemini-3.6-flash-medium` PASS | `gemini-3.6-flash-low` NO-VERDICT (503, no capacity) | not agreed |
| 7 | `gemini-3.1-pro-high` PASS | `gemini-3.6-flash-medium` NO-VERDICT (503, no capacity) | `gemini-3.6-flash-high` PASS | not agreed |
| 8 | `gemini-3.1-pro-high` FAIL (4 findings, all this evidence's own stale counts) | `gemini-3.1-pro-low` PASS | `gemini-3.6-flash-high` PASS | not agreed |
| 9 | `gemini-3.1-pro-high` FAIL (4) | `gemini-3.1-pro-low` PASS | `gemini-3.6-flash-high` FAIL (4) | not agreed — two lanes found the same stale counts |
| 10 | `gemini-3.1-pro-high` PASS | `gemini-3.1-pro-low` PASS | `gemini-3.6-flash-high` FAIL (3, all false: it claimed the receipt's evidence hashes were stale; every one matches byte for byte at HEAD, and its own byte-sum arithmetic was wrong) | not agreed |
| 11 | `gemini-3.1-pro-high` FAIL (2) | `gemini-3.1-pro-low` FAIL (3) | `gemini-3.6-flash-medium` FAIL (1) | not agreed — three more stale-count findings, all true, all fixed |

`gemini-3.8-flash-*` returned NO-VERDICT in every round it ran in (1, 2 and 3) — a SUCCESS
envelope carrying no verdict object each time. That is a property of the lane,
not of the branch, and it is why the round width kept collapsing to two.

Round 4 produced two findings worth answering and one repeat: gate R's "6 PR(s)
since v1.30.0" against the other gates' 5 (both right, different windows — the
six include #556, whose squash commit IS the tag), and a claim that the
CHANGELOG should cite the PR rather than the issue (refuted by the file's own
convention, `(PMAT-549, #549)` and four more like it). Its third was the
line-number claim two lanes now share and neither measured.

Round 3's lane 1 was the most valuable single review of any round here: six findings,
two of which refuted claims the author had written into the cut log and the
dogfood receipt (the after-composition that did not sum, and a false cause for
the 42-vs-43 gap). Both are fixed and recorded in
`.quorum/evidence/release-1.31.0-judges.md`; the other four are refuted there by
re-reading the files the lane cited.

Round 5's lane 1 caught the thing this file was getting wrong: its own opening
prose still said "three rounds" and named 203d8a65 as the final head while the
table beneath it and the receipt said four. Same for `release-1.31.0-agy.md` and
`release-1.31.0-claims.md`, which still described a single round. All three are
corrected, and the paragraph above says why the number moves.

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
validator's own words for the salvage path mark it as unreviewed claim
material, never a PASS.

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
