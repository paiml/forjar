# Quorum evidence — PMAT-235 — pmat and the gates

- The sync itself is `pmat work edit <id> -s inprogress` then `-s completed`, sixteen times. The tool refuses `planned -> completed` directly (`Invalid transition`, citing work-dbc-v1.yaml §work_lifecycle), and no status in this diff was written by editing YAML.
- `pmat work edit` drops any top-level field it does not know: fourteen rows lost `kind:`. All fourteen carry the matching `kind:*` label and `kind-gate.sh` reads the label first, so nothing the gates use was lost — checked, not assumed.
- `pmat work add` minted PMAT-235 and PMAT-236; `scripts/release-goal.sh tag` labelled both `release:v1.29.0` at mint time. Acceptance criteria and notes were patched textually, because `pmat work edit` exposes neither.
- `pmat work validate`: passes (one pre-existing warning, PMAT-200 has no acceptance criteria).
- `kind-gate.sh`: kind=triage files=0. `model-gate.sh`: model=opus class=opus decision=admit basis=file. `goal.sh set`: refused under R-5; `goal.sh worker` declared the lane. Routing: review `route=agy-quorum w=1.00 basis=absent effort=1[U]`.
- `scripts/dogfood/tagged.sh`: GATE T PASS on the branch — 6 tagged releases reconcile, 26 tickets carry their tag, 3 of 3 PRs merged since v1.28.0 carry release:v1.29.0.
- No test was run and none was reverted: this branch contains no code. That is `falsification.not_applicable`, printed by the gate as NOT VERIFIED.
- I-3 `transcript-gate.sh`: `PASS transcript-gate: attempted=20 denied=0 running_peak=1 slots=3 segments=679 files=20 (agent_calls=20 resumes=0 workflow_started=0; denied from hook log) (session <this session>, rule=explicit)`.
