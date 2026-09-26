# Claims — correct how the 24h nightly gate failed (PMAT-631)

Counts and heads live in `PMAT-631-lanes.md`.

- **C1** the gate comment in `nightly.yml` states the 24h gate's real failure modes: a failed nightly was never retried on a quiet day, and a dropped, queued or late scheduled run left commits outside every window
- **C2** the falsifier's module doc states the same, and says outright that a commit landing after the day's 04:00 run is NOT missed
- **C3** the falsifier's commits are described as dated 2001-09-09, which is what `@1000000000` is
- **C4** the change is comments only: the gate script, the release step and every test body are byte-identical, and the falsifier still discriminates

## The measured symptom

forjar#630 (PMAT-629) replaced the 24h gate with a tag-vs-HEAD gate, and its
comment and test doc said the 24h gate "never rebuilt a commit that landed after
the day's run and was followed by a quiet day". That is false: such a commit is
under 24h old at the next day's 04:00 run and is built. The agy lane on the
identical pzsh port caught it; all of forjar#630's lanes had missed it. The test
doc also called commits dated 2001 "30 days old". forjar#631 records both.
