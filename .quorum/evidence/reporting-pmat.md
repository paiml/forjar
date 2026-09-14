# Quorum evidence — PMAT-228 + PMAT-229 — pmat and the gates

- `kind-gate.sh`: kind=code files=2 (PMAT-228). `model-gate.sh`: model=opus class=opus decision=admit basis=file. `goal.sh set`: refused under R-5; `goal.sh worker` declared the one shared round. Routing: impl `route=agy-goal w=1.00 basis=absent note=fable-binding effort=1[U]` (self), review `route=agy-quorum w=1.00 basis=absent effort=1[U]` (delegate, three lanes).
- `pmat work edit` moved both tickets planned -> inprogress -> completed, the lifecycle the tool enforces; PMAT-235 established that a ticket's status is completed when its work merges.
- Gates: `cargo test --no-fail-fast` over the three suites that drive these scripts — 19 + 2 + 14 green at HEAD, and both new cases red against main's scripts (docs/audits/logs/PMAT-228-gate-tests.log). `bashrs lint` 0 errors on both changed scripts. `scripts/dogfood/tagged.sh` byte-identical to main's, verified with `git diff main --`. Gate T green on the branch.
- `--no-fail-fast` is in the log for a reason: an earlier run stopped at the first failing binary and left one red case unmeasured, which would have been a receipt claiming a proof it did not have.
- `pmat analyze vacuous-tests --path tests --format json`: 43 of 4717 tests examined flagged tree-wide; 0 in either touched test file.
- Gate F's mutation arm: nothing under src/ changes, so it measures zero (PMAT-216 unchanged).
- I-3 `transcript-gate.sh`: `PASS transcript-gate: attempted=23 denied=0 running_peak=1 slots=3 segments=786 files=23 (agent_calls=23 resumes=0 workflow_started=0; denied from hook log) (session <this session>, rule=explicit)`.
