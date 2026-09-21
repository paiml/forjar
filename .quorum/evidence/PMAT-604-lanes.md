# Lanes — the v1.32.0 booking (PMAT-604)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

2 agy + 1 claude, per the operator's ruling. The two agy lanes were read-only
over the recorded gate lines, the commit messages and the diff, all pasted inline,
and were run from an empty scratch directory so there was no tree to write into.
The Claude lane had the repository read-only, with no build and no credential,
and read-only `gh` calls so it could re-measure the GitHub facts the agy lanes
cannot reach.

| round | head | agy gemini-3.1-pro-high | agy gemini-3.1-pro-low | claude sonnet |
|---|---|---|---|---|
| 1 | a094fc7d | FAIL conv-37deb686 | FAIL conv-37d61113 | PASS |

## What the Claude lane re-measured instead of reading

The PR window from `git log v1.31.0..v1.32.0` and from a live merged-PR search
(the same seven PRs), the cut instant from the tag object's tagger timestamp,
next.due as cut plus two days, the cookbook master's head by `git ls-remote`, and
both cookbook files at a8e758ec. It also fetched sovereign-ci.yml from
paiml/.github and read the coverage job's own `if:` and the gate's
`coverage_owed` logic.

## What the fixes after this round were measured against

Gates A and E were re-run at b12a8392 over the full release window in a scratch
clone, 7 of 7 each. The strengthened test was run under eight mutations, and the
workflows were restored byte-identical to HEAD afterwards. The fix commit is
6b70299e. No second round: every finding was either a sentence (fixed and
re-measured) or a test gap (fixed and mutation-proven), and one finding was
refuted by a measurement.
