# Implementation receipt — PMAT-542 — the PR lane selects the gate the change can move

verdict: PASS — the dogfood job's release build and its two binary-measuring gates now run only when the diff can move them. Measured over the 25 PRs merged as of cddf78cd: 10 stop paying 21.3 of a 25.2-minute critical path, which becomes 6.3 — 75% faster on 40% of PRs, with nothing skipped and nothing lowered.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (one script, one workflow, 11 cases)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_pr_lane_selects_the_gate_the_change_can_move"  rerun_exit=0 (11 passed)  log=docs/audits/logs/PMAT-542-per-gate-selection.log §6
  cmd="cargo test --test falsification_pr_lane_runs_what_the_change_can_break"  rerun_exit=0 (13 passed)  log=§6
  cmd="the same 11 cases against origin/main's classifier"  rerun_exit=101 (0 passed, 11 failed)  log=§6
  cmd="scripts/ci/changed-class.sh over the 25 PRs merged as of cddf78cd"  rerun_exit=0  log=§4
  cmd="bashrs lint scripts/ci/changed-class.sh"  rerun_exit=1, 0 errors (0 errors on origin/main too)
  cmd="cargo clippy --all-targets -- -D warnings"  rerun_exit=0
  cmd="cargo fmt --all -- --check"  rerun_exit=0

## What PMAT-237 already does, and what it cannot

`scripts/ci/changed-class.sh` decides WHETHER the heavy jobs run, and it works:
**3 of the 25** PRs measured were `code=false` and skipped every one of them.

What it cannot decide is WHICH heavy gate. It emits one boolean for all of them,
so a change to `scripts/` or `tests/` — which genuinely needs the guard tests —
also paid for a release build and two gates that read a surface it cannot reach.

## The measurement

CI run 34670955910 on `main`:

| | |
|---|---|
| `dogfood` | **25.2 min** — the critical path on every `code=true` PR |
| `ci / test` | 6.3 min — the next-longest job |
| sum of all job time | 51.8 min |

Inside `dogfood`: build 9.0, gate C 6.2, gate D 6.1, guard tests 3.8. **21.3 of
the 25.2 minutes exist to measure the built binary.**

Gate C reads the built binary and `docs/audits/surface_audit.csv`. Gate D reads
the built binary, `README.md` and that same CSV. Neither sources a library;
§2 of the log quotes the lines.

The 25 PRs merged as of `cddf78cd`, each run through the classifier as this
branch implements it:

| | count |
|---|---|
| `code=false` — already skipped everything | 3 |
| can move gate C or D | 12 |
| **cannot** — paid 21.3 min for nothing | **10** |

Their critical path becomes `ci / test` at 6.3 min: **75% faster, on 40% of all
PRs.**

## A number this receipt got wrong

The first scan said **fourteen**. It used a narrower trigger set than the
classifier that shipped — it did not count `.github/**` or `scripts/dogfood/**`
as reaching a gate. The shipped classifier is the conservative one, and the
figure against it is **ten**. §4 of the log is that run; the code comments, the
roadmap row, the commit message and issue #542 all say ten, and the issue says
what it used to say.

## Two booleans, not one string

The workflow selects with `needs.classify.outputs.gate_c == 'true'`, not
`contains(needs.classify.outputs.gates, 'C')`.

GitHub's `contains(search, item)` is a **case-insensitive substring test**.
`contains('none', 'C')` is false today only because the word `none` happens not
to contain the letter `c` — rename a gate to `N` and the summary word would
select it, in the direction that SKIPS a measurement. So the script emits
`gate_c=`/`gate_d=` for the workflow to compare exactly and `gates=` as a
readable token for the run summary, and
`the_booleans_and_the_summary_token_are_one_decision` re-derives each from the
other so two spellings cannot become two decisions.

## This is not a `--skip`

`CLAUDE.md`: *an UNMEASURED check is a FAILING check: a gate that could not run
and a gate that passed must never print the same thing.* Four things hold that
line, each with a case:

1. A gate the lane did not select prints `GATE C NOT-SELECTED <what it reads> —
   measured in full by make dogfood-release before the tag`. Never `PASS`.
2. `ci / gate` refuses a skipped `dogfood-surface` whenever the selection says
   it should have run. That job skips on its OWN condition rather than on the
   class, so the gate re-reads the selection the same way PMAT-237 made it
   re-read the class.
3. The script prints `none` and **never the empty string**, and the gate refuses
   an empty selection — because an empty value cannot be told apart from a
   classifier that did not run.
4. The selection is an **allow-list of the harmless**, like the class: an
   unclassified path selects every gate. Being wrong that way costs 21 minutes;
   being wrong the other way ships a surface nothing measured.

`make dogfood-release` is untouched and still runs A–H and T over the whole
window before any tag. This is a latency change, not a coverage change.

## Falsification

24 cases over one shared fixture, `tests/changed_class_fixture/mod.rs`, so
PMAT-237's suite and PMAT-542's cannot drift apart about what the classifier's
output means.

**All 11 of the new cases are RED against `origin/main`** (log §6): reverting
the three non-Rust files leaves a classifier that prints no selection at all,
and `field()` refuses it by name — *"the classifier printed 0 `gates=` lines for
["src/core/mod.rs"]; exactly one is the contract"*. PMAT-237's 13 stayed green
throughout, which is the other half of the claim: the class is unchanged.

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
  `README.md`-only change selects D alone and still rebuilds the binary, because
  gate D runs it. Splitting the build from the gates would need a cached
  artifact, which is a different ticket.
- **`.github/**` selects both gates conservatively.** A change to
  `coverage.yml` cannot move gate C, and 4 of the 25 PRs measured pay for that.
  Narrowing it means naming which workflows define the dogfood job, which is one
  more place to be wrong; the cost of being conservative here is 21 minutes and
  the cost of being wrong is a surface nothing measured.
- **The selection is not measured end to end until a PR lands.** These cases
  drive the script and read the workflow's text; that a GitHub `if:` evaluates
  as the cases assume is checked by CI on this PR and by the first receipt-only
  PR after it, not by a test.
- **No case proves the saving.** The 25-PR run in §4 is a measurement, not an
  assertion: nothing turns red if the figure drifts to 5 or 20. It is a
  description of the fleet, and the fleet is allowed to change.

IMPL-PMAT-542-RECEIPT-END
