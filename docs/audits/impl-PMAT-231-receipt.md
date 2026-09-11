# Implementation receipt — PMAT-231 — the closing record of the forjar 1.28.0 release, and the three defects shipping it exposed

verdict: PASS — `docs/audits/release-1.28.0-receipt.md` records the release against the live sources: the annotated tag on cdcc0e80, the published GitHub release with 14 assets, crates.io at `created_at 2026-09-10T16:12:45.938977Z`, docs.rs `doc_status: true`, gate R exit 0 and `make dogfood-published VERSION=1.28.0` green against the crate crates.io serves. Six lanes over two rounds refuted **five** facts in the drafts — a truncated timestamp, a miscounted cancellation, two roadmap rows a patching bug left null, a gate paraphrased rather than quoted, and a verdict line that flattened two by-hand steps into one — and all five are corrected. Three defects found while shipping are filed with their measurements: PMAT-232, PMAT-233, PMAT-234. Not claimed: that the release was published entirely by the workflow. It was not, and the receipt says which two steps a person took and why.

orch_model: opus [A]   orch_class: triage   orch_decision: admit   orch_basis: release
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1.quorum   class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate, three lanes on fcbc5b7e)
  ph1.quorum2  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate, three lanes on f2c5cb58)
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="scripts/dogfood/release-check.sh (gate R) on the published release, and make dogfood-published VERSION=1.28.0 (gates C and D against what crates.io serves)"  claimed_exit=0(lanes)  rerun_exit=0/0  log_path=docs/audits/logs/PMAT-231-release-check.log  sha256=recorded-in-the-log-header

## What this branch is

A record, and nothing else: one release receipt, four roadmap rows, one audit log. It is `kind: triage`, on the rail PMAT-226 widened. The release itself was cut by PMAT-226 and booked by PMAT-227; this is the third and last PR of the 1.28.0 release, and it exists because a release nobody wrote down is a release nobody can check.

## What shipping it exposed

| ticket | what | how it was found |
|---|---|---|
| PMAT-232 | `publish-release` un-drafts only when `needs.create-release.outputs.created == 'true'`, and the assertion that would catch it is inside the same guard — so a release whose creating run fails after `create-release` can never be published by the workflow | run 34530301814 reported `publish-release: success` while the release stayed `draft=true` |
| PMAT-233 | the `homebrew` job's tap push has an empty `GH_TOKEN` and dies on `remote: Invalid username or token` | PMAT-230's fixes carried the job past the checksums download and the leftover `/tmp/tap` for the first time, so the step ran at all for the first time |
| PMAT-234 | gate R's verdict line says `PASS pre-tag … PENDING until the tag is cut` whenever ANY arm is pending, so after every successful release the single line an operator is told to read states the opposite of the truth | the line printed on this very release, quoted in the release receipt |

All three carry `release:v1.29.0`, applied when they were minted. That is the property PMAT-225 built, and the v1.28.0 release is the first to exercise it end to end.

## Review record — two rounds, five corrections

| round | commit | lanes | what they refuted |
|---|---|---|---|
| 1 | fcbc5b7e | 0/3 PASS | crates.io's timestamp was truncated to whole seconds (2/3); "four jobs cancelled in the first tag run" was three, with four more in two other runs (3/3); PMAT-231's and PMAT-232's rows carried `notes: null` (3/3) |
| 2 | f2c5cb58 | 1/3 PASS | all ten claims confirmed by two lanes; **two lanes independently** answered the hostile-reader question with the same finding — the gate table paraphrased gate R instead of quoting it; **one lane** found the verdict line flattening two by-hand steps into one |

Every finding was re-run before it was acted on, and every one is corrected in the tree rather than argued away. One lane error is recorded rather than dropped: round 2 lane 3 reported that several of the new rows lack a `notes` field; at f2c5cb58 all four carry one.

The round-1 findings were about numbers. The round-2 findings were about a receipt describing itself more kindly than the record supports, and they are the ones worth the two rounds: the gate-table paraphrase would have left a reader believing gate R said something it did not, in a document whose only job is to be checkable.

**Two of the three `notes: null` rows also say something about the tooling.** The row patcher matched `notes: null\n`; `notes` is the last line of a roadmap row, so the newline belongs to the next row's start and the replacement silently did nothing while the surrounding edits succeeded. Recorded in `docs/audits/jidoka.jsonl` with the rule: assert each replacement individually.

## Gates measured

| gate | result |
|---|---|
| R, post-tag | exit 0. Its printed line is quoted in full in the release receipt, including the part that is false, with the reasoning that shows its arms passed (PMAT-234) |
| C and D on the published artifact | `make dogfood-published VERSION=1.28.0` exit 0 — 211 CLI names, 12 MCP tools, 12 HTTP verbs live in the crate crates.io serves; 18 documented invocations run; 98 cookbook configs validate |
| T | `GATE T PASS 6 tagged release(s) since v1.25.0 reconcile with git and GitHub and 26 ticket(s) carry their tag; 2 of 2 PR(s) merged since v1.28.0 carry release:v1.29.0; due 2026-09-12T16:07:14Z` |
| falsification | not applicable and stated as such: no code, no test, nothing reverted. The gate prints it as NOT VERIFIED |
| pre-commit | green on every commit; each carries `Pmat-Ticket: PMAT-231` |

## Gaps, named

- The release was not published entirely by the workflow. Un-drafting was done by hand because of PMAT-232; promotion from prerelease to full release is the operator's step by design, taken after `make dogfood-published` passed. The receipt separates the two, after a lane found it lumping them.
- PMAT-234 is quoted, not fixed: repairing gate R's summary is a code change and this is a triage branch.
- `homebrew` has still never published a formula for any release (PMAT-233).
- The next cut, v1.29.0, is due 2026-09-12T16:07:14Z. Eight roadmap rows already carry its label.

IMPL-PMAT-231-RECEIPT-END
