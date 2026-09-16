# PMAT-576 — the claims put to the round

The v1.31.0 booking is a ledger change: `docs/roadmaps/releases.yaml` gains the
row for a release that was tagged and published on 2026-09-16, `roadmap.yaml`
gains the status and label edits the release gates require, and
`docs/audits/impl-PMAT-576-receipt.md` records what was measured. No code.

The claims put to the lanes were:

1. Every field of the v1.31.0 row is MEASURED — the cut instant is the tag's,
   the PR list is what GitHub reports merged in the commit range, and the ticket
   list is what those PRs name — not what the cut intended.
2. The `cookbook:` commit is the one this release was qualified against, whose
   `Cargo.toml` admits 1.31.0 and whose `Cargo.lock` PINS it, rather than the
   1.30.0 commit `release-goal.sh cut` copies by default.
3. `next: v1.32.0` is declared with `due` equal to the cut instant plus
   `cadence_days`, which is the rule gate T's cadence arm applies.
4. PMAT-574 is marked completed because its PR MERGED, and the booking PR is the
   right place to do it — a ticket is open in its own PR.
5. The three `release:v1.31.0` labels `release-goal.sh` moved to
   `release:v1.32.0` were moved because those tickets were NOT in this window,
   which is the fabricated-link arm of gate T doing its job.
6. Gate T goes from FAIL on main to the v1.31.0 row passing on this branch, and
   the receipt quotes both.
7. The receipt states what this booking does NOT close: the GitHub Release was a
   draft prerelease with binaries still building, so `releases/latest` did not
   yet resolve to v1.31.0.

Every round's lanes, models and verdicts are the table in
`book-1.31.0-lanes.md`, which is the only place this receipt counts them.
