# Quorum evidence — PMAT-237 — pmat and the gates

- `kind-gate.sh`: kind=code files=8. `model-gate.sh`: model=opus class=opus decision=admit basis=file. `goal.sh set`: refused under R-5; `goal.sh worker` declared the round. Routing: impl `route=agy-goal w=1.00 basis=absent note=fable-binding effort=1[U]` — executed by self, the deviation named: workers and lanes may not edit `.github/workflows`, and this change is almost entirely workflows.
- Gates: `cargo test --test falsification_pr_lane_runs_what_the_change_can_break` 13 passed; the registered mutation (the unclassified arm made harmless) turns three cases red; two wiring mutations each turn their rule red (docs/audits/logs/PMAT-237-class-census.log). `actionlint` finding set identical to main's (22 both sides). All four edited workflows parse as YAML. `bashrs lint scripts/ci/changed-class.sh`: 0 errors.
- **The 95% floor, measured on this branch rather than asserted**: `cargo llvm-cov --workspace --locked --summary-only --fail-under-lines 95` exit 0 at 95.69% lines, 92.80% regions. Coverage is deliberately NOT among the gated jobs.
- `pmat analyze vacuous-tests --path tests --format json`: 0 in the touched test file.
- Gate F's mutation arm: nothing under src/ changes, so it measures zero (PMAT-216 unchanged).
- The classifier is a script rather than a YAML `if:` for one reason: a test can drive it. Six cases do, in both directions; three pin the wiring; one re-derives the exclusion list from the tree so the allow-list cannot rot; and three more pin what the review found.
- I-3 `transcript-gate.sh`: `PASS transcript-gate: attempted=25 denied=0 running_peak=1 slots=3 segments=852 files=25 (agent_calls=25 resumes=0 workflow_started=0; denied from hook log) (session <this session>, rule=explicit)`.
