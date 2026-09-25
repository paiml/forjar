# Claims — the nightly rebuilds whenever its tag is not HEAD (PMAT-629)

Counts and heads live in `PMAT-629-lanes.md`.

- **C1** the gate builds exactly when `refs/tags/nightly^{commit}` is absent or is not HEAD, or on `workflow_dispatch`
- **C2** the release step tags the commit the run built, not the branch tip at API time
- **C3** the falsifier executes the workflow's own gate script, so a comment cannot satisfy it
- **C4** each declared mutation reddens its named tests, measured
- **C5** the check-activity checkout carries the tag the gate compares against

## The measured symptom

On 2026-09-24 forjar's published nightly was 12 days behind main and copia's 33,
while every scheduled nightly run in between was green with build and release
skipped: the gate asked "was there a commit in the last 24 hours?", so a commit
landing after the 04:00 run and followed by a quiet day was never built, and a
failed nightly was never retried. The fleet installer (infra PMAT-1055) reads
the nightly as the newest green build and was therefore installing stale bytes.
