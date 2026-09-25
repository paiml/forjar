# Lanes — correct how the 24h nightly gate failed (PMAT-631)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

Read-only lanes: the brief pastes the claim and the full diff; each lane may run
read-only `git show`/`git diff`/`git log`/`grep`/`sed` in a checkout, and
nothing else. No lane builds. Standing shape: sonnet + one agy
gemini-3.1-pro-high + haiku. The author is claude-opus-5-5 and no lane ran that id.

| round | head | lane 1 | lane 2 | lane 3 |
|---|---|---|---|---|
| 1 | 3e47bc2c | claude-sonnet-5 PASS conv-f44011a2 | agy gemini-3.1-pro-high PASS conv-5e7403c6 | claude-haiku-4-5 PASS conv-7289d05c |

One round, 3/3 PASS, no findings. These three verdicts are this receipt's
`quorum.lanes`.
