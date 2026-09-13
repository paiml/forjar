# PMAT-555 — adjudicated claims

One round of three sandboxed agy lanes confirmed all seven claims, 3/3 PASS, and
raised no finding. There was no separate judge phase for this cut and this file
does not pretend there was: what follows is the adjudication that DID happen,
which is the release gates ruling on this branch before the lanes saw it, plus
the seven claims the lanes confirmed.

The gates are named as the refuting authority because they are the ones that
refused. A reader who sees `claims_refuted: 3` and three PASS lanes should read
it exactly that way.

## REFUTED

1. [gate-b-ratchet] "The roadmap bookkeeping is in order, so the CB-21xx ratchet
   holds." Refuted by gate B on this branch, before any lane ran.
   - evidence: `GATE B FAIL the CB-2110..CB-2115 ratchet (Arm 7): REGRESSION:
     CB-2114: 36 finding(s), ceiling 34 (recorded 2026-09-11); CB-2115: 49
     finding(s), ceiling 43` — recorded verbatim at
     docs/audits/logs/PMAT-555-cut.log:2. The growth was the author's: six GitHub
     issues were filed during this window and not one carried the three things
     the baseline's own convention requires of a new ticket.
   - corrected: rows, milestones and `release:` fields for #546 #547 #550 #552
     #553 #554 (milestone 1.31.0) and #555 (1.30.0). Measured after, on the
     committed tree: CB-2112 34 against a ceiling of 35, CB-2114 34 against 34,
     CB-2115 42 against 43. No ceiling is raised and none is lowered in this
     commit, because the baseline permits lowering only from a measurement of the
     committed tree and this tree IS that measurement.

2. [gate-t-label] "Every ticket of the open window carries its release label."
   Refuted by gate T.
   - evidence: `GATE T FAIL PMAT-549 merged since v1.29.0 and its roadmap row
     does not carry release:v1.30.0`. The row is at
     docs/roadmaps/roadmap.yaml:3962. A PR is labelled when it MERGES, not when
     the cut is made, and #551 merged into this window hours before the cut began
     — so a cut that labels at cut time will always miss the last merge.
   - corrected: `scripts/release-goal.sh sync` is the only writer; it labelled
     PMAT-549 and reported the other nine as `already`, which is the shape a
     correct sync has. The same row is also marked `status: completed`, because
     its issue is closed and a `planned` row against a closed issue is precisely
     what the ORPHAN-ROADMAP finding means.

3. [gate-h-shape] "The CHANGELOG's window entries are behaviour bullets." Refuted
   by gate H, three times over.
   - evidence: first `no behaviour bullet under [1.30.0] in CHANGELOG.md` — the
     nine entries were markdown list items, and the parser counts only a bold
     span that OPENS a paragraph, so a release of nine behaviour changes would
     have shipped with the gate that requires a comparison printing nothing.
     Second, with the bullets visible, `no docs/audits/crux-1.30.0.md`. Third,
     with the document written, `1 crux row(s) name fewer than 3 world-class
     systems` — the row at docs/audits/crux-1.30.0.md:32 named Gerrit, which is
     not one of the 28 surveyed systems, so it counted two.
   - corrected: nine paragraphs under CHANGELOG.md:10, a crux row for each, and
     Chef added to the thin row as a third SURVEYED system rather than dropping
     Gerrit, which is the honest comparison even though it does not count.

## CONFIRMED

4. [version-everywhere] The version is bumped everywhere it is declared. All
   three lanes.
   - evidence: `version = "1.30.0"` at Cargo.toml:3 with the lockfile updated in
     the same commit, and README.md:96 moved from 1.29 to 1.30. The second README
     line was at **1.28**: the 1.29.0 cut moved line 96 and left its neighbour, so
     the library-only example has been a version behind the binary example for a
     whole release. Both are at 1.30 now.

5. [changelog-window] The `[1.30.0]` section describes the nine PRs of the window
   and no others. All three lanes.
   - evidence: nine paragraph bullets under CHANGELOG.md:10, one per merged PR
     (#551 #545 #544 #543 #541 #539 #538 #536 #532), cross-checked against
     `GATE T PASS … 10 ticket(s) from 9 PR(s) merged since v1.29.0 carry
     release:v1.30.0`. The `--json` upgrade note inside the drift entry is
     italic, not bold-at-column-0, because it is a note ON a behaviour and not a
     tenth behaviour.

6. [crux-provenance] The crux audit says truthfully how it was produced. All
   three lanes.
   - evidence: docs/audits/crux-1.30.0.md:5 states that 1.29.0's comparison was
     surveyed by an agy quorum lane and this one was written by the release
     orchestrator from documentation memory, with every third-party claim marked
     `[X]`. It is the weaker source and it is labelled as one rather than
     inheriting the previous audit's method paragraph unchanged.

7. [no-behaviour-change] Nothing in this diff changes forjar's behaviour, so no
   falsification test is owed. All three lanes.
   - evidence: gate F reported `no .rs differs from origin/main, so there is
     nothing to mutate`, quoted at docs/audits/dogfood-1.30.0-receipt.md:14, and
     the receipt names that arm as vacuous FOR THIS CUT rather than counting it as
     a pass. The falsification this release rests on is #551's, already merged and
     already measured on its own branch.
