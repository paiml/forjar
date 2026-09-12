# Implementation receipt — PMAT-542 — the PR lane selects the gate the change can move

verdict: PASS — the dogfood job's release build and its two binary-measuring gates now run only when the diff can move them. Measured over the 25 PRs merged as of cddf78cd: 7 stop paying 21.3 of a 25.2-minute critical path, which becomes 6.3 — 149 job-minutes over that window, with nothing skipped and nothing lowered. A three-lane review round found the first version of the selection FAILED OPEN on the gates' own fixtures; that is fixed and is the ticket's most valuable finding.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (one script, one action, one workflow, 14 cases)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_pr_lane_selects_the_gate_the_change_can_move"  rerun_exit=0 (14 passed)  log=docs/audits/logs/PMAT-542-per-gate-selection.log §6
  cmd="cargo test --test falsification_pr_lane_runs_what_the_change_can_break"  rerun_exit=0 (13 passed)  log=§6
  cmd="both suites against origin/main's three files"  rerun_exit=101 (14 of 14 red, and 1 of PMAT-237's 13)  log=§6
  cmd="scripts/ci/changed-class.sh over the 25 PRs merged as of cddf78cd"  rerun_exit=0  log=§4
  cmd="bashrs lint scripts/ci/changed-class.sh"  rerun_exit=1, 0 errors (0 errors on origin/main too)
  cmd="cargo clippy --all-targets -- -D warnings"  rerun_exit=0
  cmd="cargo fmt --all -- --check"  rerun_exit=0
  review: 3 agy lanes, 3 × FAIL, 31 findings; the fail-open hole, the HEAD~1 fallback, the substring assertions and four false sentences acted on below

## What PMAT-237 already does, and what it cannot

`scripts/ci/changed-class.sh` decides WHETHER the heavy jobs run, and it works:
**3 of the 25** PRs measured were `code=false` and skipped every one of them.

What it cannot decide is WHICH heavy gate. It emits one boolean for all of them,
so a change to `scripts/` or `tests/` — which genuinely needs the guard tests —
also paid for a release build and two gates that read a surface it cannot reach.

## The measurement

CI run 34670955910 on `main`: `dogfood` is the critical path at **25.2 min**,
against 6.3 for the next-longest job (`ci / test`); the sum of all job time is
51.8. Inside it: build 9.0, gate C 6.2, gate D 6.1, guard tests 3.8 — 25.1 of
steps plus 0.1 of setup and checkout. **21.3 of the 25.2 minutes exist to
measure the built binary.**

The 25 PRs merged as of `cddf78cd`, each run through the classifier as this
branch implements it:

| | count |
|---|---|
| `code=false` — already skipped everything | 3 |
| `gates=C,D` — unchanged | 15 |
| `gates=C` / `gates=D` — one gate saved | 0 |
| **`gates=none`** — no build, no gate | **7** |

**28% of PRs**, whose critical path becomes `ci / test` at 6.3 min — 75% faster
— and **149 job-minutes** over that window.

## What the gates actually read, and the hole a review round found

The first version of this arm said gate C reads the binary and
`docs/audits/surface_audit.csv`, gate D the binary, `README.md` and that CSV,
*"Nothing else."* **Two lanes measured it and refuted it**:

```
surface.sh:50   BIN="$(bash "$(dirname …)/lib/binary.sh")"
surface.sh:277  FIXTURE="tests/fixtures/dogfood/local-files.yaml"
docs.sh:41      FIXTURES="tests/fixtures/dogfood"
docs.sh:62      BIN="$(bash "$(dirname …)/lib/binary.sh")"
docs.sh:279     open("Cargo.toml")
```

All of that lives under `scripts/` and `tests/`, which the harmless arm names.
**A PR editing the very fixture gate C compares two evaluations of would have
skipped gate C** — the fail-open direction, which ships an unmeasured surface.
No test caught it; a lane reading the two scripts did.

The selecting set is now `scripts/dogfood/lib/*`,
`scripts/dogfood/surface.sh` (C), `scripts/dogfood/docs.sh` (D) and
`tests/fixtures/*`. `every_path_the_two_gates_name_is_selected_by_the_classifier`
re-derives that closure from the two scripts on every run, so a gate that starts
reading a new fixture turns the suite red rather than blinding itself — the same
shape as PMAT-237's `every_record_path_a_test_reads_is_classified_as_code`.

Selecting the WHOLE of `scripts/dogfood/` was tried first and measured: it takes
the saving from 7 PRs in 25 to **2**, because this repository develops
`harness.sh`, `tagged.sh`, `comply.sh` and the rest constantly and neither gate
reads any of them. Over-selecting is the cheap mistake, not the free one.

## The three numbers, in order

**14** was a scan before any of this existed, from a trigger set too narrow.
**10** was the first implementation, measured. **2** was the first fix for the
fail-open hole, which over-corrected. **7** is the closure derived by reading
the two scripts. §7 of the log carries each with its reason. Every revision came
from a measurement; the one that mattered came from the review round.

## Two booleans, not one string

The workflow selects with `if: needs.classify.outputs.gate_c == 'true'`, not
`contains(needs.classify.outputs.gates, 'C')`. GitHub's `contains` is a
**case-insensitive substring test**: `contains('none', 'C')` is false today only
because the word `none` happens not to contain the letter `c`, and renaming a
gate to `N` would make the summary word select it — in the direction that SKIPS
a measurement. `the_booleans_and_the_summary_token_are_one_decision` re-derives
each representation from the other so two spellings cannot become two decisions,
and `the_workflow_compares_the_selection_exactly` is anchored on `if: ` rather
than on the bare expression, because a comment carrying the same words would
have satisfied a bare substring test. A lane made that point.

## An absent base is unmeasured

The action fell back to `HEAD~1` when it had no PR base and no `event.before`,
so a `workflow_dispatch` re-run of a twenty-commit branch was classified from
ONE commit — narrowing the diff, which selects too little. Found by a lane.
It now hands the empty file list to the classifier's own rule that *no readable
file list is not a licence to skip anything*, which is `code=true` and
`gates=C,D`. This is a PMAT-237 defect, fixed here because it is four lines in
the file this ticket is already changing.

## This is not a `--skip`

`CLAUDE.md`: *an UNMEASURED check is a FAILING check: a gate that could not run
and a gate that passed must never print the same thing.* Four things hold that
line, each with a case:

1. A gate the lane did not select prints `GATE C NOT-SELECTED <what it reads> —
   measured in full by make dogfood-release before the tag`. Never `PASS`.
2. `ci / gate` refuses a skipped `dogfood-surface` whenever the selection says
   it should have run. That job skips on its OWN condition rather than on the
   class, so the gate re-reads the selection the way PMAT-237 made it re-read
   the class.
3. The script prints `none` and **never the empty string**, and the gate refuses
   an empty selection — an empty value cannot be told apart from a classifier
   that did not run.
4. The selection is an **allow-list of the harmless**: an unclassified path
   selects every gate.

`make dogfood-release` is untouched and still runs A–H and T over the whole
window before any tag. This is a latency change, not a coverage change.

## Falsification

27 cases over one shared fixture, `tests/changed_class_fixture/mod.rs`, so
PMAT-237's suite and PMAT-542's cannot drift apart about what the classifier's
output means.

**All 14 new cases are RED against `origin/main`** (log §6): reverting the three
non-Rust files leaves a classifier that prints no selection at all, and
`field()` refuses it by name. One of PMAT-237's 13 goes red too, which is honest
rather than incidental — `every_heavy_job_runs_only_when_the_change_can_reach_it`
reads job names out of the workflow, and the revert puts back a job called
`dogfood`. The other 12 stay green: the CLASS is unchanged.

The suite was split rather than grown: the combined file reached 653 lines, and
`tests/falsification_pr_lane_runs_what_the_change_can_break.rs` is now 372.

## This branch pays in full

```
$ git diff --name-only origin/main...HEAD | bash scripts/ci/changed-class.sh
code=true  gate_c=true  gate_d=true  gates=C,D
```

`.github/**` is unclassified for the selection, so a change to the lane itself
runs both gates. The PR that changes the rules does not get to skip the gates it
is changing them for.

## Gaps, named

- **The 9-minute build is still paid whenever either gate is selected.** A
  `README.md`-only change selects D alone and still rebuilds the binary. Cutting
  that needs a cached artifact, which is a different ticket.
- **`.github/**` selects both gates conservatively**, and 4 of the 25 PRs pay
  for it. Narrowing it means naming which workflows define the dogfood job,
  which is one more place to be wrong.
- **`scripts/dogfood/lib/*` selects both**, though only `binary.sh` is read
  today. The directory holds four files and the prefix is insurance against a
  `source` line added later that no test would otherwise notice.
- **The selection is not measured end to end until a PR lands.** The cases drive
  the script and read the workflow's text; that a GitHub `if:` evaluates as they
  assume is checked by CI on this PR and by the first receipt-only PR after it,
  not by a test.
- **No case proves the saving.** §4's 25-PR run is a description of the fleet,
  not an assertion: nothing turns red if the figure drifts to 3 or 15.

IMPL-PMAT-542-RECEIPT-END
