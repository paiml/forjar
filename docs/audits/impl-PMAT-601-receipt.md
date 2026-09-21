# Implementation receipt — PMAT-601 — impl receipts for the v1.32.0 window

verdict: PASS — writes the three harness receipts gate A found missing on the tag candidate 7bfb5720 (PMAT-592 for #593, PMAT-567 for #596, PMAT-598 for #599) and this one. Each records that its work was implemented directly with a quorum rather than through the paiml-implement harness, and cites only artefacts that exist: the quorum receipts, the lanes, the refutations, the RED proofs, and the positive-control runner names.

## How this was implemented — stated, not inferred

Directly, in a forjar session; a triage branch touching only paths on the
kind: triage rail (docs/audits/**, docs/roadmaps/roadmap.yaml, .quorum/**). Its
quorum receipt, `.quorum/PMAT-601-impl-receipts-v1.32.0.json` (kind: triage), is
produced BY the quorum that reviewed this file and is committed in this same PR —
a review lane correctly noted it did not yet exist when it read this sentence.

## Batched

Reviewed as its own branch (PR #602, quorum above) and merged batched into the
PMAT-566 PR, closing #602 unmerged, so a saturated fleet paid for one CI run
instead of two. The content is what that quorum reviewed.

## Measured on this branch, before push

    GATE A PASS 6 of 6 merged PR(s) since v1.31.0 carry a harness receipt
    GATE T PASS 9 tagged release(s) since v1.25.0 reconcile with git and GitHub and 59 ticket(s) carry their tag and say they shipped; 6 ticket(s) from 6 PR(s) merged since v1.31.0 carry release:v1.32.0; cut in flight: Cargo.toml is at 1.32.0

Two lanes refuted "gate A reports 6 of 6" because that number appeared nowhere
they could read. It was true, and it was uncheckable; it is now quoted here.

## Why honest absence and not reconstruction

Gate A asks that each receipt exist, end with its marker and carry exactly one
verdict line. It does not ask for harness fields, and inventing a route.sh
verdict or a dispatch ledger for work that never ran through the harness would
satisfy nothing the gate checks while putting a falsehood in the audit trail. The
receipts say what happened instead.

## Gaps

Gate A passing is necessary for the tag, not sufficient: gates E, T, F and H had
not run on the tag candidate when this branch was cut, because `make
dogfood-release` stops at the first failing gate.

IMPL-PMAT-601-RECEIPT-END
