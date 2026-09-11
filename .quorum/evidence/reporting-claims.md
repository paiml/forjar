# Quorum evidence — PMAT-228 + PMAT-229 — the claims as put to the lanes

# PMAT-228 + PMAT-229 — claims for the quorum lanes

Branch PMAT-228-notes-go-to-stderr, two commits (1500102c, cdd03571) on
main (0c1277c6). Judge the diff `main...cdd03571`. Two tickets in one
branch and one review: both are `release-goal.sh` telling the operator the
wrong thing, both are a few lines, and three lanes per one-line ticket is
the kind of spend this repository's own cadence brief calls waste.

## PMAT-228 — a census note is a diagnostic

1. Both "not counted" notes in `dogfood_prs_between` go to stderr, and
   nothing else in that function changes.
2. `a_census_note_is_reported_and_never_reaches_the_ledger` is RED against
   main's window.sh and GREEN at HEAD; it asserts BOTH that the note is
   still reported and that the ledger does not contain it, so a fix that
   stopped reporting would fail it.
3. The case's fixture is the situation that fired: a PR whose merge commit
   is an ancestor of the PREVIOUS tag.
4. The row `cut` writes still declares the PR that IS in the window
   (`prs: [10]`), so a note was removed and not a measurement.

## PMAT-229 — the status line renders what it can

5. `release-goal.sh show` now renders the goal line — next tag, bar, due
   instant, `basis=` — even when the merged count cannot be measured, and
   prints `UNMEASURED` in place of the two counts.
6. It still exits non-zero in that state, so no script can read a degraded
   line as a pass.
7. `scripts/dogfood/tagged.sh` is byte-identical to main's: gate T never
   renders a degraded line. The opt-in is `DOGFOOD_WINDOW_SOFT`, set by
   `show` and by nothing else in the tree.
8. `the_status_line_renders_the_goal_when_the_merged_count_is_unmeasurable`
   is RED against main's scripts and GREEN at HEAD, and its last assertion
   is that the gate over the same window is still red.
9. The case's fixture is the state that fired on PMAT-227's booking branch:
   the previous release's own PR is all GitHub reports, so the window is
   empty while a commit sits in it.

## Both

10. `docs/audits/logs/PMAT-228-gate-tests.log` records both RED cases
    (`--no-fail-fast`, so both binaries report) and the three suites green
    at HEAD: 19 + 2 + 13.
11. `bashrs lint` reports 0 errors on both changed scripts, and gate T is
    green on the branch.
12. No file under src/ changes; the diff is two shell files and two test
    files.
