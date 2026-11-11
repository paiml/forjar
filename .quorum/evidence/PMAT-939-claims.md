# PMAT-939 (`forjar#945`) — Extracted Claim Manifest (`C1..C3`, `R1..R2`)

Base commit: `5b8f8123be40731340e2a74a6e4ae365648cab45`
Head commit: `a33086cbd1c9b04a01303445d1370b576fc27497`
Branch: `s-forjar-2/939`

1. **C1 (`[test]`)**: `lock_pins_no_libsqlite3_sys_below_0_39_0` (`tests/falsification_lockfile_libsqlite3_sys_not_in_0459_range.rs:45`) parses every `libsqlite3-sys` `[[package]]` block in `Cargo.lock` and asserts `version >= (0, 39, 0)` (`FJ945_REV1_KILLED`).
2. **C2 (`[test]`)**: `advisory_is_not_ignored_in_audit_or_deny_config` (`tests/falsification_lockfile_libsqlite3_sys_not_in_0459_range.rs:56`) reads `deny.toml` and `.cargo/audit.toml` fail-closed and forbids any non-comment mention of `RUSTSEC-2026-0459` (`FJ945_REV2_KILLED`).
3. **C3 (`[doc]`)**: `docs/roadmaps/roadmap.yaml` (`PMAT-939`) and `docs/audits/impl-PMAT-939-receipt.md` accurately credit `#942` for the `rusqlite 0.41` / `libsqlite3-sys 0.39.1` bump on `main`.
4. **R1 (`[test]`)**: `locked_libsqlite3_sys` (`tests/falsification_lockfile_libsqlite3_sys_not_in_0459_range.rs:30`) strips `-rc` pre-release suffixes (`0.39.0-rc.1` -> `(0, 39, 0)`) and omits an `!is_empty()` check.
5. **R2 (`[doc]`)**: `docs/audits/impl-PMAT-939-receipt.md` omitted a RED test run for `advisory_is_not_ignored_in_audit_or_deny_config`, and inline trailing `#` comments on non-comment lines would also trip `.contains("RUSTSEC-2026-0459")`.
