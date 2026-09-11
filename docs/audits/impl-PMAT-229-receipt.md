# Implementation receipt — PMAT-229 — the status line renders what it can and says what it cannot

verdict: PASS — `release-goal.sh show` shares gate T's window, and that window refuses when commits reach HEAD that no merged PR contains, which is every feature branch and therefore every place an operator reads the cadence: `make release-goal` printed no goal at all, not the tag, not the due instant, not the bar. It renders them now, prints `UNMEASURED` for the two counts that cannot be measured, and still exits non-zero. `scripts/dogfood/tagged.sh` is byte-identical to main's. The soft path is the third argument of `dogfood_prs_between`, not an environment variable — the first version read one, and two review lanes independently showed that an operator's exported shell variable would soften every gate; the hole was closed before they reported and a case pins it shut.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: release
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=agy-goal  w=1.00  basis=absent  note=fable-binding  effort=1[U]  (executed by self, shared branch with PMAT-228)
  ph2.quorum  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate, three lanes, shared with PMAT-228)
  ph3  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --no-fail-fast over the three suites, with main's scripts checked out (RED) then at HEAD (GREEN: 19 + 2 + 14)"  claimed_exit=101(lanes)  rerun_exit=101/0  log_path=docs/audits/logs/PMAT-228-gate-tests.log  sha256=recorded-in-the-log

## The distinction the change rests on

The due instant, the elapsed bar, the next tag and the `basis=` line do not depend on the commits that could not be measured. Only the merged and tagged counts do. Refusing to print any of it because one number is unmeasurable made the cadence unreadable exactly where work happens — and a cadence you cannot read from a feature branch is not a cadence anybody keeps.

The exit code is unchanged: still non-zero, so no script can read a degraded line as a pass. An UNMEASURED check is still a failing check; it is now a failing check that tells you what it did measure.

## What the gate does NOT get

`scripts/dogfood/tagged.sh` is byte-identical to main's, verified by `git diff main -- scripts/dogfood/tagged.sh` returning nothing. A release gate that rendered a degraded line would be a gate passing over an unmeasured window.

**The soft path is an argument, and that is the whole of the safety.** The first version read `${DOGFOOD_WINDOW_SOFT:-0}`. Two lanes independently refuted it: an operator who exports that name in a shell profile softens every gate that sources the library, silently. It is the third positional argument of `dogfood_prs_between` now, passed by `show` and by nothing else in the tree; an argument cannot be inherited. The question was in the brief I wrote for those lanes, and answering it before they did cost four lines.

## Falsification

- `the_status_line_renders_the_goal_when_the_merged_count_is_unmeasurable`: the fixture is the state that fired on PMAT-227's booking branch — the previous release's own PR is all GitHub reports, so the window is empty while a commit sits in it. It asserts the goal line, the word UNMEASURED, a non-zero exit, and that the gate over the same window is still red. Red against main's scripts, green here.
- `no_environment_variable_can_soften_the_gate`: runs gate T with `DOGFOOD_WINDOW_SOFT=1` and `SOFT=soft` exported and asserts it reaches a real verdict with no degraded line. **This one is green against main too, and says so rather than being presented as a falsifier**: main has no soft path at all. It is a regression guard, and it is the only kind of guard that can exist for a hole that was closed before it shipped.

## Review record

One round of three lanes shared with PMAT-228; two returned, one produced no output. Both returning lanes refuted the environment-variable claim — the hole above — and one refuted two claim wordings: that `window.sh`'s function changed "nothing else" (the soft path is in the same function), and that the gate-tests log existed (it did not at the commit they judged; it was written at the next one). All three are recorded in `.quorum/evidence/reporting-judges.md`.

## Gaps, named

- The lanes judged `cdd03571`; the hardening is `50073b89`, one commit later, and was verified by the orchestrator with its own case rather than by a second round. The tree they judged no longer exists as HEAD, and this receipt says so.
- One lane returned nothing.
- `show` prints the two counts as UNMEASURED together; it does not distinguish "no PRs merged" from "GitHub could not answer", which the reason line beside it does.

IMPL-PMAT-229-RECEIPT-END
