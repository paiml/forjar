# Quorum evidence — PMAT-232 — pmat and the gates

- `kind-gate.sh`: kind=code files=3. `model-gate.sh`: model=opus class=opus decision=admit basis=file. `goal.sh set`: refused under R-5; `goal.sh worker` declared the lane. Routing: impl `route=agy-goal w=1.00 basis=absent note=fable-binding effort=1[U]` — executed by self, the deviation named, because workers and lanes may not edit `.github/workflows`; review `route=agy-quorum w=1.00 basis=absent effort=1[U]`.
- `pmat work edit PMAT-233 -s cancelled` is the triage half: a duplicate closed with the tool, not by editing YAML. `pmat work validate` passes.
- Gates: `cargo test --test falsification_release_workflow_shape` 8 passed at HEAD, 7 passed / 1 failed against main's release.yml (docs/audits/logs/PMAT-232-gate-tests.log); three mutations each red (docs/audits/logs/PMAT-232-rule5-mutations.log); both workflows parse as YAML; actionlint's finding set identical to main's (22 both sides).
- Not measured: the job itself. This change cannot be executed outside a release, and it will first run for real at the v1.29.0 cut, due 2026-09-12T16:07:14Z. What stands in for that is rule 5, three mutations, and three lanes reasoning about every state the shell can be in.
- `pmat analyze vacuous-tests --path tests --format json`: 43 of 4715 tests examined flagged tree-wide; 0 in the touched test file.
- Gate F's mutation arm: nothing under src/ changes, so it measures zero (PMAT-216 unchanged).
- I-3 `transcript-gate.sh`: `PASS transcript-gate: attempted=21 denied=0 running_peak=1 slots=3 segments=718 files=21 (agent_calls=21 resumes=0 workflow_started=0; denied from hook log) (session <this session>, rule=explicit)`.
