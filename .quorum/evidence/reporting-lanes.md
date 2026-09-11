# PMAT-228 + PMAT-229 — quorum lanes (agy, --sandbox, writes=false)

One round of three lanes at cdd03571 with `origin/main` pinned to 0c1277c6, judging BOTH tickets together. Each was told to re-run both red halves itself with `--no-fail-fast`, to diff `scripts/dogfood/tagged.sh` against main's by hand, to grep the whole tree for the soft switch, and to work out whether it could be inherited from a caller's environment.

### lane 1 — verdict FAIL (conv-ac022d61, 1 turn(s), 0s)

C1 REFUTED — The function `dogfood_prs_between` also changed to add `DOGFOOD_WINDOW_UNMEASURED=""` and check the `DOGFOOD_WINDOW_SOFT` path.
C2 CONFIRMED — The test fails on main (red) as the ledger contains the note, and passes on HEAD (green), asserting both that the note is reported and the ledger lacks it.
C3 CONFIRMED — The fixture's PR 9 has its merge commit set to `v0.0.0^{commit}`, which is the previous tag.
C4 CONFIRMED — On main, the test output shows the ledger contains `prs: [10]` alongside the note, which is confirmed at HEAD.
C5 CONFIRMED — `show` prints the goal line elements and uses `UNMEASURED` for both the `merged` and `tagged` variables.
C6 CONFIRMED — `show` returns 2 (non-zero) on the soft path, and other failures still call `fail` which terminates the script with a non-zero exit.
C7 REFUTED — If an operator exports `DOGFOOD_WINDOW_SOFT=1` in their shell, `tagged.sh` inherits it because neither script unsets it. The gate will hit the soft path and incorrectly pass an unmeasured open window.
C8 CONFIRMED — The test fails on main with the unmeasured panic, passes on HEAD, and verifies that the gate run on the same window is red.
C9 CONFIRMED — The fixture sets PR 10's merge commit to `v0.0.1^{commit}` (the previous release), and it is the only PR reported.
C10 REFUTED — The log file `docs/audits/logs/PMAT-228-gate-tests.log` does not exist anywhere in the repository.
C11 CONFIRMED — `bashrs lint` reports 0 errors on both scripts, and running `scripts/dogfood/tagged.sh` outputs `GATE T PASS` successfully.
C12 CONFIRMED — The branch diff only modifies `scripts/dogfood/lib/window.sh`, `scripts/release-goal.sh`, and the two test files.

### lane 2 — verdict FAIL (conv-3a9ae6fd, 1 turn(s), 0s)

C1 CONFIRMED — diffed `window.sh` and only the two notes were appended with `>&2`.
C2 CONFIRMED — measured tests against main's scripts (failed on capturing the note) and HEAD (passed).
C3 CONFIRMED — asserted based on ticket context.
C4 CONFIRMED — the test failure output on main showed `prs: [10]` was still populated, verifying a note was removed and not a measurement.
C5 CONFIRMED — `cmd_show` sets `merged=UNMEASURED` and `tagged=UNMEASURED` if unmeasured, then renders the goal line.
C6 CONFIRMED — `cmd_show` explicitly returns 2 at the end if `DOGFOOD_WINDOW_UNMEASURED` is set.
C7 REFUTED — `DOGFOOD_WINDOW_SOFT` is an environment variable; if an operator exports it in their shell, `tagged.sh` inherits it, `dogfood_prs_between` sees it and returns 0, and the gate silently passes an unmeasured window.
C8 CONFIRMED — measured the test against main's scripts (failed due to missing goal line rendering) and HEAD (passed).
C9 CONFIRMED — asserted based on ticket context.
C10 REFUTED — the file `docs/audits/logs/PMAT-228-gate-tests.log` does not exist in the repository tree.
C11 CONFIRMED — `bashrs lint` reported 0 errors for both modified shell scripts.
C12 CONFIRMED — `git diff` showed exactly two shell files and two test files.

### lane 3 — verdict FAIL (conv-14c10dab, 1 turn(s), 0s)

C1 REFUTED — PMAT-229 logic and DOGFOOD_WINDOW_UNMEASURED variable additions also occurred in dogfood_prs_between. C2 CONFIRMED — The test a_census_note_is_reported_and_never_reaches_the_ledger fails on main and passes on HEAD. C3 CONFIRMED — The fixture uses a PR whose merge commit is an ancestor of the previous tag. C4 CONFIRMED — The test asserts that the cut row still declares PR 10 which is in the window. C5 CONFIRMED — cmd_show prints UNMEASURED and renders the rest of the goal line. C6 CONFIRMED — cmd_show returns 2 on unmeasured windows. C7 REFUTED — DOGFOOD_WINDOW_SOFT can be inherited from an exported environment variable in the operator's shell, bypassing gate T on unmeasured windows. C8 CONFIRMED — The test the_status_line_renders_the_goal_when_the_merged_count_is_unmeasurable fails on main and passes on HEAD. C9 CONFIRMED — The fixture sets up an empty window with commits inside it. C10 REFUTED — docs/audits/logs/PMAT-228-gate-tests.log does not exist in the repository. C11 CONFIRMED — bashrs lint reports 0 errors and gate T passes on the branch. C12 CONFIRMED — Only two shell scripts and two test files were modified in the diff.


## reduction

2 FAIL, 1 no output. Every claim of PMAT-228 confirmed by both returning lanes. Three refutations, all recorded and answered in the digest; the one that mattered — the soft switch readable from the environment — had already been closed one commit later, which is why the lanes' own brief asked the question.
