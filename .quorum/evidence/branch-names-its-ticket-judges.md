# PMAT-535 — adjudicated claims

One round of three sandboxed agy quorum lanes: 1 do-not-implement-as-written,
2 FAIL. Four refutations and two measured false refusals, every one re-measured
before it was acted on and every one reproduced.

The rule survived. Its reason did not, two of its normalisations did not, and
one of its reads failed open.

## CONFIRMED

1. [placement] That the arm sits in the right place — after the `PRINT_HASH`
   early exit and the deletion and merge-base guards, immediately before
   `receipt=`.
   - evidence: a lane read the whole file above the arm rather than the diff and
     confirmed `$pushed` and `$merge_base` are both validated commits by that
     point, that a branch deletion has already exited, and that a misnamed
     branch is refused before the script derives a receipt path from the name it
     is refusing. `scripts/quorum-gate.sh:204` (deletion), `:227` (merge-base),
     `:274` (PRINT_HASH) all precede it.

2. [no-false-refusal-on-chores] That a branch with no `PMAT-<n>` in its name is
   not this arm's business and is not refused by it.
   - evidence: a lane traced the `awk match(...)` to an empty `branch_id` and
     the whole block to a skip, and the shell probe in §5 of the log runs
     `fix/ci-lint` through the real gate and reaches the receipt arm.
     `tests/falsification_a_branch_names_its_own_ticket.rs:241` is the case,
     and `tests/falsification_a_branch_names_its_own_ticket.rs:176` is the
     positive case it is the mirror of.

3. [git-finds-it-less-often] That git's own trailer parser finds the trailer
   LESS often than reading the line does, so this arm cannot refuse a commit
   CB-2113 or the commit-msg hook would accept.
   - evidence: a lane measured both on `b4719737`, the merge commit of PR #532:
     `%(trailers:key=Pmat-Ticket,valueonly=true)` returns nothing and
     `sed -n 's/^Pmat-Ticket:…//p'` returns `PMAT-531`. Reproduced in §2 of the
     log. This turned out to be the arm's real justification, and
     `tests/falsification_a_branch_names_its_own_ticket.rs:266` asserts it as a
     precondition before asserting anything about the arm.

4. [receipt-shape] That the receipt satisfies gate A's shape: it ends with
   `IMPL-PMAT-535-RECEIPT-END` and exactly one line matches `^verdict:`.
   - evidence: a lane measured it with `tail -1` and `grep -c '^verdict:'`
     rather than reading for it, and PMAT-520 carrying both `release:v1.29.0`
     and `release:v1.30.0` was confirmed independently at
     `docs/roadmaps/roadmap.yaml:3655-3656`.

## REFUTED

1. [false-reason] That "pmat's own CB-2113 and this repository's commit-msg hook
   both match the LINE wherever it appears."
   - evidence: two lanes measured it and both refuted it. CB-2113 asks GIT —
     `src/services/commit_traceability/mod.rs:275` and `:320` build
     `--format=…%(trailers:key=Pmat-Ticket,valueonly,separator=…)`. The
     commit-msg hook asks git first with `git interpret-trailers --parse`, and
     on a miss falls back to `grep -qE 'PMAT-[0-9]+|#[0-9]+'` over the whole
     message — any ticket-shaped string anywhere, not a `Pmat-Ticket:` line.
     That fallback is why every multi-paragraph commit in this session passed.
   - corrected: the comment and the receipt now give the reason that is true and
     is measured on the defect itself — git's parser returns NOTHING for
     `b4719737`, so an arm built on it would have let PR #532 through.
     `tests/falsification_a_branch_names_its_own_ticket.rs:276` pins that
     precondition so the case cannot quietly stop testing anything. The
     stronger claim, that reading the line is strictly MORE permissive than
     either tool, replaces the false one. Log §3 prints both tools' source.

2. [crlf] That the arm produces no false refusals.
   - evidence: a lane built a CRLF commit message and measured the gate refusing
     `PMAT-535-crlf` whose own commit claims PMAT-535 — `sed` left the carriage
     return glued to the id, so `PMAT-535\r` did not match. The refusal printed
     `Trailers on this branch: PMAT-535` one line under the sentence saying no
     commit claims it. Reproduced against the committed arm in §4 of the log.
   - corrected: `tr -d '\r'` in the extraction, and
     `tests/falsification_a_branch_names_its_own_ticket.rs:294` is the
     regression case — RED against `9c2426af`, green now.

3. [comma] That one `Pmat-Ticket:` line names one ticket.
   - evidence: the same lane measured `Pmat-Ticket: PMAT-535, PMAT-536` refusing
     a branch named `PMAT-535-…`: unsplit, the first id reads as `PMAT-535,` and
     matches nothing. Reproduced in §4 of the log.
   - corrected: `tr ',' ' '` splits the claims, and
     `tests/falsification_a_branch_names_its_own_ticket.rs:317` is the
     regression case. The key is also matched case-insensitively and a leading
     indent is allowed, both for the same reason: every one of those is looser
     than git and looser than the hook, so none can add a refusal those tools
     disagree with.

4. [fail-open] That an unreadable history is not a pass.
   - evidence: a lane read the trailer extraction — a `git log` redirected to
     `/dev/null` with no exit check — and traced a failure to an empty
     `$trailers`, an empty `$claimed`, and the arm's own skip — a check that
     could not run printing exactly what a check that passed prints, which this
     repository forbids in `CLAUDE.md`.
   - corrected: `|| die` with a message that says the arm is UNMEASURED. The
     receipt states plainly that this path has no falsifier, because both of its
     arguments are commits the script has already validated and a test would
     have to break those validations first — unlike every other arm here, which
     `tests/falsification_a_branch_names_its_own_ticket.rs:175` onwards drive.

5. [vacuity] That the six falsification cases each fail if the arm is deleted.
   - evidence: a lane read the suite and found five of the six assert only that
     a message was NOT printed, so deleting the arm leaves them green. Measured
     directly afterwards by cutting the arm's lines out of the script: both the
     `PMAT-520-book-v1.29.0` and the multi-paragraph branches fall through to
     the receipt arm, and only the two positive cases fail.
   - corrected: `a_pmat_ticket_line_git_would_not_call_a_trailer_still_counts`
     was rewritten as `the_arm_refuses_on_a_trailer_gits_own_parser_cannot_see`
     at `tests/falsification_a_branch_names_its_own_ticket.rs:266` — it asserts
     git sees nothing, then asserts the arm refuses AND names the ticket git
     could not see, so the arm's absence fails it. The remaining six are
     false-refusal guards, which cannot die from deletion by construction; two
     of them are RED against the arm this branch had already committed, which is
     the answer to the vacuity charge. Section 6 of the log is that measurement.

6. [bashrs-overclaim] That all four new bashrs warnings are artifacts of the
   multi-line `die` message string.
   - evidence: two lanes quoted the sentence and measured the counter-example —
     SC2101 fires on `*[![:space:]]*` at `scripts/quorum-gate.sh:319` of the
     previous revision, a functional `case` PATTERN outside the string entirely.
     The receipt's own next sentence described it correctly as a case pattern,
     which made the topic sentence false rather than merely loose.
   - corrected: the case pattern is gone — `[ -n "$claimed" ]` replaced it — so
     SC2101 no longer fires at all, and §9 of the log now prints all eighteen
     new findings with the exact text at the column bashrs named, including the
     two (SC2089 at 319 and 334) that are on real code and are intended.

7. [scope] That refusing a misnamed branch closes the misattribution.
   - evidence: a lane pointed out that the window rule reads BRANCH, then TITLE,
     then BODY, and this arm can only see the first. A branch named
     `fix/ci-lint` — the naming this repository's own guidelines suggest for
     work with no ticket — with a PR titled `(PMAT-520)` reproduces PR #532's
     misattribution exactly, and the arm never runs.
   - corrected: not fixable at push time, because the PR does not exist yet and
     its title is typed afterwards. Filed as PMAT-540 (forjar#540) against gate
     A, which already holds the PR object, the merge commit and the receipt.
     Named in the receipt's "Gaps, named" as the first gap.

8. [advice] That the refusal tells the reader the right thing to do.
   - evidence: a lane quoted the refusal's closing advice, which offered a
     choice between renaming and adding a commit, and observed that the second
     half satisfies
     the gate while LEAVING THE BRANCH MISNAMED — advice that walks the reader
     into the exact outcome the arm exists to prevent.
   - corrected: the message now says RENAME THE BRANCH, and says in so many
     words that adding a commit which merely names the id would satisfy this arm
     and leave the branch misnamed.

## What the round cost and what it bought

Three lanes, one round, ~14 minutes of agy. It refuted the arm's stated reason,
found two false refusals that would have blocked real pushes, found a read that
failed open, found a vacuous half of the suite, found a false sentence in the
receipt, and produced a ticket. The arm as first committed would have refused
CRLF and comma-separated trailers in production.
