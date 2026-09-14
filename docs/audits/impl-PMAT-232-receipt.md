# Implementation receipt — PMAT-232 — publish-release publishes a draft whoever created it

verdict: PASS — `publish-release` un-drafted the release only when its own run created it, and the assertion that it had done so sat behind the same guard, so a release whose creating run failed after `create-release` could never be published by the workflow; v1.28.0 was un-drafted by hand because of it. The job now decides from the release's own state: it publishes a draft, leaves a published release exactly as it is, and asserts unconditionally that the release is not a draft when it finishes. Rule 5 pins all three properties, is red against main (7 passed / 1 failed) and green here (8 passed), and three mutations each turn it red. Not claimed: that the job has been observed doing this. It cannot run outside a release and will first run for real at the v1.29.0 cut.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: release
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=agy-goal  w=1.00  basis=absent  note=fable-binding  effort=1[U]  (executed by self: workers and lanes may not edit .github/workflows)
  ph1.quorum  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate, three lanes)
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_release_workflow_shape, with main's release.yml checked out (RED: 7 passed, 1 failed) then at HEAD (GREEN: 8 passed)"  claimed_exit=101(lanes)  rerun_exit=101/0  log_path=docs/audits/logs/PMAT-232-gate-tests.log  sha256=127943dbfef4ef21
  cmd="three mutations of publish-release: the created guard restored, the assertion removed, the edit made unconditional"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-232-rule5-mutations.log  sha256=e47d4486bf4e92c8

## What was wrong

```
if [ "${{ needs.create-release.outputs.created }}" = "true" ]; then
  gh release edit "$RELEASE_TAG" --draft=false --prerelease ...
fi
...
if [ "${{ needs.create-release.outputs.created }}" = "true" ] && [ "$state" != "prerelease=true draft=false" ]; then
```

Both halves behind the same guard. In run 34530301814 every asset job succeeded, `publish-release` reported `success`, and v1.28.0 stayed a draft: the run that created the release had died in `dist-artifacts`, and the re-dispatch that carried the fix did not create anything. By then the release was complete — fourteen assets, installer included — and invisible to everyone but its author, because a draft is.

## What it does now

Reads the release, edits **only** when the state ends `draft=true`, re-reads, and exits non-zero with an `::error::` if it is still a draft. Three properties, all pinned by rule 5:

| property | why |
|---|---|
| the edit is conditional on `draft=true` | a re-dispatch over a release an operator promoted to a full release must not demote it back to a prerelease |
| the assertion is unconditional | a job that reports success while doing nothing is the whole of this defect |
| the assertion is `draft=false`, not `prerelease=true draft=false` | asserting `prerelease=true` would fail on that correctly-promoted release; rule 4 still asserts it for a run that creates one, which is the only job that can know |

What the old guard protected is kept, decided by reading the release instead of by asking who created it.

## Falsification

Red against main and green here (`docs/audits/logs/PMAT-232-gate-tests.log`). Three mutations, each applied alone (`docs/audits/logs/PMAT-232-rule5-mutations.log`): the `created` guard restored, the end-state assertion removed, the edit made unconditional — all three RED.

**The first version of rule 5 caught only one of the three.** It asserted that `draft=false` appears in the run block, which the `gh release edit --draft=false` command satisfies by itself. The rule now requires the `draft=true` test to come before the edit, and an `::error::` with `exit 1` after it. That is the second time in two tickets that the mutation battery has found a rule too weak to be worth having, and it is why the battery is run before the review rather than after.

## Review record

Three sandboxed lanes: all eleven claims confirmed, nine by all three. The FAIL verdicts came from the hostile-reader half of the brief and they were right twice:

- the new comment claimed the re-dispatch left "thirteen assets and no installer". It left fourteen, `dist-artifacts` having succeeded six minutes earlier. Thirteen-and-no-installer was the first run's state.
- PMAT-200's note claimed the tap step's empty token stands out "while every other step in the same job shows `GH_TOKEN: ***`". Only one sibling step declares a token at all.

Both corrected. The lanes also did the one thing no rule here can: they worked out by hand which of the three possible `state` strings takes which branch, and what the job does when `gh` fails or returns nothing — the step runs under `bash -e`, an empty or unexpected state falls to the catch-all and exits 1. Rule 5 reads the job's text and cannot execute it, so that reasoning is the only runtime evidence this change has.

One lane refuted the actionlint count as 11 rather than 22; re-run on both trees, it is 22 on each, and it is recorded as a lane error rather than dropped.

## Triage in the same change

`pmat work edit PMAT-233 -s cancelled`: a duplicate of PMAT-200, which already carries the missing `HOMEBREW_TAP_TOKEN` as an operator decision. PMAT-200 absorbs PMAT-233's measurement — the first sight of that decision from inside the job, now that PMAT-230's fixes carry the step far enough to reach it.

## Gaps, named

- The job has not been observed doing this. It first runs for real at the v1.29.0 cut, due 2026-09-12T16:07:14Z.
- `create-release` still exposes `created` as a job output and nothing consumes it. Kept deliberately: rule 3 requires it, and it is the only record in a run of which run created the release.
- PMAT-234 is still open: gate R's verdict line says `pre-tag` after every successful release.
- `homebrew` still cannot push (PMAT-200). It is not on `publish-release`'s `needs` and has never blocked a release.

IMPL-PMAT-232-RECEIPT-END
