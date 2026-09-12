# Implementation receipt — PMAT-535 — a branch names the ticket its work claims

verdict: PASS — the quorum gate refuses, at push time, a branch that names a ticket no commit being pushed claims. The branch name is read by three gates and was checked by none; the defect it produced is still visible on main, where PMAT-520 carries two release labels and nobody put the second there by hand. A three-lane review round refuted four sentences of the first version and found two false refusals; both were reproduced, fixed, and are now regression cases.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (one gate arm, eight cases)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_a_branch_names_its_own_ticket"  claimed_exit=-  rerun_exit=0 (8 passed)  log=docs/audits/logs/PMAT-535-branch-names-its-ticket.log §7
  cmd="cargo test --test falsification_quorum_gate_{reads_the_pushed_ref,has_a_triage_shape} --test falsification_quorum_anchors_release_shaped --test falsification_dogfood_scripts_declare_mutations"  claimed_exit=-  rerun_exit=0 (5+8+4+4)  log=§8
  cmd="the gate against seven branch shapes, as a shell"  claimed_exit=-  rerun_exit=measured  log=§4 (red), §5 (green), §6 (arm deleted)
  cmd="bashrs lint scripts/quorum-gate.sh"  claimed_exit=-  rerun_exit=1, 0 errors — and 1 with 0 errors on origin/main too  log=§9
  review: 3 agy lanes, verdicts do-not-implement-as-written / FAIL / FAIL, 29 findings; 4 refutations and 2 measured false refusals acted on below

## What the branch name decides

| gate | resolves from it |
|---|---|
| A | `docs/audits/impl-<ticket>-receipt.md`, the receipt path |
| E | `.quorum/<slug>.json`, this receipt's own filename |
| T | the RELEASE WINDOW a merged PR belongs to |

Three gates read one unvalidated string.

## What it produced

PR #532 was pushed from `PMAT-520-book-v1.29.0`. Its merge commit `b4719737`
carries `Pmat-Ticket: PMAT-531`; its title says PMAT-531; its receipt says
PMAT-531.

Every window arm resolved it to **PMAT-520** — which had already SHIPPED in
1.29.0 — so gate T demanded `release:v1.30.0` on it, `release-goal.sh sync`
obligingly added the label, and PMAT-520 now claims two releases on main
(`docs/roadmaps/roadmap.yaml:3655-3656`). The ticket that actually owned the
work was invisible to all three gates.

**Nobody added that label by hand.** The gate demanded it, and the gate was
doing exactly what it was told.

## The rule, and what it is not

If the branch names a ticket, **some commit being pushed must claim it**.

Not "every trailer equals the branch's id" — a branch legitimately carries
commits for more than one ticket, which this repository does on every release
cut. What it must not do is name one that none of them claims.

And not "every branch names a ticket": `fix/ci-lint` is the naming this
repository's own guidelines suggest for work with no ticket, and refusing it
would be this arm inventing a requirement nobody asked for.

**Only when the commits claim something.** A branch whose commits carry no
`Pmat-Ticket` at all is a different defect with a different owner — the
commit-msg hook judges it, CB-2113 judges it again — and an arm that also
refused it would be reporting someone else's finding in its own words.

At push time, because after the push the name is in three gates' arithmetic and
a merge commit cannot be renamed.

## Why `sed` and not git's trailer parser — the corrected reason

The first version of this arm justified `sed` by saying *"pmat's own CB-2113 and
this repository's commit-msg hook both match the LINE wherever it appears."*
**Two review lanes refuted that sentence and they were right.** Measured:

- **CB-2113 asks git.** `src/services/commit_traceability/mod.rs:275,320` build
  `--format=…%(trailers:key=Pmat-Ticket,valueonly,separator=…)`.
- **The commit-msg hook asks git first**, with `git interpret-trailers --parse`,
  and on a miss falls back to `grep -qE 'PMAT-[0-9]+|#[0-9]+'` over the whole
  message — so *any* ticket-shaped string anywhere satisfies it. That fallback,
  not a trailer, is how every multi-paragraph commit in this session passed.

The real reason is better and is measured on the defect itself. `b4719737`,
the merge commit of PR #532:

| asked | answer |
|---|---|
| `git log -1 --format='%(trailers:key=Pmat-Ticket,valueonly=true)'` | *(nothing)* |
| `git log -1 --format=%B \| sed -n 's/^Pmat-Ticket:…//p'` | `PMAT-531` |

git's parser reads the last paragraph only, and that squash message ends with
bullet paragraphs. **An arm built on git's parser would have read "this branch
claims nothing", taken the skip, and let PR #532 through** — the exact push it
exists to refuse. CB-2113 saw nothing there for the same reason.

Reading the line is therefore strictly *more permissive* than either tool: this
arm can refuse only a branch whose commits demonstrably name some other ticket,
never one those two consider unlabelled.

## Three defects the review round found, and what they cost

**1. CRLF was a false refusal (measured).** A commit message with CRLF line
endings left `\r` glued to the id, so `PMAT-535\r` did not match `PMAT-535` —
and the gate refused the branch *while printing the very trailer that named it*:

```
✗ QUORUM GATE: branch 'PMAT-535-crlf' names PMAT-535 and no commit being pushed claims it.
     Trailers on this branch: PMAT-535
```

**2. A comma-separated trailer was a false refusal (measured).**
`Pmat-Ticket: PMAT-535, PMAT-536` is two claims; unsplit, the first read as
`PMAT-535,` and matched nothing.

Both are reproduced in §4 of the log against the arm exactly as committed at
`9c2426af`, and both are now cases in the suite.

**3. `git log` failing was a silent pass.** The read was `git log … 2>/dev/null`
with no exit check: a failure produced an empty string, which the arm reads as
"claims nothing" and skips. That is an UNMEASURED check printing what a passing
check prints, which this repository forbids. It is now `|| die`.

The fix normalises before matching: strip `\r`, split on commas, allow a leading
indent, match the key case-insensitively. Each of those is looser than git and
looser than the hook, so none of them can add a refusal those tools disagree
with.

## Falsification

`tests/falsification_a_branch_names_its_own_ticket.rs`, eight cases.

**Two are positive and die if the arm is deleted** (measured, log §6 — with the
arm's lines cut out, both branches fall through to the receipt arm and the
`contains` assertions fail):

- `a_branch_named_for_a_ticket_no_commit_claims_is_named_and_red` — the exact
  shape PR #532 was in; asserts the refusal names PMAT-520, names PMAT-531, and
  says `RELEASE WINDOW`, so renaming the trailer instead of the branch is not a
  fix the message invites.
- `the_arm_refuses_on_a_trailer_gits_own_parser_cannot_see` — asserts *first*
  that git's parser returns nothing for the multi-paragraph shape, then that the
  arm refuses and names the ticket git could not see. If git's parser ever
  starts reading that shape, this case fails loudly rather than quietly stopping
  to test anything.

**Six are false-refusal guards.** They cannot die from deleting the arm —
they assert it does not fire — and a review lane called that vacuity. It is not:
they die when the arm fires *wrongly*, which is not hypothetical, because §4 of
the log is two of them RED against the version this branch had already
committed. Each also asserts the gate reached the `no quorum receipt` arm by
name, so a gate that died earlier for any reason fails them too. They cover: a
branch naming its own ticket; a branch carrying three tickets and naming one; a
branch with no ticket id; a branch whose commits claim nothing; CRLF; and the
comma-separated trailer.

## bashrs, narrowed to what is checkable

`bashrs lint scripts/quorum-gate.sh` exits **1** on this branch and **1** on
`origin/main`: bashrs returns non-zero whenever it has anything to say, so the
exit code carries no information about this change. ERRORS are **0** on both.

The arm adds 8 warnings and 10 infos. Log §9 prints each with the exact text at
the column bashrs named. Sixteen of the eighteen are quote-context artifacts:
six SC2086 on variables that *are* double-quoted (bashrs loses context inside
`$( … )` and inside a multi-line `die` string), BRS0023/SC2162/REL003 twice on
the English word **"read"** in prose, SC2230/SC2023 on the English word
**"which"**, SC2016/BRS0006 on `'$branch'` quoted for a reader. The remaining
two, SC2089 at lines 319 and 334, are the standard note that a single-quoted
`sed` script or `printf` format is literal — which is what those lines intend.

The first version of this receipt said *"All four are one artifact"* while
listing SC2101 on a case **pattern** among them. Three lanes quoted that
sentence back, and they were right. The pattern is gone — `[ -n "$claimed" ]`
replaced `*[![:space:]]*` — and SC2101 with it.

## Gaps, named

- **The arm cannot see a PR title.** At push time the PR does not exist, and
  gates A and T fall back to the title and then the body when the branch names
  no ticket. A branch called `fix/ci-lint` with a PR titled `(PMAT-520)`
  reproduces the whole defect and this arm never runs. Filed as a ticket; the
  natural home is gate A, which reads GitHub and can compare the three sources
  against each other.
- **It is a client-side pre-push hook.** `--no-verify`, a clone with no hooks
  installed, and a merge done in the GitHub web UI all bypass it. `.quorum`'s
  advisory mode also lets a non-enforced author push past every arm with a
  warning. This arm narrows a window; it does not close it.
- **A `Pmat-Ticket:` line quoted inside a commit body counts.** A message that
  quotes someone else's trailer in a code block claims that ticket as far as
  this arm is concerned. The direction is safe — it can only make the arm
  *accept* — but it means a determined misnaming passes.
- **The first `PMAT-<n>` in the branch name is the one judged.** A branch named
  `fix-PMAT-1-and-PMAT-2` is judged on PMAT-1 alone.
- **The `|| die` on `git log` has no falsifier.** Both of its arguments are
  commits this script has already validated, so the only way to reach it is
  repository corruption, and a test that forced it would have to break the
  earlier validations first. It is defence in depth, stated as such rather than
  claimed as tested.
- **PMAT-520's second label stays until someone decides.** The arm runs at push
  time; a branch already merged keeps its name. The ledger row is the record of
  what shipped, and the label is a gate's demand rather than a claim anyone made.

IMPL-PMAT-535-RECEIPT-END
