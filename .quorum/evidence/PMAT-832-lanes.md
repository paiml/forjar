# Lanes — PMAT-832

Every lane read the diff pasted inline, with no repo and no shell (agy --sandbox; the Claude lane via claude -p --tools empty). Author model: Claude Sonnet 5.5. agy Claude and gpt-oss lanes are quota-exhausted (429), so the third lane is Claude Opus 5.5, never the author's model id; the two Gemini 4 Argon lanes are a non-Claude family, so this quorum is not degraded: same-family.

| round | head | Gemini 4 Argon High | Gemini 4 Argon Medium | Claude Opus 5.5 |
|---|---|---|---|---|
| 1 | ab68c5f1 | PASS | PASS | PASS |

Known and not fixed here: see the REFUTED section of the judges digest.
