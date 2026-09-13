# PMAT-555 — the independent review

`bash ~/.claude/skills/quorum-review/quorum-review.sh --base main --ticket
PMAT-555 --pr 556` with three `--lane-model` overrides. Each lane runs inside
agy in its own clone of the repository: push capability removed
(`pushInsteadOf` cleared and a refusing `pre-push` hook installed), a 32-hex
tree witness written into the clone's git-dir that a lane can only return by
reading its own workspace, and `writes=false`.

| lane | declared | measured (`agy` log) | source | family | verdict |
|---|---|---|---|---|---|
| 1 | `gemini-3.1-pro-high` | `gemini-3.1-pro-high` | measured | gemini | PASS |
| 2 | `gemini-3.6-flash-high` | `gemini-3.6-flash-high` | measured | gemini | PASS |
| 3 | `gemini-3.8-flash-medium` | `gemini-3.8-flash-medium` | measured | gemini | PASS |

Author: `claude-opus-5`, family `claude`. No lane is in the author's family,
which is the rule `receipt-lint.sh` enforces where receipts are linted.

The declared and measured columns are separate fields because agy's JSON
envelope carries no model at all: the measured value is read from the lane's own
`--log-file` line, and a lane with no measurement is refused rather than counted.

## What this review is not

Three lanes of one family are three samples of one model's judgement, and the
config's own comment records why: gpt-oss returned no verdict on review lanes,
so the default list is all gemini. A quorum drawn from one family agrees more
readily than one drawn from three, and this artifact records the family so the
reader can discount accordingly.
