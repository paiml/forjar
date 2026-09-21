# Lanes — a file check asks for the declared content and mode (PMAT-600)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

Three agy lanes per round, three distinct models, read-only, diff and ticket
pasted inline, JSON-schema-constrained verdicts. No Claude lane: the author is
Claude and the gate counts distinct lanes.

| round | head | agy gemini-3.1-pro-high | agy gemini-3.1-pro-low | agy gemini-3.6-flash-low |
|---|---|---|---|---|
| 1 | c2e4ee24 | NO-VERDICT (429 quota) | NO-VERDICT (429 quota) | NO-VERDICT (429 quota) |
| 2 | c2e4ee24 | NO-VERDICT (429 quota) | NO-VERDICT (429 quota) | NO-VERDICT (429 quota) |
| 3 | c2e4ee24 | PASS | PASS | PASS |

Rounds 1 and 2 are not verdicts: every lane returned `RESOURCE_EXHAUSTED (code
429): Individual quota reached` before reading the diff. They are recorded so the
count is honest; round 3 ran after the reset on the same head.

## What moved the diff before the quorum

The review did not move the diff; the tests did. The first form of the content
assertion, `[ "$( { sha256sum p || shasum -a 256 p; } | cut …)" = 'hex' ]`, was
refused by forjar's own I8 gate: bashrs SC2107 read the `||` inside the
substitution as `[ a || b ]` at Error, and 20+ lib tests (`cli::tests_check_*`,
`cli::apply_quality_gate`) failed. The assertions now read into a variable and
then test. The bashrs defect is paiml/bashrs#366, fixed in bashrs#367.
