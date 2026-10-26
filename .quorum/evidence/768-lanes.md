# Quorum evidence — #768 / PR 771 — lane summaries

Author models on the commits: Claude Sonnet 5.5 and Claude Opus 5.5 (both Co-Authored-By trailers). No lane used either.
agy quota: the Claude-family buckets (opus 5.5) and gpt-oss returned a quota error at launch, so the agy lanes ran the two
Gemini models that were available and the Claude lane ran claude-haiku-4-5.

## claim lanes (scoped to the PR's own delta plus the guard, at least five delta claims each)
- lane 1, agy gemini-4-argon-high, sandboxed: SOUND-WITH-CORRECTIONS. Counted the 19 sites, confirmed release jobs keep X64, found the matrix-flattening hole in `runner_labels`.
- lane 2, agy gemini-3.8-flash-high, sandboxed: SOUND-WITH-CORRECTIONS. Raised the matrix-flattening hole and the staleness-ignores-self-hosted hole; both survived refutation and are recorded as REFUTED claims C4 and C5.
- lane 3, claude-haiku-4-5, read-only tools: SOUND. A first, unscoped run over the whole 49-file diff rubber-stamped everything and was discarded; the scoped rerun is the one counted.

## refuters (three per claim, same three models, adversarial prompt: kill the claim or say what you read)
12 claims, 36 verdicts. The planted control C9 was refuted by 3 of 3. Two real defects were refuted into the open: C4 (matrix legs) 3 of 3, C5 (staleness) 3 of 3. C10 (gx10 pickup is not guaranteed by the diff) 3 of 3. Disagreement existed on C8 only at judging time.

## falsification probe (measured on a build host)
Green on the PR head: `cargo test --test falsification_every_ci_job_runs_on_the_fleet` 13 passed, 0 failed.
Red with the test file reverted to the epic base's version: 12 passed, 1 failed (`a_fleet_job_names_the_architecture_it_needs`: "these fleet jobs name no architecture, and `clean-room` spans X64 and ARM64", naming audit, behavior, classify, doctests and others). Restored: 13 passed.
