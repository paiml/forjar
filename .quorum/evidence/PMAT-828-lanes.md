# Lanes — PMAT-828

Every lane read the diff pasted inline, with no repo and no shell (agy --sandbox; the Claude lane via claude -p, no tools). Author model: Claude Sonnet 5.5. Per the operator's fallback rule (gpt-oss and the Claude models on agy are quota-exhausted, 429, resets in about 46 h), the third lane is Claude Opus 5.5, never the author's model id. The Gemini 4 Argon lanes are a non-Claude family, so this quorum is not degraded: same-family.

| round | head | Gemini 4 Argon High | Gemini 4 Argon Medium | Claude Opus 5.5 |
|---|---|---|---|---|
| 1 | 8030f564 | PASS | PASS | FAIL |
| 2 | fbc084c8 | PASS | PASS | PASS |

Round 1 Opus FAIL asked for: the false "every assertion is on the files" doc line fixed, the misplaced comment above `mod data_isolation`, and proof that apply does not forget state across an outage. All three were done before round 2 (the hyphenated-key boundary came from both Argon lanes). After round 2 three nits were applied without a round 3: comment placement, the test-doc wording, and the lock assertion parsing the key.
Known and not fixed here: a data reference reached only through `params:` or `machines:` is not detected as a dependent (follow-up).
