# Implementation receipt — PMAT-241 — every tagged release names the cookbook it was qualified against

verdict: PASS — from `cookbook_floor` (v1.29.0) every row of `docs/roadmaps/releases.yaml` names the paiml/forjar-cookbook commit the release was qualified against, and gate T refuses a row that names none, names a branch, names a commit the cookbook does not carry, or names one whose `Cargo.toml` cannot admit the version that shipped under Cargo's own requirement rule. `release-goal.sh window` takes the commit from `git ls-remote` at the moment of the cut, so `cut` books it. Two rounds of review lanes ran; both returned FAIL and every refutation reproduced when re-run here.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: release
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=agy-goal  w=1.00  basis=absent  effort=1[U]
  ph1.quorum  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (three lanes, all FAIL)
  ph2.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (three lanes, all FAIL)
  ph3  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_release_cookbook_is_part_of_the_release"  claimed_exit=101(lanes)  rerun_exit=0  log_path=docs/audits/logs/PMAT-241-lane-findings.log
  cmd="cargo test --test falsification_release_goals_are_measured"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-241-caret.log
  cmd="bash scripts/dogfood/tagged.sh"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-241-gate-tests.log

## What the ticket is about

paiml/forjar-cookbook is where forjar is USED rather than described. Gate D
validates its configs against the built artifact on every dogfood run, and
`make dogfood-published VERSION=x.y.z` does it against what crates.io serves,
so the cookbook IS exercised. Nothing recorded WHICH cookbook that was.

Measured: the cookbook's master is `7c100454`, dated 2026-08-29T15:05:21Z, and
**ten** `v*` tags have been cut since, counted in seconds from the tagger
dates. Ten releases claimed to have been dogfooded against a cookbook nobody
named, and nothing required the cookbook to move with them.

`cookbook_floor` is v1.29.0 rather than the existing floor. Retrofitting the
ten releases whose cookbook commit nobody recorded would be inventing the
record, not keeping it.

## The requirement is read the way Cargo reads it

The interesting half of the arm is not "does the commit resolve" but "can the
cookbook at that commit USE the version that shipped". The cookbook carries
`forjar = { version = "1.2", default-features = false }`, which is a CARET:
`>=1.2.0, <2.0.0`.

The first implementation compared with `dogfood_semver_ge`, and

    dogfood_semver_ge v2.0.0 v1.2   ->   true

which is a false green at exactly the release where this arm matters most: the
one that BREAKS the cookbook. That was measured here before the lanes reported
it, and three lanes reported it independently.

`dogfood_req_admits` in `scripts/dogfood/lib/releases.sh` now applies Cargo's
rule, operator by operator:

| requirement | admits | ceiling |
|---|---|---|
| `^1.2`, `^1.2.3`, `1.2` | `>=1.2.0` | 2.0.0 |
| `^0.2`, `^0.0.3`, `^0`, `^0.0` | `>=` the given version | 0.3.0, 0.0.4, 1.0.0, 0.1.0 |
| `~1.2`, `~1.2.3` | `>=1.2.0` | 1.3.0 |
| `~1` | `>=1.0.0` | 2.0.0 |
| `=1.2.3` | exactly it | 1.2.4 |
| `=1.2`, `=1` | `>=` the given version | 1.3.0, 2.0.0 |

Exit 0 admits, 2 is below the requirement, 3 is at or past the ceiling, 4 is a
version the rule refuses to evaluate. Two and three get different sentences
because a cookbook that is BEHIND the release and one that is BROKEN by it are
different problems with different fixes.

## Two rounds of lanes, six refutations, every one re-run

Round one (three lanes, all FAIL) refuted five claims. Round two (three lanes,
all FAIL) refuted three more, unanimously, and confirmed two. **Every
refutation was reproduced here before it was accepted**, and the reproduction
is what the fix was written against:

1. **`sort -V` is not Cargo's predicate.** Measured: `v2.0.0 >= v1.2` is true.
   Fixed by the table above.
2. **The `sed` requirement parser could not read `^1.2` at all**, so the gate
   said "declares no forjar version requirement" — a false statement about the
   cookbook, and about the spelling Cargo writes by default.
3. **`forjar =` matched in any table**, so a `[dev-dependencies]` entry was
   read as the real one. The parser is now section-aware.
4. **A trailing comment was read as the requirement.** Measured:
   `forjar = "1.2" # version = "2.0"` produced `2.0`. awk now cuts the line at
   the first `#` outside a string, the way TOML reads one.
5. **`~` and `=` were STRIPPED and evaluated as carets.** Measured: `=1.2.3`
   admitted everything below 2.0.0. Cargo admits exactly 1.2.3. Reading an
   operator as a caret is always WIDER than Cargo, which is the direction that
   produces a false green. The operator now reaches the rule.
6. **Neither side was validated.** Measured: `$((10#3-9 + 1))` is `-5`, so
   `^0.0.3-9` produced an upper bound of `0.0.-5` and admitted 0.0.4;
   `0.0.3.4` was read as `0.0.3`; `01.0.0` was evaluated at all; and a
   pre-release sorts ABOVE its release under `sort -V`. The released version
   was never checked either, and it comes from the tag. Both sides are now one
   to three numeric components with no leading zeros, and anything else is
   REFUSED by name rather than measured wrong.

Two claims were CONFIRMED by all three lanes of round two: the ten-tag count
(each lane measured it independently from the repository's own tags), and that
nothing in the diff can make gate T pass where it previously failed.

## The test measures the gate, not the gate's formatting

The rule lives in `scripts/dogfood/lib/releases.sh` beside `dogfood_semver_ge`
rather than in `tagged.sh`, and that is what lets the falsifier SOURCE it. The
first version of the test sliced the function out of the script with
`sed -n '/^caret_admits()/,/^}/p'`, and a lane showed both directions of
failure: writing `caret_admits () {` with one extra space makes the slice empty
and the test red while the gate is fine, and a function inside a string literal
would be extracted and pass while the gate is broken. A test whose subject
depends on the formatting of the file it reads is measuring the formatting.

The table is 35 rows across all three operators and every input a lane broke
the old rule with. It asserts the function is DEFINED before it calls it: a
missing function comes back 127, and `|| return 2` inside the rule would read
that as "below the requirement" — a measurement of nothing, dressed as a
verdict.

## Falsification

`tests/falsification_release_cookbook_is_part_of_the_release.rs`, eight cases:
a row at the floor with no cookbook is red; a branch name instead of a commit
is red; a commit the cookbook does not carry is UNMEASURED and red; a release
below the floor is not asked for one; the requirement is read the way Cargo
writes it (caret, dev-dependency, multi-clause); a cookbook that cannot build
against the release is red and says so; the operator reaches the rule and a
comment does not; and the 35-row rule table.

Red/green proof, both rounds, in `docs/audits/logs/PMAT-241-lane-findings.log`
and `docs/audits/logs/PMAT-241-caret.log`: the new cases go red against the
gate as it was, and green against the gate as it is.

Against the real repository, `bash scripts/dogfood/tagged.sh` exits 0: six
tagged releases reconcile, 26 tickets carry their tag and say they shipped, 11
tickets from 8 PRs carry `release:v1.29.0`, due 2026-09-12T16:07:14Z. The
cookbook arm does not fire, because no tag at or above v1.29.0 exists yet —
which is the floor doing what it says.

Exercised against the real cookbook by hand: `7c100454` requires `1.2`, which
admits 1.29.0 and 1.28.0 and refuses 2.0.0.

## A red proof destroyed uncommitted work, again

`git checkout HEAD -- <path>` was run to RESTORE the tree after a red proof
while the fixes were still uncommitted. HEAD was the commit before them, so the
restore reverted them. The work was replayed from the session's own edit
scripts and the second round was then run from a committed state, which is the
rule this repository already learned once: **commit before the red proof, and
check out the named parent, never `HEAD`, to go back.**

## Gaps, named

- The arm reads only `[dependencies]` and `[workspace.dependencies]` in the
  cookbook's ROOT `Cargo.toml`. A member crate that pins forjar differently is
  not read. The cookbook is a workspace of three crates and this was not
  measured against each of them.
- `forjar.workspace = true` and a multi-line inline table are not handled; the
  first produces "declares no forjar version requirement" and the second may
  miss the version. Both are red, not falsely green, but the message would be
  wrong for the first.
- A multi-clause requirement (`>=1.2, <1.9`) is refused by name rather than
  evaluated. That is a gate that says what it cannot do, not one that does it.
- Nothing yet makes the cookbook's own CI run against an unreleased forjar, so
  "the cookbook is bumped as part of the cut" is a written rule with a gate
  behind it, not an automated bump.

IMPL-PMAT-241-RECEIPT-END
