# Quorum evidence — PMAT-231 — pmat and the gates

- Roadmap rows PMAT-231 (kind: triage), PMAT-232, PMAT-233 and PMAT-234 (kind: code) minted with `pmat work add`, patched textually, each labelled `release:v1.29.0` at mint time with `scripts/release-goal.sh tag` — the open window's tag, which is the continuous half of PMAT-225's brief.
- `kind-gate.sh`: kind=triage files=0. `model-gate.sh`: `model=opus class=opus decision=admit basis=file` (a kind: triage row admits opus only, which is why this ticket could not have run under the session's earlier model). `goal.sh set`: refused under R-5; `goal.sh worker` declared both rounds.
- Routing (`route.sh`, verbatim): review `route=agy-quorum w=1.00 basis=absent effort=1[U]` (followed, two rounds); orchestration `route=self w=100.00 basis=absent`.
- Gates: `scripts/dogfood/release-check.sh` exit 0 on the published release; `make dogfood-published VERSION=1.28.0` exit 0 with GATE C and GATE D PASS against the crate crates.io serves; `scripts/dogfood/tagged.sh` GATE T PASS on the branch — 6 tagged releases reconcile, 26 tickets carry their tag, 2 of 2 PRs merged since v1.28.0 carry `release:v1.29.0`. Both runs are in docs/audits/logs/PMAT-231-release-check.log.
- No test was run and none was reverted: this branch contains no code. That is `falsification.not_applicable` in the artifact and the gate prints it as NOT VERIFIED.
- `pmat analyze vacuous-tests`: no test file is touched; the tree-wide figure (43 of 4713) is recorded in the PMAT-230 artifact.
- Line stops in docs/audits/jidoka.jsonl: the row patcher that silently left two `notes: null` fields, and the receipt that paraphrased a gate.
- I-3 `transcript-gate.sh`: `PASS transcript-gate: attempted=19 denied=0 running_peak=1 slots=3 segments=645 files=19 (agent_calls=19 resumes=0 workflow_started=0; denied from hook log) (session <this session>, rule=explicit)`.
