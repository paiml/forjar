# Lanes — v1.32.0 release cut (PMAT-592, PR #593)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED. Every other
file in this receipt quotes it rather than restating it. That rule exists because
the 1.31.0 cut restated its counts in four files and lanes caught them stale in
three, across two separate rounds.

## Composition, and why it is not three agy lanes

Two families, per the operator's ruling of 2026-09-20: **2 agy + 1 claude**.
agy is a separate quota bucket, so width is bought there rather than from the
Anthropic weekly limit; the single Claude lane is the cheap one and is the only
lane in the author's own family.

Lanes are READ-ONLY REVIEWERS OF THE RECORDED GATE RESULT. They never execute the
build, never touch a `target/` directory, and hold no token. The brief pastes the
claims, the recorded `make dogfood-release` JSON and the complete base..head diff
inline, so no lane needs repository access at all. This is deliberate: a lane
that cannot build cannot fill a disk, and a lane with no credential cannot
publish. Both of those have happened on this fleet before.

## The rounds

| round | head | lane 1 (agy) | lane 2 (agy) | lane 3 (claude) | findings |
|---|---|---|---|---|---|
| 1 | ac06c1be | gemini-3.1-pro-high FAIL, conv-eb4c6032 | gemini-3.1-pro-low FAIL, conv-335ed5ef | sonnet FAIL | 4 false sentences, 1 tree defect, 4 claim-wording refutations — every one adjudicated in the judges digest |

One round. It is one round because the round found real defects in all three
lanes and the repairs are recorded rather than argued with, not because agreement
was reached cheaply.

## Lane errors

One, and it is worth recording because it cost a dispatch: `--model
gemini-3.1-pro-low --effort high` is refused by agy with `invalid model
selection: --model gemini-3.1-pro-low conflicts with --effort=high`. The `-low`
models carry their effort in the model name and must be dispatched without
`--effort`. Re-dispatched without it and the lane returned a verdict.

## What each lane was right about

- **Both agy lanes and the claude lane, independently**: the CB-21xx ratchet note
  quoted 55 and 53 as though they were one measurement. Two lanes finding the
  same arithmetic defect without seeing each other is the strongest signal in
  this round.
- **agy gemini-3.1-pro-high**: the `—` escapes. No gate reads that file's
  formatting, so nothing else on this branch would have caught it.
- **agy gemini-3.1-pro-low**: the single reason given for holding back both #590
  and #591, refuted from the diff's own roadmap rows.
- **claude sonnet**: the cadence sentence, refuted against two dates in the same
  diff.

## What no lane was wrong about

Nothing was dismissed as a false finding in this round. The four claim-wording
refutations (C1, C4, C5, C6) are recorded as REFUTED rather than argued down,
because in each case the lane was right about what the claim said even where the
substance was sound.
