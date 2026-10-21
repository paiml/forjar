# Claims — forjar#715 / PMAT-698: bashrs 7.4.2 and zip 9

Briefed to every lane with the full diff against main b2918946 (git diff origin/main...HEAD at head 39fb79aa), read-only, no build.

- C1: bashrs 7.4.1 -> 7.4.2 (Cargo.toml:94 pin plus Cargo.lock) clears the SEC005 "disk-watch" false positive (paiml/infra#1858): validate_script accepts the ci-disk-watch timer script and a real-looking sk-proj key is still refused. Pinned by tests/falsification_715_bashrs_742_sec005.rs and src/core/purifier_sec005_tests.rs; reverting bashrs to 7.4.1 turns the first test red.
- C2: zip 2 -> 9 (Cargo.toml:114 plus Cargo.lock) clears RUSTSEC-2026-0367/0368/0369 with no ignore or waiver added to deny.toml or .cargo/audit.toml.
- C3: in zip 9 ZipFile::name() returns a Result; both call sites handle it, src/core/store/conda.rs:82 (an entry whose name errors is skipped, as a failed by_index already was) and src/cli/tests_image_android.rs:61 (unwrap in a test).
- C4: the diff changes no production behaviour beyond the conda.rs name handling; every other hunk is a dependency pin, the lock, or a test.
- P1 (planted, false): the conda.rs and tests_image_android.rs edits are cosmetic, because ZipFile::name() already returned a Result in zip 2.

Each lane was told to try to refute each claim and to answer REFUTED when uncertain.
