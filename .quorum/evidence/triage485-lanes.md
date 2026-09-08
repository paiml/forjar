# Quorum evidence — PMAT-210 — lane rulings

Two review rounds, each three independent lanes in standalone clones, plus one `/teamwork-preview` and one CRUX survey.

## Round 1 at a2a6876a — 3 lanes, all FAIL, and the finding was about the FRAME

All three lanes returned the same finding: the diff does not implement PMAT-209's acceptance criteria — no code, no fixture, no reason-string change. They were right about the text they were given. `quorum-review.sh` reads `pmat work status <ticket>` and puts that ticket beside the diff, and the ticket it was handed was PMAT-209, whose acceptance criteria describe the FIX.

The diff was never a fix. The correction was to the frame, not to the diff: the triage is its own ticket, PMAT-210, `kind: triage`, and PMAT-209 is what the triage MINTS. Re-run against PMAT-210, the same three lanes returned PASS, PASS, PASS at 00f973f6.

That is a refutation worth keeping in the record rather than tidying away. A branch whose commit message says "deferred" and whose ticket says "fix this" is a branch that reads as unfinished work to anyone who does not already know the story, and three independent readers proved it in the only way that counts.

## Round 2 at 00f973f6 — 3 lanes, all PASS

Judged against PMAT-210: the disposition, the row, the receipt and the issue comment, with no `src/` change claimed and none present.

## `/teamwork-preview` — PASS, and the sharpest finding of the branch

Two judgments were asked for. On the disposition it agreed with deferral and killed the alternatives by name: yanking punishes users who just pinned 1.26.0 for a defect that predates it; a same-day 1.26.1 buys a rushed patch against a defect the fleet has carried since at least 1.25.2; holding the board open is not available, because the release is already tagged, published and pinned.

On the evidence it refused the narrowing, and it was right:

> Without the 1.25.2 local probe, the narrowing is a guess disguised as a measurement.

Probes A and B ran on 1.26.0 only. They cannot separate "the local path is immune" from "1.26.0 already fixed what 1.25.2 got wrong", and those two readings send the next person to opposite places — one to the remote read-back, the other to a changelog. Probe C was added: the same three commands on the 1.25.2 release binary, provenance checked against that tag's `SHA256SUMS`. It also reports `1 converged`, then `0 converged / 1 unchanged`, then `No drift detected`. The local path is immune in the reported version too, the pin did not silently fix the fleet, and the narrowing now rests on a control instead of on an absence.

## CRUX — PASS

Surveyed Ansible, Puppet/Chef, Terraform and nixpkgs on what happens to an S1 filed against an already-published release. Its one finding: forjar amends a documentation artifact belonging to a release that is already cut, where the surveyed projects treat published release notes as immutable history.

Recorded, and answered rather than accepted. The amended file is `docs/audits/triage-1.26.0.md`, an internal audit artifact, not a published release note and not shipped in the crate. The claim that stood there — "No S1 finding is open at the cut" — was true and remains true; it now reads "was open **at the cut**", and the new row is under a heading that says "Filed after the cut". Nothing about the cut's own record was rewritten. The alternative, a fresh document per post-cut arrival, would put the board's state in a place no reader of the release's triage would look.
