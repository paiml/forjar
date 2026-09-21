# agy lanes — the v1.32.0 booking (PMAT-604)

Round count and heads: see `PMAT-604-lanes.md`.

`gemini-3.1-pro-high` (with `--effort high`) and `gemini-3.1-pro-low` (the suffix
is the effort), `--sandbox`, JSON schema, a brief that forbade tool calls. Both
returned structured verdicts on the first run; no lane error this time.

Both agy lanes refuted C2's tree-identity sentence and C5's opt-in predicate. The
three test loopholes that were fixed came from them: the negated tag pattern
(high), the unchecked `uses` (both) and the job-level `if:` (low). Both also said
the tag trigger widens the whole workflow, and the receipt now says so.

agy-high's one finding that did not survive: that the fourth label move landed on
PMAT-591 rather than PMAT-594. It inferred row ownership from a unified diff's
hunk context, where the lines shown after a change can belong to the next row.
The file itself answers it. agy-low assessed C2 to C5 and did not return an
assessment for C1 or C6, so it is not counted as judging those two.
