# Implementation receipt — PMAT-540 — the PR and its commits name one ticket

verdict: PASS — gate A now refuses a merged PR whose own merge commit CONTRADICTS the ticket it is filed under. Measured over the thirty most recently merged PRs: 29 agree, PR #532 is the one mismatch, and it is exempted by a floor set at its own merge commit and no later. Nine cases, nine killers: five die when the arm is removed and each of the other four is killed by a mutation that makes the arm fire wrongly. A three-lane round refuted four sentences and found a fixture that did not reproduce the shape it existed for.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (one gate arm, seven cases)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_gate_a_the_pr_and_its_commits_name_one_ticket"  rerun_exit=0 (9 passed)  log=docs/audits/logs/PMAT-540-the-pr-and-its-commits.log §4
  cmd="cargo test --test falsification_dogfood_harness_and_quorum"  rerun_exit=0 (19 passed, unchanged)  log=§4
  cmd="the same 9 against origin/main's harness.sh"  rerun_exit=101 (5 failed, 4 guards green)  log=§4
  cmd="four targeted mutations"  rerun_exit=101 each; the kill matrix is §6  log=§5, §6
  review: 3 agy lanes, 3 × FAIL, 21 findings; four false sentences, a fixture that did not reproduce the shape, an unguarded pipeline and an over-stated promise, all acted on below
  cmd="bash scripts/dogfood/harness.sh"  rerun_exit=0, and exit 1 with a floor this repository does not carry  log=§3
  cmd="the rule against the 30 most recently merged PRs"  rerun_exit=0  log=§2
  cmd="bashrs lint scripts/dogfood/harness.sh"  rerun_exit=1, 0 errors (0 errors on origin/main too)
  cmd="cargo clippy --all-targets -- -D warnings" / "cargo fmt --all -- --check"  rerun_exit=0 / 0

## The half PMAT-535 could not close

The window rule reads the first `PMAT-<n>` in the **branch**, then the **title**,
then the **body** — one rule shared by gates A, E and T. PMAT-535 closed the
branch: `scripts/quorum-gate.sh` refuses, at push time, a branch naming a ticket
no commit being pushed claims.

It cannot close the other two. **At push time the pull request does not exist**
and its title is typed afterwards, so `fix/ci-lint` — the naming this
repository's own guidelines suggest for work with no ticket — with a PR titled
`(PMAT-520)` reproduced PR #532's misattribution untouched.

Gate A is where the PR does exist: it holds the PR object, the merge commit and
the receipt, so it can compare the id it files the PR under against what the
work claims.

## Read by the line, because git's parser is blind to the commit that matters

| asked of `b4719737`, PR #532's merge commit | answer |
|---|---|
| `%(trailers:key=Pmat-Ticket,valueonly=true)` | *(nothing)* |
| `sed -n 's/^…Pmat-Ticket:…//p'` | `PMAT-531` |

git reads trailers from the **last paragraph only**, and GitHub writes that
paragraph itself: after the branch commits' bodies it appends a `---------`
separator and a `Co-authored-by:` block. `%(trailers)` on `b4719737` returns
that one `Co-authored-by:` line and nothing else. pmat's CB-2113 asks git the
same way (`src/services/commit_traceability/mod.rs:275`), which is why it saw
nothing either. An arm built on git's parser would have been blind to the exact
commit it exists for.

The first version of this receipt said the message *"ends with whatever bullets
GitHub assembled"*. **A review lane quoted that and measured it false** — it is
the separator and the co-author block, not the bullets — and the same lane found
that the test fixture therefore did not reproduce the shape at all: git parsed
`squash_claiming`'s trailers happily, so every case would have passed over an
arm "simplified" to use git's parser. That is the one regression this suite
exists to prevent. The fixture now carries the separator, and two cases pin it:
`the_arm_reads_a_claim_gits_own_parser_cannot_see` and
`a_mismatch_gits_own_parser_cannot_see_is_still_refused`, both of which assert
FIRST that git's parser returns nothing. Mutation 4 — read with git's parser —
kills three cases (log §5); before the fix it would have killed none.

The same normalisation PMAT-535 arrived at, for the same measured reasons: strip
`\r` (a CRLF message glues it to the id), split on commas (one line can name two
tickets), allow a leading indent, match the key case-insensitively. Each is
looser than git and looser than the commit-msg hook, so none can invent a
refusal those tools would not make.

## Resolved before compared

PR #496's branch says `PMAT-218`, a ticket that never existed, and its commits
say `PMAT-219` — whose roadmap row declares `alias:PMAT-218`. **An arm comparing
raw ids would have called that a mismatch.** Both sides are resolved through
`dogfood_resolve_id` first, which is also why a real row can never be shadowed:
that function resolves an id to itself before it looks at any alias.

## The floor, and why it is where it is

| over the 30 most recently merged PRs | |
|---|---|
| agree | **29** |
| mismatch | **1** — PR #532 |
| no trailer at all | 0 |

A merged branch cannot be renamed, so #532 could only be exempted by name or
erased by rewriting history. `TRAILER_FLOOR` is **`b4719737`, #532's own merge
commit** — and no later. Every PR merged after it is judged, and #536, #538,
#539, #541 and #543 all pass. A floor at main's tip would have exempted five PRs
that need no exemption — measured while choosing it, and not what §3 of the log
shows; §3 contrasts the shipped floor with a floor this repository does not
carry.

**The exempted set cannot grow.** PMAT-535 refuses that shape at push time, and
it is the only way a record like #532 was made.

**A floor this repository does not carry exempts NOTHING.** A typo, a shallow
clone or a fixture repository all make the gate stricter rather than looser,
because an exemption that cannot be found is not an exemption — and the
direction that hides a defect must never be reached by accident.
`a_floor_this_repository_does_not_carry_exempts_nothing` pins it, and
`the_shipped_floor_is_the_commit_it_says_it_is` reads the constant out of the
script and resolves it against the real repository, because a floor naming a
commit that is not here would silently stop exempting anything — safe, but not
what the comment beside it claims.

## Falsification, and what each case is worth

Nine cases, and **nine killers**. Five die when the arm is removed (log §4).
The other four are over-refusal guards and cannot die from removing an arm they
assert does not fire — so each is killed by a mutation that makes it fire
wrongly. Measured, not argued (log §5, matrix in §6):

| mutation | what fails |
|---|---|
| M1 compare raw ids, never resolving `alias:<id>` | `a_claim_that_resolves_through_an_alias_agrees` |
| M2 drop the "only when the commits claim SOMETHING" guard | `a_merge_commit_that_claims_nothing_is_not_this_arms_finding` |
| M3 make the match arm unreachable, refusing every PR | the alias case, `a_pr_whose_commits_claim_its_own_ticket_passes`, `the_arm_reads_a_claim_gits_own_parser_cannot_see` |
| M4 read the trailers with git's own parser | both mismatch cases and `a_floor_this_repository_does_not_carry_exempts_nothing` |

The 19 cases of `falsification_dogfood_harness_and_quorum` stay green throughout,
which is the other half of the claim: nothing the gate already did has changed.

## What this branch pays

```
the whole diff:            gates=C,D
the diff without .github/: gates=none
```

The single `.github/` line is this suite being added to the guard job's list.
Without it the branch would be the first `gates=none` PR since PMAT-542 landed —
21.3 of 25.2 critical-path minutes, not paid. It is paid on purpose: the
integration suites DO run in CI under the coverage job (`cargo llvm-cov` builds
and runs every test binary), so naming this one buys **ordering** — a fast red
before the slow jobs — and not coverage. Its sibling is already in that list.

## Gaps, named

- **A merge commit that claims NOTHING passes.** The arm enforces "does not
  claim a DIFFERENT one", not "claims this one": a commit with no
  `Pmat-Ticket:` line at all is the commit-msg hook's finding and CB-2113's. A
  review lane read CLAUDE.md's promise that the PR is filed under a ticket its
  merge commit *claims*, found this guard, and was right that the two disagreed
  — the promise was what was wrong, and both CLAUDE.md and the skill now say
  "does not contradict". None of the thirty PRs measured is in that shape.
- **A commit claiming TWO tickets where the PR names one passes.** A lane put
  it plainly: a merge commit claiming `PMAT-520 PMAT-531` under a PR titled
  PMAT-520 satisfies the arm while PMAT-531's work is still credited to the
  wrong window. That is deliberate — a release cut legitimately carries several
  tickets — but it is a hole in the same wall.
- **If the PR and its commits agree on a lie, it passes.** The arm measures
  agreement, not truth.
- **The BODY is still not checked against the commits.** The rule falls through
  to the body only when branch and title name nothing; the arm compares what the
  rule RESOLVED, so a body-resolved id is covered — but a PR whose body names a
  second, different ticket that the rule never reaches is not.
- **The arm runs at gate A, which is post-merge.** It refuses the RELEASE, not
  the merge: `make dogfood-release` goes red and the record must be fixed before
  a tag. Refusing at merge time would need a required check that reads the merge
  commit, which does not exist until the merge happens.
- **A merge commit whose trailers were rewritten after the fact would pass.**
  The gate reads what is in the commit now.
- **`TRAILER_FLOOR` is a bare commit hash in a shell script.** It is asserted to
  resolve, and the reason beside it names the single record it exempts, but
  nothing asserts that record is still the only one below it.

IMPL-PMAT-540-RECEIPT-END
