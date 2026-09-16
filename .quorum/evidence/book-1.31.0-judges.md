# PMAT-576 — adjudicated claims

The rounds, lanes and verdicts are the table in `book-1.31.0-lanes.md`. What
matters here is the adjudication: seven claims CONFIRMED and two REFUTED, with
the refuting authority named in each case. The lanes have no `gh` auth, so every
claim about what GitHub reports was measured by the orchestrator and is marked
as such rather than attributed to a lane that could not see it.

## CONFIRMED

1. [row-is-measured] That every field of the v1.31.0 row is measured rather than
   declared — the cut instant from the tag, the PR list from GitHub over the
   commit range, the ticket list from those PRs.
   - evidence: `docs/roadmaps/releases.yaml:86` is the row
     `release-goal.sh cut` wrote, and the window command prints the identical
     six PRs and six tickets from the live API. The
     cut instant 2026-09-16T08:50:38Z is the tag's own, not the merge's.

2. [cookbook-locks-this-release] That the `cookbook:` commit is the one this
   release was qualified against, not the copied default.
   - evidence: the row names 710b0877. Its manifest requires forjar 1.31 and
     its `Cargo.lock` pins
     1.31.0; the default the script copies, 60acf9c9, pins 1.30.0 and
     gate T's cookbook arm would have refused it. paiml/forjar-cookbook#22 was
     merged BEFORE this row was written, which is the ordering CLAUDE.md
     requires.

3. [next-is-cut-plus-cadence] That `next: v1.32.0` is declared with `due` equal
   to the cut instant plus `cadence_days`.
   - evidence: the `next:` block at the end of `docs/roadmaps/releases.yaml`
     declares v1.32.0 due 2026-09-18T08:50:38Z, which is the row's own cut
     instant at `docs/roadmaps/releases.yaml:86` plus two days. Gate T's
     cadence arm derives the same instant and refuses a declared goal that
     disagrees with the derived one.

4. [574-completed-here] That PMAT-574 is marked completed in this PR because its
   PR merged, and that this is the right PR to do it in.
   - evidence: `docs/roadmaps/roadmap.yaml:4297` is the completed status; #575
     merged at 2026-09-16T08:43:22Z and closed #574. The cut could not do it
     itself: the commit-msg hook refused that commit, saying the ticket is
     completed while work belongs to an open item.

5. [labels-moved-because-not-in-window] That the three `release:v1.31.0` labels
   moved to `release:v1.32.0` were moved because those tickets were not in this
   window.
   - evidence: `release-goal.sh cut` reported removing `release:v1.31.0` from
     PMAT-526 and labelling it `release:v1.32.0`, and the same for PMAT-528 and
     PMAT-529. None of the three appears in the row's ticket list, and gate T
     refuses a ticket claiming a release no PR in that window names.

6. [gate-t-moves] That gate T goes from refusing main to accepting the v1.31.0
   row on this branch, and that the receipt quotes both.
   - evidence: on origin/main gate T FAILs, saying v1.31.0 is reachable from
     HEAD and has no row; on this branch it reports the v1.31.0 cut at
     2026-09-16T08:50:38Z with 6 PRs and 6 tickets labelled. Both lines are in
     `docs/audits/impl-PMAT-576-receipt.md`.

7. [receipt-names-what-is-owed] That the receipt states what this booking does
   NOT close.
   - evidence: it records that the tag's workflow created the GitHub Release as
     a DRAFT prerelease with binaries still building, so
     the releases/latest pointer still resolved to v1.30.0 when the
     receipt was written, and names gate R (PMAT-534) as the instrument for that
     arm rather than claiming the release is finished.

## REFUTED

1. [kind-triage-is-declared] That PMAT-576's ledger row declared the triage
   kind its receipts claim.
   - evidence: round 1's lane 2 refuted it, cited at
     `docs/roadmaps/roadmap.yaml:4362`: the row carried only the release label,
     while PMAT-557's row — the 1.30.0 booking this one copies in shape — carries
     `kind:triage` beside it, and this quorum receipt declares that kind,
     which is the field that selects the documentation-citation rule in the gate.
     The label is on the row now.

2. [lanes-can-judge-a-booking] That three sandboxed lanes are a sufficient panel
   for a ledger booking.
   - evidence: the lanes have no `gh` auth (a measured property, recorded in the
     1.30.0 booking's receipt as 401s), so they cannot ask which PRs merged in
     the window, cannot resolve the cookbook commit, and cannot run gate T
     against the live API. That is most of this branch's evidence, so the binding
     adjudication is the gates and the orchestrator's own measurements, and
     `book-1.31.0-agy.md` says so rather than implying three confirmations of
     facts no lane could reach.

## The kill rule

No lane finding was dismissed. Round 1's single finding was true and is fixed in
the tree. Lane 1's four PASS-with-notes are the receipt's own claims, confirmed
against the lines that carry them. A red gate stops this booking and no verdict
overrides it.
