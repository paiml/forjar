# Implementation receipt — PMAT-533 — the 1.29.0 release record

verdict: PASS — forjar 1.29.0 is on crates.io, docs.rs built it, the GitHub release carries 14 assets and is what `/releases/latest` resolves to; `make dogfood-published VERSION=1.29.0` exits 0 against what crates.io actually serves; gate R and gate T are both green; and the five false sentences three review lanes found in the record are corrected before it becomes the record.

orch_model: opus [A]   orch_class: triage   orch_decision: admit   orch_basis: release
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=measurement  route=self  w=100.00  basis=absent  (the published artifact, measured)
  ph1.record  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (three lanes, 3 FAIL)
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="bash scripts/dogfood/release-check.sh"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/release-1.29.0-receipt.md
  cmd="make dogfood-published VERSION=1.29.0"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/release-1.29.0-receipt.md
  cmd="bash scripts/dogfood/tagged.sh"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/release-1.29.0-receipt.md

## What shipped

forjar 1.29.0: tag `bb979f9e` on `20e80f64`, crates.io at 2026-09-11T19:56:31Z
(3,876,443 bytes, not yanked), docs.rs `doc_status: true`, the GitHub release
published with 14 assets, not a draft, not a prerelease, and now what
`/releases/latest` resolves to.

**The first release whose ledger row names the cookbook commit it was qualified
against** — `7c100454` (PMAT-241).

## Four steps by hand, and one thing not measured

The record names all four and why: the tag-triggered `Release` run died on the
shared cargo registry sweep and was re-run; `binary-release.yml` was dispatched
with the tag because it did not fire on the push; the promotion from prerelease
to full release, which is the operator's step by design here; and the crates.io
publish itself, which is this repository's standing arrangement.

**Why `binary-release.yml` did not fire on the tag push is NOT MEASURED**, and
the record says so in those words rather than guessing. The `Release` and
`Security Audit` workflows both fired on the same push, so the usual
explanation — a tag pushed with `GITHUB_TOKEN` — does not fit.

## What the release exposed

**`/releases/latest` resolved to v1.25.2** while v1.26.0, v1.27.0 and v1.28.0
were all published, non-draft, non-prerelease. Anyone resolving that URL got a
four-version-old binary for days, and gate R was green throughout: it READS
`isPrerelease`, reports it, never asserts it, and never asks what `latest`
says. Corrected by hand; filed as PMAT-534 (#534).

**A branch named for the wrong ticket credits a shipped ticket to the next
window.** PMAT-520 shipped in 1.29.0 — its PR #527's merge commit IS the commit
the tag points at — and also carries `release:v1.30.0`, which gate T demanded,
because the window rule resolves a PR's ticket from the first `PMAT-<n>` in its
BRANCH NAME and the booking PR #532 was pushed from `PMAT-520-book-v1.29.0`
while its ticket is PMAT-531. The branch name is load-bearing for three gates —
A resolves a receipt path from it, E the quorum slug, T the window — and
nothing checks it against the trailer. Filed as PMAT-535 (#535).

## Five false sentences, caught before they were the record

Three lanes, one instruction above the six claims: quote any sentence a reader
could check and find false. Every one re-measured here:

1. **The Gate R quote was not a quote** — truncated at `(it is present)` and
   rewrapped across four lines, under a heading claiming it was verbatim.
   Unanimous. One line now, byte for byte, with the heading saying what
   happened.
2. **"Three steps taken by hand" was four** — the crates.io publish is done by
   hand from this host, which the Identity table two paragraphs above already
   said. Unanimous.
3. **The window said 12 PRs and 15 tickets** where the ledger row says 13 and
   16. Both are right about different windows; a RELEASE receipt describes the
   shipped release, so it takes the ledger's.
4. **PMAT-520 did not merge after the tag.** Two lanes caught the receipt
   asserting otherwise, and that correction opened PMAT-535.
5. **The crux Method said "ten of the eleven"** where twelve bullets shipped.

## What the lanes could not check

Two lanes reported `docs.rs` as UNCHECKED — one got a 404 from the API in its
sandbox. Measured here directly: `https://docs.rs/crate/forjar/1.29.0/status.json`
returns `{"doc_status":true,"version":"1.29.0"}`. Two also could not run
`make dogfood-published`; it was run here and exits 0.

## Gaps, named

- Why `binary-release.yml` did not fire is unmeasured, and the next cut will
  hit it again unless someone looks.
- PMAT-520 keeps two release labels until PMAT-535 lands. The `release:v1.29.0`
  one is true and the ledger row says so; the other is a gate's demand, not a
  claim anyone made.
- Nothing yet asserts that a release is what `/releases/latest` resolves to.
  That is PMAT-534 and it was green for four releases.

IMPL-PMAT-533-RECEIPT-END
