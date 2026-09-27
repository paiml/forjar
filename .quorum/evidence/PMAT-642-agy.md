# agy lanes — a mount's ownership options are checked state (PMAT-642)

Round count and heads: see `PMAT-642-lanes.md`.

Three lanes per round in the configured canary shape: gemini-3.1-pro-high, a
second gemini-3.1-pro-high (the configured claude-opus-4-6-thinking was skipped
as author-family) and claude-haiku-4-5. Each ran in its own conversation with
JSON-schema output and the brief pasted inline, with no build. The lanes read
the workspace and wrote nothing. The advisory apr lane was busy and was not
counted.

Round 1 at 6c1cabf2: 1/3. Both gemini lanes refuted C2 on the omitted-key
wildcard (R1). Corrected in 0b5882e0.

Round 2 at 0b5882e0: 3/3 PASS, with no defect findings. Lane 1 confirmed C1–C4
with cited lines in mount.rs and tests_mount_options.rs.
