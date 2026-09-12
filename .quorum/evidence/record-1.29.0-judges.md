# PMAT-533 — adjudicated claims

One round of three sandboxed agy quorum lanes on the 1.29.0 release record:
3/3 FAIL. Every refutation was re-measured before it was acted on, and all five
reproduced.

This is the third round in this release run with the same standing instruction
above the numbered claims — quote any sentence a reader could check and find
false — and the third time it produced every finding that mattered.

## CONFIRMED

1. [latest] That the `/releases/latest` finding is stated at the right
   strength.
   - evidence: confirmed 3/3 independently from the GitHub API. v1.26.0,
     v1.27.0 and v1.28.0 were each `prerelease=false draft=false` and none was
     latest; the pointer resolved to v1.25.2. Anyone using that URL got a
     four-version-old binary.
   - evidence: all three also confirmed the second half — that gate R READS
     `isPrerelease` and reports it without asserting it, and never asks what
     `latest` resolves to — which is why it was green for four releases while
     what a user downloads was stale. Filed as PMAT-534.

2. [consistency] That the record does not contradict the dogfood receipt, the
   crux audit or the CHANGELOG.
   - evidence: confirmed by two lanes cross-reading all four documents for a
     number appearing twice with two values, and finding none in the record
     itself.
   - evidence: the third found one in the CRUX audit rather than the record —
     its Method paragraph said "ten of the eleven behaviour changes" where
     twelve shipped — which reproduces and is corrected. A confirmation of the
     record that surfaced a defect next door is worth more than a clean one.

## REFUTED

1. [quote] That the Gate R line in the record is quoted rather than
   paraphrased.
   - evidence: refuted 3/3, unanimously, and it is the plainest kind of false
     sentence: the heading said "quoted rather than paraphrased" over text that
     had been truncated at `(it is present)` and rewrapped across four lines.
     The real line is 426 bytes on one line and ends `… so no release is being
     cut and its reconciliation is not re-run here`.
   - corrected: one line, byte for byte, verified by substring match against
     the live gate's output, and the heading now says what happened to the
     first version.

2. [count] That four hand-taken steps are three.
   - evidence: refuted 3/3. The crates.io publish is done by hand from this
     host with `cargo publish --locked` against the local credentials file —
     there is no trusted publishing on this repository — and the Identity table
     two paragraphs above the section said so all along. A section headed
     "three" while its own document listed a fourth is the shape of error a
     record is written to avoid.
   - corrected: four, with the fourth named and its standing arrangement
     stated.

3. [window] That the release's window is 12 PRs and 15 tickets.
   - evidence: refuted by one lane against the ledger row, which records 13 and
     16. Both numbers are right about different windows — the CHANGELOG's
     twelve was true when it was written, before the cut's own PR #527 merged —
     but a RELEASE receipt describes the shipped release, and the shipped
     release includes #527 because its merge commit IS the tagged commit.
   - corrected: the ledger's 13 and 16, with the asymmetry explained and
     pointed at gate T's T9 arm, which exists for it.

4. [tag] That PMAT-520's PR merged after the v1.29.0 tag.
   - evidence: refuted by two lanes and it reproduces in one command: PR #527's
     merge commit is `20e80f64`, and `git rev-parse v1.29.0^{commit}` is
     `20e80f64`. It merged AT the tag and shipped in the release.
   - corrected: the record says so, and the correction opened the real defect
     below.

5. [branch] That PMAT-520's `release:v1.30.0` label was put there by a person.
   - evidence: nobody put it there — gate T demanded it, and gate T is doing
     what it was told. The window rule resolves a PR's ticket from the first
     `PMAT-<n>` in its BRANCH NAME, and the booking PR #532 was pushed from
     `PMAT-520-book-v1.29.0` while its ticket is PMAT-531: the trailer, the
     title and the receipt all say 531. So every window arm resolves #532 to
     PMAT-520, PMAT-520 claims two releases, and PMAT-531 is invisible to all
     of them.
   - corrected: nothing here, deliberately. The branch name is LOAD-BEARING for
     three gates — A resolves a receipt path from it, E the quorum slug, T the
     window — and nothing checks it against the trailer. Filed as PMAT-535
     (#535) with PMAT-520's double label as its regression case, because
     changing a resolution rule three gates share is not a release-record PR's
     work.
