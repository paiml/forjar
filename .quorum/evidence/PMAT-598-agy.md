# agy lanes — tool-requiring jobs on bare metal (PMAT-598)

Round count and heads: see `PMAT-598-lanes.md`.

Two lanes per round, three rounds: `gemini-3.1-pro-high` (effort high) and
`gemini-3.1-pro-low` (no `--effort`: the suffix is the effort). `--sandbox`, no
`--add-dir`, never `--dangerously-skip-permissions`, output constrained by a JSON
schema. The brief pastes the claims, the live runner-label table, the tool
inventory and the diff — and from round 3, the commit messages, after round 2's
agy-high refuted a claim about a commit message it had not been shown.

## What the agy lanes found

- Round 1, both: the test declared one mutation while the commit claimed four.
- Round 2, agy-low: the substring detector missed `--locked --lib`, found
  independently of the Claude lane; and mutation 4 reddens two tests, not one.
- Round 2, agy-high: a name filter after `--` was misclassified.
- Round 3, agy-high: a stale function count in the commit message.
- Round 3, both: PASS on the detector as it then stood.

## Why the round-3 agy PASSes are not the end of it

Both agy lanes passed round 3 while the Claude lane failed it with a real false
negative, and that same Claude lane then reported a `&&` chain as correct when it
was not. Neither family was reliable alone in this round. The disagreement was
the signal, and the measurement — an asserted case — settled both.
