# PMAT-534 — adjudicated claims

One round of three sandboxed agy quorum lanes: 2/3 PASS, 1/3 FAIL, 19 findings.
Six confirmations and five refutations, every one re-measured before it was
acted on.

The arm refused what it was written to refuse. What it also refused was a state
it had promised, in its own comment, to report — and the case set did not
defend one of its two guards. Both were found by re-measuring a finding, not by
reading the verdicts.

Reverting the script to origin/main kills 5 of the 6 cases, not all of them:
`a_full_release_that_is_latest_passes` asserts the arm does NOT fire, so removal
cannot kill it and M3 (`elif true`) is its killer. This receipt said "all six"
until the revert was actually run.

## CONFIRMED

1. [empty-answer] That `if [ -z "$latest_rel" ]` is defended by no case (lane 2,
   the round's only FAIL).
   - evidence: the guard was deleted from the script and the suite re-run:
     `running 4 tests … test result: ok. 4 passed; 0 failed`. Nothing went red.
     The broken-api stub exercises `rc != 0` and no stub produced exit 0 with
     empty stdout, so the branch was unreachable from the case set. The case
     that now defends it is at
     `tests/falsification_release_check_latest_points_at_the_release.rs:121`,
     and log section 8 records it dying under M4.

2. [stub-wrapper] That adding the new stubs breaks neither suite that shares the
   fixture, because the original signature was kept as a wrapper.
   - evidence: `stub_gh_published` at `tests/release_check_fixture/gh.rs:48` is
     one line delegating to `stub_gh_published_as`. Both neighbours were run:
     `falsification_dogfood_release_check_pr_window` 9 passed and
     `falsification_release_check_says_what_is_pending` 4 passed, before and
     after the extraction of the stubs into their own file.

3. [no-collision] That `latest_rel` collides with nothing and is read by no
   later arm.
   - evidence: `latest_tag` is the only similarly named variable in the script
     and it is first assigned at line 426, more than two hundred lines AFTER arm
     2b reads its own at line 193; `grep -n latest_rel` finds the variable only
     inside arm 2b. `GH` is defaulted at line 44 as `GH="${GH:-gh}"`, so an
     unset `GH` is `gh` and not the empty string under `set -u`.

4. [placement] That arm 2b runs only when the tag is on the remote, so the
   pre-tag states are arm 1's to report and not this one's.
   - evidence: `if [ "$remote_tagged" -eq 0 ]; then` opens at line 133 and its
     `else` is at line 142; arm 2b begins at line 192, inside that `else`. The
     pre-tag suite confirms the behaviour rather than the structure:
     `before_the_tag_is_cut_the_verdict_still_says_so` stays green under every
     mutation in log section 8, including the two that refuse every release.
     Every case that reaches arm 2b builds the POST-tag fixture, as
     `tests/falsification_release_check_latest_points_at_the_release.rs:59`
     does with `published_fixture()`.

5. [receipt-contract] That the receipt satisfies the contract gates A and R
   apply to it.
   - evidence: `grep -c '^verdict:'` is 1 and the last line is
     `IMPL-PMAT-534-RECEIPT-END`, re-checked after the receipt was rewritten
     from four cases to six. `bash scripts/dogfood/release-check.sh` exits 0 on
     this repository and prints one `GATE R PASS` line naming 21 PRs since
     v1.28.0, all `receipt=ok`.

6. [collateral-disclosed] That the log was right to disclose M4/M3 as a
   collateral kill rather than a clean one.
   - evidence: re-run with `--no-fail-fast` so both binaries run, `elif true`
     turns `a_full_release_that_is_latest_passes` red AND three of the four
     `says_what_is_pending` cases. The fourth,
     `before_the_tag_is_cut_the_verdict_still_says_so`, survives because it is
     pre-tag. The original log said three; three is what was measured. The case
     M3 is named for is
     `tests/falsification_release_check_latest_points_at_the_release.rs:59`, and
     it is the only one in this suite proven by an over-refusal.

## REFUTED

1. [false-sentence] That no sentence in the documentation is false — lane 1's closing finding,
   offered after searching "the script's comments, the receipt, the log, and the
   commit messages".
   - corrected: the arm's own comment said "A PRERELEASE IS A DELIBERATE STATE
     and this arm does not force it … That case is REPORTED, with the command
     that ends it." For a repository whose releases are all prereleases the arm
     REFUSED that case as UNMEASURED, because the `rc -ne 0` branch ran before
     `prerelease` was ever consulted. True for forjar, false for the instrument.
     The case that now holds the sentence to it is at
     `tests/falsification_release_check_latest_points_at_the_release.rs:150`.

2. [ordinary-404] That a non-zero exit from `gh api …/releases/latest` always means the pointer
   is UNMEASURED (the premise under lane 1's finding 1).
   - corrected: `/releases/latest` serves only non-prerelease, non-draft
     releases. MEASURED on `electron/electron`, whose newest release
     `v45.0.0-alpha.6` is a prerelease created 2026-09-10T20:00:16Z and whose
     `/releases/latest` answers `v44.3.0`, created two days earlier; and on a
     repository with no releases, where `gh` exits 1 with HTTP 404. A 404 is the
     ORDINARY state for a prerelease, not a broken instrument.

3. [asserted-coverage] That lane 2's coverage finding was `asserted`. Lane 2 marked every one of its
   five findings `asserted`, including the claim that a branch of the script was
   uncovered — a claim a lane briefed NO WRITES could not execute.
   - corrected: it is now `measured`, and the measurement is the one in
     CONFIRMED 1. The brief that produced it is the right one — a lane that
     cannot write must say what it would write — but a finding that reaches the
     receipt has to carry the orchestrator's measurement, not the lane's
     reasoning, and the two were a step apart here. What the measurement
     produced is
     `tests/falsification_release_check_latest_points_at_the_release.rs:121`,
     and the stub it needed is `tests/release_check_fixture/gh.rs:96`.

4. [fail-fast] That this round's first kill matrix showed six mutations each killing exactly
   one case.
   - corrected: it ran `cargo test --test A --test B` without `--no-fail-fast`,
     so cargo stopped after the first failing binary and the
     `says_what_is_pending` suite never ran under any mutation. M3 looked like a
     clean single kill. Re-run with `--no-fail-fast`, M3 kills four cases. The
     wrong run is recorded in log section 8 because the receipt would have
     carried its claim.

5. [fabricated-line] That `.github/workflows/release.yml:593` is where the promotion is named
   (lane 3, grounding `measured`).
   - corrected: the file is 578 lines long, so line 593 does not exist. The
     promotion is named at lines 562-577, and the substance of the finding
     survives: `${{ github.repository }}` is interpolated into double-quoted
     `echo` arguments at lines 576 and 577, a GitHub `owner/name` carries no
     shell metacharacter, and the file parses under `yaml.safe_load`. Lane 3's
     roadmap citation, line 3842, is the row's `priority: high` line and not the
     `labels:` array it described; the row itself, at line 3835, carries
     `status: completed` and `release:v1.30.0` as claimed.
