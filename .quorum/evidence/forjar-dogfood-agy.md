# Independent review — agy /teamwork-preview — PMAT-163

One /teamwork-preview lane (conv-e44667f5, 303 s, sandboxed, no builds) hunted for theater in the gate scripts without seeing the claim lanes.

## Verdict (FAIL)

The branch introduces several unsound gating mechanisms that can silently mask failures or pass over empty sets. Notably, release-check.sh ignores squash-merged PRs and masks remote failures if tags aren't fetched locally, and coverage.sh fails to ratchet down ignored test counts, allowing new failures to be hidden in the slack.

## Findings, as returned

- T-F1 [asserted] scripts/dogfood/docs.sh:197 — Gate theater: docs.sh reads the surface list from the committed CSV rather than the built artifact, allowing dead verbs to pass the presence check if the CSV is stale.
- T-F2 [asserted] scripts/dogfood/release-check.sh:140 — Exit 0 on failure: release-check.sh uses git log --merges to find PRs. If PRs are squash-merged, the pipeline returns empty but exits 0, bypassing the loop and silently passing the quorum check.
- T-F3 [asserted] scripts/dogfood/surface.sh:41 — Vacuity floors: The floors are hardcoded in the bash scripts (e.g., MIN_CLI=211). Shrinking the ledger does not lower the floor; a developer must manually edit the script to lower the hardcoded value.
- T-F4 [asserted] scripts/dogfood/comply.sh:144 — Mutation comments: They are genuine one-line changes that turn their respective gates red, proving the gate is falsifiable.
- T-F5 [asserted] scripts/dogfood/docs.sh:250 — docs.sh executes blocks, so dead verbs fail execution. However, a dead verb listed in KNOWN_BROKEN (like forjar make clean) bypasses the exit code check and silently passes.
- T-F6 [asserted] scripts/dogfood/release-check.sh:158 — release-check.sh exits 0 (PASS) with PENDING if the tag is missing locally. If the release failed on GitHub but the tag isn't fetched locally, this script masks the failure by passing.
- T-F7 [asserted] scripts/dogfood/coverage.sh:98 — #452 gating: coverage.sh checks if ignored > IGNORED_CEILING, but lacks a check to enforce shrinkage if ignored < IGNORED_CEILING. Slack can be used to silently ignore new failures.

## What became of it

Its four mechanisms were each answered: the squash-blind PR enumeration and the local-tag PENDING were real and are fixed (f7d97024, 3c740863, PMAT-178 and PMAT-180); the one-directional ratchet was real and is now exact (PMAT-179); the ledger read by docs.sh is verified against the built binary earlier in the same run (D5). It also affirmed that the mutation comments are genuine one-line reds and that the floors are hardcoded, so shrinking the committed ledger does not lower them.
