# Quorum evidence — PMAT-215 — the claims put to the lanes

1. `drift` evaluates a `type: task` whose lock records it as not converged, rather than counting it in the skipped census.
2. The census still distinguishes inspected from not-inspected, and a resource that cannot be evaluated is never silently counted as clean.
3. The distinction is sound: a `completion_check` is an assertion needing no baseline, while the file, image and state-query paths compare against a recorded hash that a failed apply did not produce.
4. Removing the `ResourceLock` from `skip_reason` is safe: no `ResourceStatus` makes running a read-only check wrong.
5. The diff does nothing the ticket does not ask for.

Claims 1, 3 and 4 were confirmed. Claim 2 was refuted on the evidence as first written: the case pinning it used `--no-task-checks`, which is a declined measurement rather than an impossible one. Claim 5 was refuted: an out-of-scope file rode along in the first commit.
