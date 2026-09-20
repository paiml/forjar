# agy lanes — v1.32.0 release cut (PMAT-592, PR #593)

Round count and heads: see `release-1.32.0-lanes.md`. Numbers here quote it.

## How the lanes were run

Two agy lanes, dispatched detached with `setsid nohup`, `--sandbox`, no
`--add-dir`, and NOT `--dangerously-skip-permissions`. Output constrained by a
JSON schema so a lane cannot answer with narration in place of a verdict.

The brief pastes everything a lane may rely on: the eight claims, the recorded
`make dogfood-release` result as JSON, and the complete base..head diff. No lane
was given a repository path, a clone, a shell or a credential. This is the LANE-1
shape the operator ruled for, and it is what makes the historical hazards
inapplicable rather than merely unlikely — a lane that never builds cannot copy a
target directory onto a full disk, and a lane with no token cannot publish.

## The lanes

- `gemini-3.1-pro-high`, effort high, conv-eb4c6032 — FAIL
- `gemini-3.1-pro-low`, conv-335ed5ef — FAIL

Both returned a verdict object. Neither NO-VERDICTed, which is not guaranteed on
this backend: the 1.31.0 cut recorded seven lane errors across its rounds, most
of them a SUCCESS envelope carrying no verdict, and those are counted as no
review rather than as a pass.

## Dispatch error worth recording

`--model gemini-3.1-pro-low --effort high` is refused outright:

    invalid model selection: --model gemini-3.1-pro-low conflicts with --effort=high

The `-low` and `-high` suffixes ARE the effort. Passing `--effort` alongside one
of them is a conflict rather than a redundancy, and the lane exits before doing
any work. Re-dispatched with no `--effort` flag and it returned normally.

## What the agy lanes found that the Claude lane did not

- **`gemini-3.1-pro-high`**: the twenty unicode escapes converted to literal
  em-dashes by a JSON rewrite. No gate in this repository reads that file's
  formatting, so nothing else on the branch would have caught it. This is the
  clearest case in the round for buying width outside the author's own family.
- **`gemini-3.1-pro-low`**: the single reason given for holding back two
  different tickets, refuted from roadmap rows inside the diff it was reviewing.

## Verdict

Both lanes FAIL, and both were right to. Every finding is adjudicated in
`release-1.32.0-judges.md`; none was dismissed, and the four claim-wording
refutations are recorded as refutations rather than argued down.
