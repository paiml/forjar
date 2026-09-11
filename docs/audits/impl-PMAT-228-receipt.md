# Implementation receipt — PMAT-228 — a census note is a diagnostic and goes to stderr

verdict: PASS — `dogfood_prs_between` reported the PRs it did not count on stdout, and `release-goal.sh cut` captures that stdout to build the ledger row, so the v1.28.0 booking carried `#490 … is inside v1.27.0 (the previous release) — not counted` into `docs/roadmaps/releases.yaml`, where the accident of a leading `#` made it a YAML comment and nothing complained. Both notes go to stderr now. The fixture case is the situation that fired and asserts both halves — the note is still reported, and the ledger does not contain it — red against main and green here. Reviewed in one round with PMAT-229, deliberately.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: release
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=agy-goal  w=1.00  basis=absent  note=fable-binding  effort=1[U]  (executed by self: two redirections and a fixture case, smaller than a lane brief)
  ph2.quorum  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate, three lanes, shared with PMAT-229)
  ph3  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --no-fail-fast over the three suites that drive these scripts, with main's scripts checked out (RED) then at HEAD (GREEN: 19 + 2 + 14)"  claimed_exit=101(lanes)  rerun_exit=101/0  log_path=docs/audits/logs/PMAT-228-gate-tests.log  sha256=recorded-in-the-log

## Why one review for two tickets

Both tickets are `release-goal.sh` telling an operator the wrong thing, both are a few lines, and both are exercised by the same three test binaries. A three-lane review each would have cost six lanes to judge nine changed lines. The repository's own cadence brief calls that waste, and the batching is the cheapest place to answer it: one branch, one round, both diffs judged together. PR #490 set the precedent for bundling with the reason stated.

## The defect

`cut` builds the row it writes from `cmd_window`'s stdout. The census notes — the previous release's own PR, and anything merged after the upper bound — were on that stdout. They are diagnostics about PRs that are NOT in the window, and they belong on stderr, where they are still reported and never captured.

A tool that writes a comment nobody asked for will one day write a line that is not a comment.

## Falsification

`a_census_note_is_reported_and_never_reaches_the_ledger` (`tests/falsification_release_goal_cut_books_the_tag.rs`): the fixture gives `gh` a PR whose merge commit is an ancestor of the PREVIOUS tag — the case that fired — and asserts the note is said AND the ledger does not contain it, so a fix that stopped reporting would fail it. Red against main's `window.sh` (1 passed, 1 failed), green here (2 passed). The row still declares `prs: [10]`, so a note was removed and not a measurement.

## Review record

One round of three lanes on the pair; two lanes returned, one produced no output and is recorded as a lane failure. Both returning lanes confirmed every PMAT-228 claim. Their refutations are in `.quorum/evidence/reporting-judges.md` and all three are answered there; the one that mattered belongs to PMAT-229 and is in its receipt.

## Gaps, named

- The notes are still printed unconditionally; a caller that wants them silent has no switch. None does.
- One lane returned nothing. Two did, and both re-ran the red and green halves themselves.

IMPL-PMAT-228-RECEIPT-END
