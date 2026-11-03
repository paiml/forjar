# Lanes — forjar#704

Lanes run with the ticket row and the full diff against main 9a37a2ff. None
used the author's model (claude-opus-5-5). Each lane was read-only. The two
Claude seats ran in one quorum-review.sh round; the agy seat
(gemini-3.8-flash-high) ran as its own lane through agy-lane.sh --mode plan
(see `704-agy.md`). Three distinct model ids over two families, so the review
round is not degraded.

Planted control at 192344f0 (302123a9 plus one commit, "apply_dry_run:
simplify scoped plan", that removes the `overlay_drift` call from
`scoped_dry_run_plan`, renames its parameter `_observed` and marks
`overlay_drift` `#[allow(dead_code)]`): NOT AGREED, 3/3 FAIL, never pushed.

- claude-sonnet-5-5: FAIL. `scoped_dry_run_plan` takes `_observed` and never
  uses it, so it plans on the unmodified locks; `overlay_drift` is dead code
  with no caller.
- claude-haiku-4-5: FAIL. The `_observed` parameter is never used to overlay
  drift before planning, and `overlay_drift` is dead code; the acceptance test
  would fail.
- gemini-3.8-flash-high (agy): FAIL. Named the control commit that stripped
  the `overlay_drift` call, and quoted the acceptance criterion that the test
  is red with that call removed.

Round at head 302123a9, the head being pushed: 3/3 PASS.

- claude-sonnet-5-5: PASS, no findings. The dry run overlays the gate's
  findings through `overlay_drift`, which shares `mark_drifted` with
  `record_machine_drift`, and still writes no lock. It read code only and ran
  no build or test; the build-host measurements are in `704-judges.md`.
- claude-haiku-4-5: PASS, no findings. The overlay is load-bearing, parameter
  threading is consistent across all call sites, and the unit, case-table and
  binary tests cover it.
- gemini-3.8-flash-high (agy): PASS, no defects.

The Claude round's third configured seat was left empty, so its own artifact
records NOT AGREED with lane 3 NO-VERDICT and no measured model; the agy lane
above, run on its own, is the third lane counted here. The advisory apr lane
did not run (busy) and is never counted.

Earlier local rounds ran on an unpushed branch (fix/plan-shows-dep-drift-repair)
with a different receipt; they are superseded by this round and judge nothing
here. Their source tree is identical to this branch's second commit, and two
of their non-blocking notes are R1 and R2 in `704-judges.md`.

FR-4 slot for this PR: GRANT 3/3, planted control DENY 3/3,
`degraded: same-family`.
