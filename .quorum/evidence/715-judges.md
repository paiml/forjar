# Judges — forjar#715 / PMAT-698 at 39fb79aa

Three judges per claim: the three lanes of the round (gemini-4-argon-high, gemini-3.8-flash-high, claude-opus-5-5). Each item was also checked by the author's own runs on the build host: the two falsification tests green with the fix and red with bashrs pinned to 7.4.1, `cargo test --lib -- purifier conda image_android` 80 passed, and `cargo deny check advisories` ok.

## CONFIRMED

1. [bashrs] C1 — bashrs 7.4.2 clears the SEC005 disk-watch false positive and a real-looking key is still refused, so the pin is what makes the apply gate accept the ci-disk-watch timer script.
   - evidence: Cargo.toml:94 pins `bashrs = "7.4.2"` and Cargo.lock:270 is 7.4.2. All three lanes read bashrs 7.4.2 `sec005.rs`, where `is_token_start` only accepts `sk-` at a token start, while 7.4.1 used a plain substring find that matched `di`+`sk-watch`. `tests/falsification_715_bashrs_742_sec005.rs:11` and `:22` pin both halves. Measured here: green with 7.4.2 (2 passed); with Cargo.toml back to "7.4" and `cargo update -p bashrs --precise 7.4.1` the disk-watch test failed with "false SEC005: bashrs lint errors" and the key test stayed green (1 passed, 1 failed).
2. [zip] C2 — zip 2 -> 9 clears RUSTSEC-2026-0367, -0368 and -0369 and adds no waiver.
   - evidence: Cargo.toml:114 is `zip = { version = "9", ... }` and Cargo.lock:5001 resolves 9.0.5. All three advisories in the advisory db say patched `>= 9.0.0`. deny.toml and .cargo/audit.toml are untouched by the diff. Measured here: `cargo deny check advisories` prints "advisories ok" on this head.
3. [api] C3 — zip 9 changed ZipFile::name() to return a Result and both call sites in the tree handle it.
   - evidence: zip 9.0.5 `src/read/zipfile.rs:83` declares `pub fn name(&self) -> ZipResult<Cow<'_, str>>`. `src/core/store/conda.rs:82` now chains `by_index(i).ok().and_then(|f| f.name().ok()...)`, so a name error drops the entry exactly as a by_index error already did, and `src/cli/tests_image_android.rs:61` unwraps in a test. The opus lane grepped the tree and found no other caller of name() on a zip entry.
4. [scope] C4 — apart from the conda.rs name handling, the diff is dependency pins, the lock and tests.
   - evidence: the non-lock hunks are Cargo.toml:94 and :114, `src/core/store/conda.rs:82`, a test unwrap at `src/cli/tests_image_android.rs:61`, `#[cfg(test)] mod purifier_sec005_tests` at `src/core/mod.rs:36`, a `#[cfg(test)] mod sc1066_tests` in `src/core/purifier.rs:258`, and two test files. One side note from the opus lane, accepted: the zip bump also changes the library the production writer in `src/cli/image_android.rs` calls, which is the dependency pin itself and is covered by the unit run (80 passed).

## REFUTED

1. [cosmetic] P1 — The planted claim that the conda.rs and tests_image_android.rs edits are cosmetic, because ZipFile::name() already returned a Result in zip 2.
   - corrected: all three lanes refuted it. zip 2.4.2 `src/read.rs:1543` declares `pub fn name(&self) -> &str`, and a Result cannot be turned into a String by `to_string()` the old way, so `src/core/store/conda.rs:82` and `src/cli/tests_image_android.rs:61` are required to compile against zip 9. The conda.rs edit also adds a new skip path for entries whose names fail to decode.
