# PMAT-531 — adjudicated claims

One round of three sandboxed agy quorum lanes on the v1.29.0 ledger booking:
3/3 FAIL. Every refutation was re-run before it was acted on; three reproduced
and one did not.

## CONFIRMED

1. [cookbook] That `cookbook: 7c100454…` is right, and is the first such field
   any release has carried.
   - evidence: confirmed 3/3 independently. It was the tip of
     paiml/forjar-cookbook's `master` at the moment of the cut, and its
     `Cargo.toml` requires `forjar = "1.2"`, which admits 1.29.0 under the caret
     rule the gate applies. It is the first row with the field, matching
     `cookbook_floor: v1.29.0`.
   - evidence: the arm that enforces it is driven by the eight cases in
     tests/falsification_release_cookbook_is_part_of_the_release.rs:1, including
     a 35-row table of the requirement rule across caret, tilde and exact.

2. [counts] That the row names thirteen PRs where the CHANGELOG says twelve,
   and both are right.
   - evidence: confirmed by two lanes, each reasoning independently from the
     window: the cut's own PR #527 had not merged when the CHANGELOG sentence
     was written and has merged now, so the ledger window includes it and the
     CHANGELOG's does not. That asymmetry is why gate T's T9 arm reads the
     CHANGELOG only while a cut is IN FLIGHT.

3. [cadence] That `next` is correct — v1.30.0, due at the cut instant plus
   `cadence_days`.
   - evidence: confirmed 3/3 against the tag's own creatordate. 48 hours
     exactly, re-derived rather than read back from the file, which is what
     gate T's T6 arm already refuses a disagreement on.

4. [rail] That the booking diff stayed on the quorum gate's `kind: triage`
   rail when the lanes read it.
   - evidence: confirmed 3/3 — the diff touched only
     `docs/roadmaps/releases.yaml` and `docs/roadmaps/roadmap.yaml`, both named
     on the rail. **This became FALSE afterwards**, by this branch's own later
     work: fixing the ratchet's ceilings brought
     `scripts/ratchets/cb21xx-baseline.json` into the diff, which is off the
     rail, so the receipt is `kind: code` with a falsification. Recorded rather
     than left, because a confirmation that expired is not a confirmation.

## REFUTED

1. [labels] That the label moves the cut made are right.
   - evidence: refuted 3/3, unanimously, and both halves reproduce.
     **PMAT-233 reads `cancelled`** and `release-goal.sh cut` moved its
     `release:` label from v1.29.0 to v1.30.0. A cancelled ticket belongs to no
     release, so that is a treadmill: every future cut carries it one window
     further while gate T's window arms count it as open work.
     **PMAT-240 read `completed`** and was moved into the next window as
     finished work that was never done.
   - corrected: PMAT-233's label removed; PMAT-240 corrected to `planned`. Both
     gates filed — PMAT-529 (#529) for the labelling, PMAT-528 (#528) for the
     status asymmetry — rather than patched here, because each has a judgement
     in it that a booking PR is the wrong place to make.

2. [status] That `status: completed` on PMAT-240 was true.
   - evidence: refuted 2/3 and it reproduces hard. Thirteen pipelines of its
     own shape are still in `scripts/`, measured by the census predicate, and
     NEITHER acceptance criterion had been met. It was `planned` when minted at
     f98b388d and `completed` by c5e4aaa3 — flipped during unrelated work with
     no commit saying so.
   - corrected: textually and deliberately, because `pmat work edit` refuses
     `Completed -> Planned` and `Completed -> InProgress`. A lifecycle rule that
     keeps a false `completed` in the record is the wrong rule for this case.
     The row now has issue #530 and a release binding, neither of which it had.

3. [stale] That no sentence in the record is checkable and false.
   - evidence: refuted 2/3. PMAT-520's acceptance criteria said "ten behaviour
     bullets" and "twelve tickets" where the release shipped twelve and fifteen;
     and the ledger header's "ten tags since the cookbook's master" became
     ELEVEN the moment v1.29.0 was tagged — a count that goes stale at every
     release, written into a static document.
   - corrected: the criteria; and the tag count is left as the measurement it
     was, with its instant, rather than restated as a number that decays.

4. [tickets] That the `tickets` list misses PMAT-200, PMAT-233 and PMAT-240,
   named in PR bodies.
   - evidence: **does not reproduce.** Refuted by two lanes and CONFIRMED by the
     third, which is the one that read the rule. The window's ticket rule is the
     first `PMAT-<n>` in the branch, then the title, then the body — the same
     rule gate A applies — and the row matches it exactly. What the other two
     found were MENTIONS: PR #517's body says "eighteen more elsewhere are
     PMAT-240, census committed", which is a reference to future work, not a
     claim that it shipped. Measured: no commit between v1.28.0 and v1.29.0
     names PMAT-240 as its subject.
   - corrected: nothing. Recorded as a lane error, and as evidence that the
     one-ticket-per-PR rule is worth stating wherever the ticket list is read.
