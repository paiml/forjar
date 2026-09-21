# agy lanes — quorum.yml's private CARGO_HOME (PMAT-588)

Round count and heads: see `PMAT-588-lanes.md`.

Three lanes per round, four rounds: `gemini-3.1-pro-high`, `gemini-3.1-pro-low`
and `gemini-3.6-flash-low` (the `-low` suffixes are the effort; no `--effort`
passed with them). `--sandbox`, no repository write, output constrained by a JSON
schema, diff and ticket pasted inline.

## What the agy lanes found

- Round 1, pro-high: three wrong verdicts from two parsers of one rule — the
  file-level exoneration, the hosted-beside-self-hosted false positive, and a
  comment read as a declaration. The largest change the review produced.
- Round 2, pro-low: the negative control lived in a commit message instead of
  the test.
- Rounds 1 and 3, flash-low: two lane errors, each from reasoning about text the
  lane was not shown (an unchanged step outside the diff; fixtures counted by
  their numbered comments). Both refuted by running the command, not by argument.

Neither the highest tier nor the lowest was reliable alone: pro-high found the
design defect, flash-low produced both false alarms, and pro-low found the gap
between a criterion and its test.
