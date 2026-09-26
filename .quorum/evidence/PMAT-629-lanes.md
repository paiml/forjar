# Lanes — the nightly rebuilds whenever its tag is not HEAD (PMAT-629)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

Read-only lanes: the brief pastes the claim, the measured symptom and the full
diff; each lane may run read-only `git show`/`git diff`/`git log`/`grep`/`sed`
in a checkout, and nothing else. No lane builds. Rounds 1 and 2 were Claude-only
(`degraded: same-family`) because agy was logged out on the reviewing host;
round 3 restored the standing shape, sonnet + one agy gemini-3.1-pro-high +
haiku. The author is claude-opus-5-5 and no lane ran that id.

| round | head | lane 1 | lane 2 | lane 3 |
|---|---|---|---|---|
| 1 | 6ca59c8b | claude-sonnet-5 PASS conv-f80908e9 | claude-haiku-4-5 PASS conv-d1bf0b23 (low: stale header) | claude-sonnet-5 PASS conv-133abf42 |
| 2 | 33c8f41c | claude-sonnet-5 PASS conv-50cefe9f | claude-haiku-4-5 PASS conv-30983bc4 | claude-sonnet-5 PASS conv-08afb6c0 |
| 3 | 0daebb43 | claude-sonnet-5 PASS conv-a876498a | agy gemini-3.1-pro-high PASS conv-9661d5ed | claude-haiku-4-5 PASS conv-0a357202 (medium, refuted as R3) |

The same change was reviewed in paiml/copia#69 in parallel. Its round-1 lane 3
(claude-sonnet-5, conv-2c9763ea) FAILED on the missing `target_commitish`,
a defect this workflow shared byte for byte; it is adjudicated here as R1 and
fixed in round 2 on both repositories.

Round 3 is the first round to see the falsification test, and its three
verdicts are the ones this receipt's `quorum.lanes` counts.
