# PMAT-520 — adjudicated claims

One round of three sandboxed agy quorum lanes on the 1.29.0 release record:
3/3 FAIL. Every refutation was re-run by the orchestrator before it was acted
on; four reproduced and one did not.

The brief's standing instruction — find a sentence a reader could check and
find false — produced four of the five findings. The six numbered claims
produced one.

## CONFIRMED

1. [crux] That the four crux rows claiming forjar is WORSE than the industry
   default are the right four, and the `[X]` system claims are plausible as
   stated.
   - evidence: two lanes independently identified rows 5, 8, 11 and 12 as the
     four, which is the set the audit's own "limit" section names, and neither
     found a row claiming a win it had not earned. The third lane refuted a
     different sentence in row 1 and did not dispute the four.
   - evidence: the rows name Bazel, Nix, systemd, Kubernetes and SLSA against
     behaviours those systems genuinely have — incompatible-change flags,
     evaluation depth limits, ordering-cycle refusal, deprecation windows, a
     level ladder — and every one is marked `[X]`, asserted from documentation
     memory rather than measured.

2. [roster] That the dogfood receipt's roster-instability finding is supported
   and stated at the right strength.
   - evidence: two lanes confirmed the shape from the banner changes alone —
     `commit: 5db342d5` with 172 checks, `commit: unknown` with 166, and
     `commit: d76e533e` with 172 again, `pmat --version` reading 3.40.0
     throughout. The receipt calls it the largest risk to the next cut, which
     neither lane called overstated.

## REFUTED

1. [count] That thirteen PRs across sixteen tickets merged into this release.
   - evidence: refuted 3/3 and it reproduces. Gate A enumerates the window and
     reports `12 of 12 merged PR(s) since v1.28.0`, and the ticket list from
     the same window library is fifteen ids. The thirteenth PR is this cut's
     own, which has not merged — counting it is the error, and it is the kind
     a release makes every time unless the sentence says why.
   - corrected: twelve and fifteen, with the reason written into the sentence —
     and then made mechanical, because a correction only fixes this release.
     Gate T grew a ninth arm, T9, driven by
     tests/falsification_release_changelog_counts_the_window.rs:44: while a cut
     is in flight, the release's own CHANGELOG section must count what the
     window measures. Against the section exactly as it was written, gate T is
     RED naming the claim, the measurement and the reason.

2. [stale] That nothing a cut needs is missing.
   - evidence: refuted 3/3. `README.md:96` read `forjar = "1.28"` while the
     tree builds 1.29.0. Gate D passed it and is CORRECT by its predicate, a
     caret admitting 1.29.0 — but the comment above that predicate names "a
     README pinning a version the crate has moved past" as the defect, and the
     code only fails on a claim that is NEWER than the tree.
   - corrected: the README by hand, in this cut. The gate's gap is PMAT-526
     (#526), filed rather than widened inside a release cut: what counts as
     too stale is a judgement someone has to make explicitly, and a release is
     not where to make it.

3. [wording] That crux row 1 describes what gate T does.
   - evidence: refuted 1/3, and it reproduces by reading the gate. Row 1 said
     forjar "refuses the tag if the cookbook cannot use it". Gate T refuses the
     ledger ROW; the tag already exists when the gate runs, which is why the
     release is three PRs and why gate T is red on main between the tag and the
     booking.
   - corrected: the row now says the row, and says the tag exists by then. Not
     mechanised: no gate reads a crux row's prose for accuracy, and the one that
     could — tests/falsification_release_changelog_counts_the_window.rs:101,
     which refuses reading a neighbouring release's section — is about
     structure rather than truth.

4. [source] That the release changes no source file.
   - evidence: refuted 1/3 and it reproduces: twelve `.rs` files differ from
     v1.28.0, and zero of them are under `src/`. Gate F's own line was precise
     — no `.rs` differs from origin/main on THIS BRANCH — and the prose around
     it generalised that into a claim about the release, which is false.
   - corrected: both numbers stated, with the distinction between the branch
     and the release made explicit. The same class of slip is what
     tests/falsification_release_changelog_counts_the_window.rs:118 guards
     against from the other side — a number that was true of one window,
     restated about another.

5. [readability] That the CB-21xx figures in the CHANGELOG read as what they
   are.
   - evidence: refuted 2/3, and this one is a writing defect rather than a
     false statement. Two lanes read "49 specifications, 24 roadmap items, ten
     whose id, 34 with no release binding" as a list of CEILINGS and compared
     it against the receipt's 49/49/34/34/43. They are FINDING COUNTS, and the
     two lists genuinely differ because one is per-class and the other is
     per-check.
   - corrected: each figure now names the check it belongs to. A number three
     readers misread is a defect even when it is true, and
     tests/falsification_release_changelog_counts_the_window.rs:61 is the shape
     of the answer: the count is compared, not the typography.

## Refutations that do not reproduce

- **"172 − 6 + 1 = 167, not 166."** One lane called the roster arithmetic
  impossible. Measured: 172 − 6 = 166 exactly, because CB-148 is in BOTH
  rosters — the first build printed it as `RETIRED — superseded by CB-2110`,
  which is a row and not an absence. Recorded as a lane error.
- **"The dogfood receipt contains no falsification block."** All three said
  so, and they are right that it does not — because a dogfood receipt does not
  carry one. Its shape is fixed by the `forjar-dogfood` skill: a gate table,
  exactly one `verdict:` line, no timestamps, an END marker. The falsification
  belongs in the implementation receipt and the quorum artifact, and the brief
  said the receipt "will name" it without saying which receipt. A brief defect,
  recorded as one.
