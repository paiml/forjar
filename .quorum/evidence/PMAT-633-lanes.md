# Lanes — the I8 gate judges with bashrs 7.4 (PMAT-633)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

Read-only lanes: the brief pastes the claims and the diff (Cargo.lock as
name/version changes); each lane may run read-only `git show`/`git diff`/
`git log`/`grep`/`sed` in a checkout, and nothing else. No lane builds.
Standing shape: sonnet + one agy gemini-3.1-pro-high + haiku. The author is
claude-opus-5-5 and no lane ran that id.

| round | head | lane 1 | lane 2 | lane 3 |
|---|---|---|---|---|
| 1 | 08c5d340 | claude-sonnet-5 FAIL (C4 unevidenced) | agy not run (invocation error, no --repo-root) | claude-haiku-4-5 PASS |
| 2 | 08c5d340 | claude-sonnet-5 PASS conv-af3e6c79 | agy gemini-3.1-pro-high PASS conv-30f7dec5 | claude-haiku-4-5 PASS conv-321ac729 |

Round 2 brief = round 1 plus the C4 measurements and the paiml/infra#1101 diff.
Round 2 is 3/3 PASS; those three verdicts are this receipt's `quorum.lanes`.
