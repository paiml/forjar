# PMAT-939 (`forjar#945`) — Judges Claims Digest

### CONFIRMED

1. [test] Lockfile floor >= 0.39.0 across all libsqlite3-sys entries (`C1`)
   - evidence: `tests/falsification_lockfile_libsqlite3_sys_not_in_0459_range.rs:45` inspects every `[[package]]` block named `libsqlite3-sys` in `Cargo.lock` and asserts `v >= (0, 39, 0)`; mutating `Cargo.lock` to `0.30.1` on `t2fw13` failed the compiled test binary (`FJ945_REV1_KILLED`).

2. [test] Anti-waiver enforcement across deny.toml and .cargo/audit.toml (`C2`)
   - evidence: `tests/falsification_lockfile_libsqlite3_sys_not_in_0459_range.rs:56` reads both `deny.toml` and `.cargo/audit.toml` fail-closed and rejects any non-comment line containing `RUSTSEC-2026-0459`; planting an ignore entry in `deny.toml` on `t2fw13` failed the test immediately (`FJ945_REV2_KILLED`).

3. [doc] Honest attribution of #942 bump and Gate A traceability (`C3`)
   - evidence: `docs/audits/impl-PMAT-939-receipt.md` (`41` lines) explicitly credits `#942` for landing `rusqlite 0.41` / `libsqlite3-sys 0.39.1` on `main`, scopes `#945` (`tests/falsification_lockfile_libsqlite3_sys_not_in_0459_range.rs:1`) to the regression guard, and aligns `PMAT-939` in `roadmap.yaml`.

### REFUTED

1. [test] Pre-release suffix ordering and empty-match behavior in locked_libsqlite3_sys (`R1`)
   - evidence: `tests/falsification_lockfile_libsqlite3_sys_not_in_0459_range.rs:30` strips non-digit suffixes (`0.39.0-rc.1` -> `(0, 39, 0)`), which compares `>= (0, 39, 0)` despite SemVer pre-release precedence, and `line 46` (`tests/falsification_lockfile_libsqlite3_sys_not_in_0459_range.rs:46`) omits `!is_empty()`.
   - corrected: Non-blocking because `Cargo.lock` pins stable `0.39.1` via `rusqlite 0.41.1` and omitting `!is_empty()` avoids a spurious failure if `rusqlite` is ever removed from the workspace.

2. [doc] Author-unexecuted RED perturbation on advisory-ignore check (`R2`)
   - evidence: `docs/audits/impl-PMAT-939-receipt.md` records under `Not covered` that no ignore entry was planted to RED-test `advisory_is_not_ignored_in_audit_or_deny_config` (`tests/falsification_lockfile_libsqlite3_sys_not_in_0459_range.rs:56`).
   - corrected: Executed on `t2fw13` during three-lane review by appending `ignore = ["RUSTSEC-2026-0459"]` to `deny.toml` and confirming `advisory_is_not_ignored_in_audit_or_deny_config` fails (`FJ945_REV2_KILLED`).
