# Implementation receipt — PMAT-893 — encryption is a default feature; `secrets encrypt` never takes the secret as an argument

verdict: PARTIAL — .github/ is byte-identical to origin/main (workflow hunks removed for their own gate PR); fmt, clippy (default and cli-only), `--lib` and the six integration targets are green on gx10; CB-200 measures 652 on both plain main and this branch with pmat 3.43.0 (not measured with pmat 3.42.0 on t2yoga: host load above its threads); 100% coverage, pv ontology, clean-room run and quorum receipt are still owed.

## Identity

- ticket PMAT-893 (forjar#893); kind: code; PR #895, branch `fix/secrets-encryption-default`
- files: `Cargo.toml` defaults, `src/cli/secrets.rs`, `src/cli/secrets_input_tests.rs`, `src/cli/mod.rs`, `src/cli/commands/subcmd_args.rs`, `src/cli/dispatch_misc.rs`, `src/cli/dispatch_misc_b.rs`, `src/main.rs`, `src/core/secrets.rs`, `src/core/tests_secrets.rs`, `tests/falsification_893_secrets_encryption_default.rs`, `tests/falsification_893_secrets_round3.rs`, `tests/falsification_893_secrets_round4.rs`, `tests/falsification_893_secrets_round5.rs`, `tests/falsification_default_features_trim.rs`, `src/cli/tests_cov_args_extra_b.rs`, `src/cli/tests_cov_bundle2.rs`, `src/cli/tests_cov_notify_3.rs`, `src/cli/tests_cov_notify_c.rs`, ``CHANGELOG.md`, `README.md`, `docs/roadmaps/roadmap.yaml`, `docs/book/*`, `docs/cli-defects.json`, `docs/specifications/forjar-cookbook.md`, `examples/dogfood-age-secrets.yaml`

## Changes

- `encryption` (age) is in the default features; `default-features = false` still drops it.
- `secrets keygen -o` creates the identity file 0600 from the first syscall (O_CREAT|O_EXCL) and refuses to overwrite.
- `secrets encrypt`: positional `VALUE` removed (BREAKING). Plaintext comes from stdin (default; `--stdin` is an explicit form of the default, new in this PR) or `--from-file`. A positional value, or `-` as an argv secret, is refused with a message that does not contain it. Usage errors for the whole secrets family (subcommand word `secret*`, case-insensitive), clap's `help`, and an invalid subcommand followed by a later secrets word print a fixed message echoing no argument.
- Leak fix: `exit_redacted` in `src/main.rs` used `std::env::args()`, which panics (printing the whole value, exit 134) on a non-UTF-8 argument. It now uses `args_os()` with `to_string_lossy()`. No other `std::env::args()` call remains in `src/`.
- The CI step and the lint matrix change that came with this PR are NOT in it any more (fix round 1): they are saved as `/mnt/nvme-raid0/cop-inbox/handoff/forjar-cop/gate-pr-895/workflows.patch` for a separate gate PR. Until that lands, no CI job names the 893 test targets and the lint non-encryption leg still builds the default features, which now include age.

## Review rounds

- Round 1-2: clap echoed a PEM secret; `exit_redacted` panicked on non-UTF-8 argv; `secrets ... encrypt` redaction missed global flags between the words. Fixed in 51e1dd84..1ebd17b5.
- Round 3 (9225d9ff): a bad recipient echoed the (possibly secret) value; `--from-file` echoed its path; glued subcommand words (`secrets=S`) were quoted by clap; a failing stdout panicked (abort, core dump). Fixed: `invalid recipient #N ... (value not shown)`, no path, `argv_mentions_secrets`, `emit_line` returns an error. Tests: `tests/falsification_893_secrets_round3.rs`.
- Round 4 findings and fixes:
  1. CI did not run the round-3 file: added it (and round 4) to `ci.yml` beside the 893 step; `--lib` tests that the bad-recipient error omits the input and that `emit_to` maps a failing writer to an error.
  2. `argv_mentions_secrets` matched any word starting with `secrets`, hiding clap's real error for `apply -f secrets.yaml --bogus`: now only the subcommand word (first non-global-flag word) counts, including glued `secrets=S`, `secretsS`, `secret=S`. `argv_is_secrets_encrypt` uses the same position.
  3. Path echoes in `keygen -o`, `decrypt -i`, `view -f`, `rekey`, `rotate` (errors and success lines, plus `load_identity_file`): only the io error kind is printed.
  4. Unbounded plaintext read: stdin and `--from-file` are read through `read_capped` (`take(16 MiB + 1)`, fixed refusal, exit 1).
  5. Core dumps: not fixed in code. `libc`, `nix` and `rustix` appear only as transitive entries in `Cargo.lock`, none is a direct dependency, and `unsafe` is forbidden here, so no `prctl(PR_SET_DUMPABLE, 0)` / `setrlimit(RLIMIT_CORE, 0)` call was added and no dependency was added. Residual risk documented in CHANGELOG ("run under `ulimit -c 0`"); a follow-up ticket for dumpable hardening and zeroizing is owed.
  6. CHANGELOG BREAKING entry reworded to the real scope.
- Round 5 findings and fixes: clap's built-in `help` subcommand (`help secrets encrypt S`, `help S`) and glued/unknown leading flags (`--verbose=S`, `--no-color=S`, `--S`) quoted the value. The subcommand word is now the first word not starting with `-`; a `help` subcommand word or a `secret*` one (case-insensitive) redacts every usage error. `rekey`/`rotate` success lines go through `emit_line`. `--stdin` conflicts with `--from-file`. Tests: `tests/falsification_893_secrets_round5.rs` (incl. a 16 MiB + 1 `--from-file`); CI runs it and the default-features trim test. Book notes `ulimit -c 0`.
- Round 6 findings and fixes: (1) leak: `--verbose S secrets encrypt`, `-v S ...`, `-vv S ... -r x`, `--no-color S ...` put the secret in the subcommand slot (clap `unrecognized subcommand 'S'`); `exit_redacted` now also redacts an InvalidSubcommand error when a later word is `secrets`/`secret*` (`argv_has_later_secrets_word`), without over-redacting `apply -f secrets.yaml --bogus` or `apply --bogus`. (2) `forjar help bogus` has its own fixed message (`invalid arguments for \`forjar help\``, `argv_is_help`), still redacted, exit 2. (3) surviving mutants: `/dev/full` tests for `secrets view` (marker and no-marker), `rekey`, `rotate --re-encrypt` (marker and no-marker) assert exit 1 and "cannot write to stdout". (4) CHANGELOG redaction scope and `--stdin` wording corrected.
- Round 7 findings and fixes: tests lane FAIL: a mutant making `argv_mentions_secrets` case-sensitive (dropping `to_ascii_lowercase`) survived. Added `SECRETS=S` and `Secret=S` asserts to `mentions_predicate_catches_glued_words` and `SECRETS=hunter2SECRET`, `Secret=S` shapes to `glued_subcommand_word_is_never_echoed` (round3). Leak lane PASS; release lane PASS. Wording fixes: roadmap falsifier line, `exit_redacted` doc comment.

## Evidence

- RED first: `cargo test --test falsification_893_secrets_encryption_default non_utf8` failed on the old `main.rs` (thread panicked at `library/std/src/env.rs`, `Err("SEKRIT\xFF")` printed).
- GREEN after the fix: the same test file, 10 passed, 0 failed.
- `cargo fmt --all` clean; `cargo clippy --all-targets -- -D warnings` clean in default features and with `--no-default-features --features cli`; `cargo test --lib cli::secrets` 6 passed, 0 failed.

- Round 4 RED first: new lib tests failed to compile (`emit_to`, `read_capped` absent) and `non_secrets_commands_keep_clap_errors`, `oversized_stdin_is_refused` were red against the round-3 code.
- Round 4 GREEN: `cargo test --lib secrets` 125 passed; `falsification_893_secrets_round3` 5 passed; `falsification_893_secrets_round4` 4 passed; `falsification_893_secrets_encryption_default` 10 passed.
- Mutation checks: reverting `argv_mentions_secrets` to the any-word match turns `mentions_predicate_ignores_secrets_words_outside_subcommand_position` and `non_secrets_commands_keep_clap_errors` red; disabling the over-cap refusal turns `read_capped_refuses_over_the_cap_and_stops_reading` and `oversized_stdin_is_refused` red. Both reverted.

- Round 5/6 RED first, `falsification_893_secrets_round5`: `flag_then_secret_then_secrets_does_not_leak` and `help_bogus_has_its_own_message` red (2 failed, 7 passed) before the `main.rs` change; GREEN after: 9 passed, 0 failed. Restoring `println!` at each of the view, view-no-marker, rekey, rotate and no-marker sites turns `full_stdout::view_rekey_rotate_full_stdout_exit_one` red (1 failed each); all reverted.
- Round 7 RED/GREEN: with the case-sensitive mutant applied, `mentions_predicate_catches_glued_words` and `glued_subcommand_word_is_never_echoed` fail (1 failed each); reverted, both pass.

## Merge with main (2026-11-11, build host t2yoga, wrapper `fc-pre.sh pr-895`)

Merge commit d764f998 (`git merge origin/main`, no rebase). Conflicts and resolution:
- `.github/workflows/lint.yml`, fmt/clippy matrix: main dropped the `ARM64` label from the clean-room runners; this PR renamed the non-encryption legs to `features: cli-only`. Both kept (`[self-hosted, clean-room]` with `cli-only` and `encryption`; macOS legs `cli-only` and `encryption`). The PR's `cli-only` clippy branch (`--no-default-features --features cli`) is unchanged.
- `CHANGELOG.md`: main moved the earlier Unreleased entries under `## [1.34.0]`; the two PMAT-893 entries stay under `## [Unreleased]` above it.
- `docs/roadmaps/roadmap.yaml`: main's label and notes on the preceding row taken; the PMAT-893 row (already on the branch) kept after it. File parses as YAML, 376 rows, one PMAT-893.
The PMAT-893 roadmap row now carries `orch:fable`, `orch-basis:state` and `release: 1.34.1` (the `release:v1.34.0` label is dropped: v1.34.0 is already cut and these entries are Unreleased). The roadmap edit came after the t2yoga run; no test or code reads that row (not re-measured).
No default, file mode or check was changed to resolve a conflict.

Measured on t2yoga after the merge (one run, log `~/src/fc-pre/pr-895.log`, `DONE rc=0`):
- `cargo fmt --all -- --check`: rc 0
- `cargo clippy --all-targets -- -D warnings`: rc 0
- `cargo clippy --all-targets --no-default-features --features cli -- -D warnings`: rc 0
- `cargo test --lib`: 13887 passed, 0 failed, 4 ignored
- `cargo test --locked --test` `falsification_893_secrets_encryption_default` 10 passed; `falsification_893_secrets_round3` 5 passed; `falsification_893_secrets_round4` 4 passed; `falsification_893_secrets_round5` 9 passed; `falsification_default_features_trim` 7 passed; `falsification_version_matches_manifest` 4 passed; 0 failed in all
- LAB reds on plain main that no CI job runs (forjar#858: `falsification_untyped_error_sites_ratchet`, `falsification_budget_dry_run`, `falsification_release_workflow_shape`): not run, not measured.
- Tree synced with a clone of the branch's `.git`, so tests that read history saw the real history.

## Fix round 1 (2026-11-12, three non-author lanes returned FAIL at 66479df)

- Gate content: `git checkout origin/main -- .github/` in commit 47ef31a9; `git diff origin/main --stat -- .github/` is empty. The merge-conflict resolution of `lint.yml` in the section above is therefore no longer part of this branch; it is in the saved patch. No test in this PR reads either workflow, so no test moved.
- CB-200 (`bash scripts/cb200-ratchet.sh`, tree synced with .git, via `fc-pre.sh pr-895`, host gx10, pmat 3.43.0): plain origin/main dbd1ec3e measures 652 (twice); this branch (47ef31a9) measures 652 (twice); recorded ceiling is 651. The branch adds no function below grade A under this instrument. The lanes' 651 for forjar#899 came from pmat 3.42.0 on t2yoga; t2yoga load was 20, 62 and 15 (12 threads) when tried, so that measurement was not made and main under 3.42.0 is not measured. The ceiling and baseline are untouched.
- Re-run on gx10 (one run, `DONE rc=0`): `cargo fmt --all -- --check` rc 0; `cargo clippy --all-targets -- -D warnings` rc 0; same with `--no-default-features --features cli` rc 0; `cargo test --lib` 13887 passed, 0 failed, 4 ignored; `falsification_893_secrets_encryption_default` 10, `_round3` 5, `_round4` 4, `_round5` 9, `falsification_default_features_trim` 7, `falsification_version_matches_manifest` 4 passed, 0 failed.
- Trailers on the fix-round commits have no blank line between them (CB-2113).

## Owed before merge

- 100% coverage receipt of the changed path (not measured)
- pv ontology check
- clean-room run on the tagged commit
- quorum receipt (written by the quorum, not by the author)

IMPL-PMAT-893-RECEIPT-END
