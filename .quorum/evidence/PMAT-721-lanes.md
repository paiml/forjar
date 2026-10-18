# Lanes — forjar#722

Four counted rounds, each on a new head, each with three lanes from two families, none the author's model. Every lane was read-only and had the full diff. agy was sandboxed, with no build and no writes.

- Round 1, head 8a52e0e2: claude-sonnet-5, agy Gemini 4 Argon (High) and agy Gemini 3.8 Flash (High), all FAIL. Findings: the guard compared key and restore-keys counts wrongly; the guard was wired nowhere; GNU-only grep; a missing workflow passed vacuously.
- Round 2, head 1cb0e5ea: the same three lanes, all FAIL. Findings: a trailing comment could carry runner.arch, a sequence restore-keys escaped the check, and the existing fleet-arch test failed because X64 was gone from three jobs (D1).
- Round 3, head fcf9b307: claude-sonnet-5 PASS, Gemini 3.8 Flash PASS, Gemini 4 Argon FAIL. Argon refuted C7 on two points: an exempted job re-pinned to X64 was not reported stale, and matrix legs are flattened before the arch check. The first was fixed in 37955c80. The second predates this PR and is unchanged (see judges, R2).
- Round 4, head 37955c80: claude-sonnet-5 PASS, Gemini 4 Argon PASS, Gemini 3.8 Flash PASS. C1–C7 confirmed by all three. Non-blocking extras are recorded in the judges file.

Crux lane (competitive survey, read-only): Swatinem/rust-cache puts runner OS and arch as separate explicit key segments; the actions/cache examples key by OS only; sccache keys on rustc -vV (whose host line differs per arch) and the target. Adding runner.arch matches the purpose-built Rust tool.

Judge input given at head 484803a5, before the three later commits (all posted on the PR): three Opus routing votes (two APPROVE_WITH_CHANGE, one APPROVE; the changes were runner.arch in the cache keys and restoring the roadmap, both done) and two non-author Opus PASS checks. They are cited, not re-run, and are not counted as lanes here.
