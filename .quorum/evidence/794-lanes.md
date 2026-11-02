# Quorum evidence — #794 — lane summaries

Author model on the commits: Claude Sonnet 5.5 (Co-Authored-By trailer). No lane used it. All lanes and judges ran claude-opus-5-5 through Claude Code with read-only tools; no writes.

- lane 1, claude-opus-5-5, code: PASS. C1 and C2 confirmed, C3 refuted. Checksum matches the cached crate.
- lane 2, claude-opus-5-5, manifest side: PASS. Cargo.toml:105 admits 1.53.3; no AsyncFd use in src or tests, so the new deprecations raise no warning under -D warnings.
- lane 3, claude-opus-5-5, adversary and waiver check: PASS. deny.toml has no ignore or unsound setting for 0386; checksum matches the cached crate.

falsification probe: tests/falsification_tokio_rustsec_2026_0386.rs is green on the fix (1 passed) and red with Cargo.lock restored to origin/main ("Cargo.lock pins tokio (1, 53, 1), unpatched", 1 failed).
