# agy teamwork — forjar#683

One sandboxed agy lane: gemini-3.1-pro-high, brief inline, no build, no
writes, 14m budget. Verdict PASS. It confirmed RUSTUP_HOME is placed under
the work root in the cross jobs and that the falsifier discovers every job
that runs cross and resolves the paths it mounts.

Rounds 2 and 3: the same sandboxed lane, PASS both times. Round 4: FAIL on
the stale receipt (3 tests, claims C1–C4) and on scope, answered in
`PMAT-683-lanes.md` and refuted as R3 in `PMAT-683-judges.md`.
