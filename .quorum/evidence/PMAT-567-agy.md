# agy lanes — lint toolchain override (PMAT-567)

Round count and heads: see `PMAT-567-lanes.md`.

Two lanes, `--sandbox`, no `--add-dir`, not `--dangerously-skip-permissions`,
JSON-schema-constrained output so a lane cannot answer with narration in place of
a verdict. The brief pastes the claims and the whole diff inline; neither lane
had a repository, a shell, a network or a credential.

- `gemini-3.1-pro-high`, effort high — FAIL
- `gemini-3.1-pro-low` — FAIL

Both returned verdict objects.

## Why buying width outside the author's family paid here

The Claude lane returned PASS on all six claims. Both agy lanes returned FAIL,
and the defect they found — a version comparison correct only for the channel
spelling in use that day — was real and is now gone from the tree. A quorum drawn
entirely from the author's own family would have shipped it.

The Claude lane was not idle: it found the `channel = "stable"` counterexample
and the over-wide test slice. But it graded the claims PASS while doing so, and
the two agy lanes graded the same evidence FAIL. On this round the disagreement
was the signal.

## Dispatch note

`--model <name>-low` REFUSES `--effort`: the suffix IS the effort, and the lane
exits before doing any work. Dispatch the `-low` models with no `--effort` flag.
