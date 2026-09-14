# Quorum evidence — PMAT-228 + PMAT-229 — adjudicated claims

One round of three lanes, base pinned at 0c1277c6, the diff at cdd03571: two lanes returned FAIL, one returned nothing. Twelve claims across two tickets; nine confirmed by both returning lanes, three refuted. The refutation that mattered is the one the brief asked for by name, and it had already been closed at 50073b89 — one commit past what the lanes judged, which this digest says plainly rather than presenting their verdict as being about HEAD.

Citations resolve at the merge base, and they are the TEST files rather than the shell ones: the committed-quorum gate anchors a code receipt only on `.rs` and the root manifests, which is PMAT-224's named gap and still true. `tests/falsification_release_goals_are_measured.rs:1` opens the suite that drives `release-goal.sh`, and `tests/falsification_release_goal_cut_books_the_tag.rs:1` the one that drives `cut`. The shell line numbers below are given as prose for a reader, not as anchors.

## CONFIRMED

1. [stderr] BOTH CENSUS NOTES GO TO STDERR — the two `echo` calls that report PRs outside the window carry `>&2`, and the function's stdout is otherwise unchanged.
- evidence: both returning lanes diffed the function. The notes are diagnostics about PRs that are NOT in the window, and `cut` captures stdout to build the ledger row, which is how one reached `docs/roadmaps/releases.yaml` in the v1.28.0 booking. The suite that drives that path is `tests/falsification_release_goal_cut_books_the_tag.rs:1`.

2. [red-green-228] THE CENSUS CASE IS RED AGAINST MAIN AND GREEN HERE — 1 passed / 1 failed with main's `window.sh`, 2 passed at HEAD.
- evidence: both lanes checked main's `scripts/dogfood/lib/window.sh` out over the branch's tests and ran it themselves, then restored and ran it again; the case lives in `tests/falsification_release_goal_cut_books_the_tag.rs:96`'s file, appended after its first case.

3. [fixture-228] THE CASE'S FIXTURE IS THE SITUATION THAT FIRED — a PR whose merge commit is an ancestor of the previous tag, which is what makes the census emit the first note.
- evidence: both lanes read the fixture in `tests/falsification_release_goal_cut_books_the_tag.rs:9`, which is the shared `release_goal_fixture` module the case builds on.

4. [measurement-kept] A NOTE WAS REMOVED AND NOT A MEASUREMENT — the row `cut` writes still declares `prs: [10]`, the PR that IS in the window.
- evidence: both lanes, from the case's own assertion and from the failure output on main.

5. [renders] `show` RENDERS THE GOAL LINE WITH `UNMEASURED` COUNTS — the next tag, the bar, the due instant and `basis=` are printed; the two counts that cannot be measured say so.
- evidence: both lanes read `cmd_show` and ran it against the fixture built by `tests/falsification_release_goals_are_measured.rs:9`'s module import.

6. [exit-code] IT STILL EXITS NON-ZERO IN THAT STATE — `cmd_show` returns 2 when the window could not be measured, so a script that reads the degraded line cannot mistake it for a measured one. An UNMEASURED check is still a failing check; it is now a failing check that tells the reader what it did measure, which is the whole distinction this ticket rests on.
- evidence: both lanes ran it and read the exit code; the case that pins it is in `tests/falsification_release_goals_are_measured.rs:171`'s file, appended after the suite's last case, and its first assertion is the non-zero exit.

7. [red-green-229] THE STATUS-LINE CASE IS RED AGAINST MAIN AND GREEN HERE, AND ITS LAST ASSERTION IS THAT THE GATE OVER THE SAME WINDOW IS STILL RED.
- evidence: both lanes ran it both ways, and the last assertion of the case in `tests/falsification_release_goals_are_measured.rs:171`'s file is that the gate over the same window is still red. The gate's refusal is untouched for every gate caller.

8. [fixture-229] THE CASE'S FIXTURE IS THE STATE THAT FIRED ON PMAT-227's BOOKING BRANCH — the previous release's own PR is all GitHub reports, so the window is empty while a commit sits in it.
- evidence: both lanes read the fixture and the stub.

9. [scope] `bashrs lint` REPORTS 0 ERRORS ON BOTH SCRIPTS, GATE T IS GREEN ON THE BRANCH, AND NO FILE UNDER src/ CHANGES.
- evidence: both lanes ran bashrs and the gate; `git diff --name-only` names two shell files and two test files.

## REFUTED

10. [env-leak] THE SOFT SWITCH WAS READABLE FROM THE ENVIRONMENT — refuted by both returning lanes, independently: `${DOGFOOD_WINDOW_SOFT:-0}` is inherited, so an operator who exports that name in a shell profile softens every gate that sources the library, silently.
- evidence: each lane traced the read in `dogfood_prs_between` and pointed out that `scripts/dogfood/tagged.sh` sources the same library and inherits the environment; the guard that now pins it shut is in `tests/falsification_release_goals_are_measured.rs:171`'s file. This is the single most dangerous thing either ticket could have shipped: a release gate that can be turned off from outside.
- corrected: at 50073b89 the switch became the THIRD POSITIONAL ARGUMENT of `dogfood_prs_between`, passed by `show` and by nothing else; an argument cannot be inherited. `no_environment_variable_can_soften_the_gate` runs gate T with the name exported and asserts a real verdict with no degraded line. The question was in the brief these lanes were given, and it was answered before they reported — which is the argument for writing the attack into the brief rather than hoping a lane finds it.

11. [claim-wording] AS WORDED: "nothing else in that function changes" — refuted by one lane: PMAT-229's soft path is in the same function, so the function changed twice.
- evidence: the diff. The claim meant "nothing else about the census output", and said it badly.
- corrected: the two tickets share `dogfood_prs_between`, and the receipts say so; PMAT-228 changes what it writes and where, PMAT-229 changes what it does when the window cannot be measured.

12. [log-not-yet-committed] AS WORDED: "`docs/audits/logs/PMAT-228-gate-tests.log` records both RED cases" — refuted by both lanes: the file did not exist at the commit they judged.
- evidence: their clones at cdd03571. It was written at 50073b89 with `--no-fail-fast`, precisely because an earlier run had stopped at the first failing binary and left one red case unmeasured.
- corrected: the log is committed and records both red cases and the three green suites. The claim was made about a file the branch did not yet carry, which is the same error PMAT-231's receipt made about a row count and the same fix: measure the commit, not the moment.

## A lane that returned nothing

Lane 1 produced no structured output. It is recorded rather than dropped: the round is three lanes wide on paper and two in evidence, and every claim above rests on two independent re-runs rather than three.
