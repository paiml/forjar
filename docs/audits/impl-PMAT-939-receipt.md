# Implementation receipt — PMAT-939 — `ci / security` red on RUSTSEC-2026-0459 (libsqlite3-sys via rusqlite)

verdict: PASS — the fix itself (rusqlite 0.41, libsqlite3-sys 0.39.1) reached main through #942; this PR adds the falsifier that keeps `Cargo.lock` out of the advisory's range and keeps the advisory out of the ignore lists

## Identity

- ticket PMAT-939 (forjar#939); kind: test (no dependency or production change in this PR)
- PR #945, branch `s-forjar-2/939`, head before this receipt d7185e99
- files: `tests/falsification_lockfile_libsqlite3_sys_not_in_0459_range.rs` (+68), `docs/roadmaps/roadmap.yaml` (row `GH-939` renamed to `PMAT-939`)

## Root cause (five whys)

1. `ci / security` was red on every PR.
2. `Cargo.lock` pinned libsqlite3-sys 0.30.1, inside RUSTSEC-2026-0459.
3. It came in through rusqlite 0.32, which nothing had bumped.
4. The only check was the audit job itself, which says "red" but not which pin, and can be silenced by an ignore entry.
5. No test in the repo stated the lower bound, so a later downgrade or an ignore would pass the suite.

## Contract and falsifier

Contract: `Cargo.lock` pins no `libsqlite3-sys` below 0.39.0, and RUSTSEC-2026-0459 is not listed as ignored in the audit or deny configuration.

Falsifiers (`tests/falsification_lockfile_libsqlite3_sys_not_in_0459_range.rs`): `lock_pins_no_libsqlite3_sys_below_0_39_0`, `advisory_is_not_ignored_in_audit_or_deny_config`.

## Evidence (kfocus, `nice -n 19`, 2026-11-11)

GREEN at d7185e99 (`cargo test --test falsification_lockfile_libsqlite3_sys_not_in_0459_range`):

    test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

RED, the same test binary with the `libsqlite3-sys` version in `Cargo.lock` set to 0.30.1 (text edit, restored afterwards):

    test lock_pins_no_libsqlite3_sys_below_0_39_0 ... FAILED
    test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out

## Not covered

- No RED was run for `advisory_is_not_ignored_in_audit_or_deny_config` (an ignore entry was not planted).
- The dependency bump is #942's, not this PR's; this receipt does not re-measure it.
- The three review lanes ran that missing RED themselves (an ignore entry planted in `deny.toml` failed the test); it is their measurement, not the author's.
- `locked_libsqlite3_sys` strips pre-release suffixes (`0.39.0-rc.1` reads as 0.39.0), indexes three version parts without a length check, and the first test does not assert that at least one `libsqlite3-sys` entry was found, so a lockfile with none passes it.

- Three-lane review of PR #945 at a33086c (agy, Gemini 4 Carbon): 3/3 PASS; the refuted claims are listed under `quorum.refuted_claims` in the receipt. The lanes' findings were added under Not covered afterwards. Quorum receipt: `.quorum/s-forjar-2-939.json`, written by agy and committed unedited.

IMPL-PMAT-939-RECEIPT-END
