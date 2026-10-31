# Quorum evidence — #835 — lane summaries

degraded: same-family. Authors are Claude (Sonnet 5.5 and Opus 5.5); all lanes and judges ran claude-haiku-4-5 with no tools, diff and claims inline, no writes. Every reply ended with the instruction to quote any sentence a reader could check and find false.

- lane 1, claude-haiku-4-5: PASS. C1 to C4 confirmed, C5 refuted. No defect; deny.toml and audit.toml carry identical ids and reasons.
- lane 2, claude-haiku-4-5: PASS. C1 to C4 confirmed, C5 refuted as test code under tests/ only.
- lane 3, claude-haiku-4-5: PASS. C1 to C4 confirmed, C5 refuted. Its one checkable sentence (audit.toml has 23 plus the legacy -0097, so 24 lines) was checked and is true.

falsification probe: tests/falsification_840_advisory_ignores_are_bounded.rs is green on the branch (8 passed). With the RUSTSEC-2026-0405 line deleted from .cargo/audit.toml, two tests fail (deny_and_audit_agree_with_identical_reasons and every_ignore_line_is_counted_in_both_files); the file was restored and the tree is clean.
