# PMAT-542 — adjudicated claims

One round of three sandboxed agy quorum lanes: 3/3 FAIL, 31 findings. Five
confirmations and eight refutations, every one re-measured before it was acted
on.

The shape held. The SELECTION did not: it failed OPEN on the two gates' own
fixtures and their shared binary resolver, and no test in the suite could have
found it — a lane reading the two scripts did.

## CONFIRMED

1. [wiring] That the workflow is wired the way GitHub will actually run it.
   - evidence: a lane traced the composite action's `outputs:` through the
     `classify` job's own `outputs:` to `needs.classify.outputs.gate_c`, checked
     that `if: needs.classify.outputs.code == 'true' && … != 'none'` is valid
     unquoted YAML with no leading brace and no bare colon, and confirmed the
     `gate` job lists both new jobs with `always()` still holding.
     `tests/falsification_pr_lane_selects_the_gate_the_change_can_move.rs:301`
     is the case that pins the plumbing.

2. [fails-closed-on-nothing] That an unmeasured class or selection is refused
   rather than skipped.
   - evidence: the same lane worked the case through: if `classify` is skipped
     or fails, every output is the empty string, `dogfood-surface` skips because
     `'' == 'true'` is false, and the gate's `[ -z "$CODE" ]` and
     `[ -z "$GATES" ]` both fire and exit 1. That is the direction this design
     has to get right, and it does.

3. [no-orphan-check] That renaming the job leaves no unsatisfiable required
   check and drops no step.
   - evidence: a lane grepped every file under `.github/` and every script for
     a job named `dogfood` and found none outside the two new jobs, then walked
     the old job's steps one by one — checkout, build, gate C, cookbook clone,
     gate D, guard tests — and found each present across `dogfood-surface` and
     `dogfood-guards`. A rename that stranded a required check would block every
     merge in the repository.

4. [receipt-shape] That the receipt satisfies gate A's shape.
   - evidence: a lane measured it with `tail -1` and `grep -c '^verdict:'`
     rather than reading for it: the file ends with `IMPL-PMAT-542-RECEIPT-END`
     and exactly one line matches `^verdict:`.

5. [class-unchanged] That PMAT-237's own cases still hold, so the CLASS is
   untouched by this change.
   - evidence: re-measured directly by reverting the three non-Rust files to
     `origin/main` and running both suites with `--no-fail-fast`: 12 of
     PMAT-237's 13 stay green. The thirteenth,
     `tests/falsification_pr_lane_runs_what_the_change_can_break.rs:265`, reads
     job names out of the workflow and the revert puts back a job called
     `dogfood`, so its red is the rename and not the class.

## REFUTED

1. [fail-open] That gate C reads only the built binary and the surface CSV, and
   gate D only the binary, README.md and that CSV — "Nothing else."
   - evidence: two lanes measured it independently and both refuted it, quoting
     the lines: `surface.sh:50` and `docs.sh:62` each run
     `bash "$(dirname …)/lib/binary.sh"`; `surface.sh:277` sets
     `FIXTURE="tests/fixtures/dogfood/local-files.yaml"` and evaluates it two
     ways; `docs.sh:41` sets `FIXTURES="tests/fixtures/dogfood"` and runs every
     documented invocation against the tree; `docs.sh:279` opens `Cargo.toml`.
   - corrected: the sentence is gone from the script, the workflow, the commit
     message and the receipt, replaced by the closure derived from those lines.

2. [harmless-arm] That `tests/*` and `scripts/*` outside `scripts/dogfood/` are
   harmless for the two gates.
   - evidence: follows from the above and is the reason it matters. Both
     fixtures and the shared resolver live under exactly those prefixes, so a PR
     editing the very fixture gate C compares two evaluations of would have
     SKIPPED gate C. That is the fail-open direction: it ships a surface nothing
     measured, and no case in the suite could have caught it.
   - corrected: `scripts/dogfood/lib/*` and `tests/fixtures/*` select both,
     `surface.sh` selects C and `docs.sh` selects D, at
     `tests/falsification_pr_lane_selects_the_gate_the_change_can_move.rs:91`.
     `…:159` re-derives the closure from the two scripts on every run, so a gate
     that starts reading a new fixture turns the suite red rather than blinding
     itself.

3. [over-correction] That closing the hole is free.
   - evidence: the first fix selected the WHOLE of `scripts/dogfood/` and was
     measured against the same 25 PRs: the saving collapsed from seven to TWO,
     because this repository develops `harness.sh`, `tagged.sh`, `comply.sh` and
     the rest constantly and neither gate reads any of them. Over-selecting is
     the cheap mistake and it is not the free one.
   - corrected: the closure is `lib/*` plus the two gate scripts by name, and
     `tests/falsification_pr_lane_selects_the_gate_the_change_can_move.rs:46`
     asserts `harness.sh` and `tagged.sh` are still NOT selected, so the arm
     cannot buy its safety by selecting everything.

4. [narrower-base] That an absent base is handled.
   - evidence: a lane read the composite action and found
     `BASE=$(git rev-parse 'HEAD~1' …)` as the fallback when there is no PR base
     and no `event.before`. On a `workflow_dispatch` re-run that classifies a
     twenty-commit branch from ONE commit — narrowing the diff, which selects
     too little, which skips a measurement.
   - corrected: an absent base now hands the EMPTY file list to the classifier's
     own rule that no readable file list is not a licence to skip anything, so
     it comes out `code=true gates=C,D`. Pinned at
     `tests/falsification_pr_lane_selects_the_gate_the_change_can_move.rs:397`.

5. [bare-substring] That the workflow cases assert on what the workflow does.
   - evidence: a lane pointed out that `ci.contains("needs.classify.outputs.gate_c == 'true'")`
     is satisfied by a COMMENT carrying those words, so the case would pass over
     a workflow that had been rewired to use `contains()` and left the old
     expression in prose.
   - corrected: anchored on `if: ` at
     `tests/falsification_pr_lane_selects_the_gate_the_change_can_move.rs:365`,
     which only a real step condition can satisfy.

6. [arithmetic] That the dogfood steps sum to the job's 25.2 minutes.
   - evidence: a lane added them: 9.0 + 6.2 + 6.1 + 3.8 = **25.1**, not 25.2.
     The 0.1 is the job's setup, checkout and completion, which the receipt did
     not say.
   - corrected: the receipt now writes "25.1 of steps plus 0.1 of setup and
     checkout", so the two numbers reconcile in the text rather than in the
     reader's head.

7. [stale-roadmap] That the roadmap row carries the measured figure.
   - evidence: a lane read `docs/roadmaps/roadmap.yaml` and found the notes
     still ending "Measured version: 56% of PRs", a figure from the issue's
     first draft that had already been corrected everywhere else.
   - corrected: the row now carries 28%, 149 job-minutes and the reason the
     number moved, and the title says 7 of 25 rather than 10.

8. [red-scope] That "all the new cases are RED against origin/main" says as much
   as it sounds like.
   - evidence: a lane observed that four of them read the WORKFLOW files, so a
     revert of the classifier alone would leave them green; the claim only holds
     because the proof reverted all three non-Rust files together.
   - corrected: the receipt and §6 of the log now say which three files the
     proof reverts, and record that one of PMAT-237's own cases goes red with
     them because it reads job names — stated rather than left for a reader to
     discover.

## What the round cost and what it bought

Three lanes, one round, about ten minutes of agy. It found a selection that
failed open on the gates' own fixtures — the one defect class this design cannot
afford — plus a pre-existing narrowing in PMAT-237's base resolution, four
assertions a comment could satisfy, and three numbers that did not reconcile.
The suite could not have found the first two; it now carries cases for both.
