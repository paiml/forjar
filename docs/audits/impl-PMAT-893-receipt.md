# Implementation receipt — PMAT-893 — encryption is a default feature; `secrets encrypt` never takes the secret as an argument

verdict: pending-quorum

## Identity

- ticket PMAT-893 (forjar#893); kind: code; PR #895, branch `fix/secrets-encryption-default`
- files: `Cargo.toml` defaults, `src/cli/secrets.rs`, `src/cli/secrets_input_tests.rs`, `src/cli/mod.rs`, `src/cli/commands/subcmd_args.rs`, `src/cli/dispatch_misc.rs`, `src/cli/dispatch_misc_b.rs`, `src/main.rs`, `src/core/secrets.rs`, `src/core/tests_secrets.rs`, `tests/falsification_893_secrets_encryption_default.rs`, `tests/falsification_893_secrets_round3.rs`, `tests/falsification_893_secrets_round4.rs`, `tests/falsification_893_secrets_round5.rs`, `tests/falsification_default_features_trim.rs`, `src/cli/tests_cov_args_extra_b.rs`, `src/cli/tests_cov_bundle2.rs`, `src/cli/tests_cov_notify_3.rs`, `src/cli/tests_cov_notify_c.rs`, `.github/workflows/{ci,lint}.yml`, `CHANGELOG.md`, `README.md`, `docs/roadmaps/roadmap.yaml`, `docs/book/*`, `docs/cli-defects.json`, `docs/specifications/forjar-cookbook.md`, `examples/dogfood-age-secrets.yaml`

## Changes

- `encryption` (age) is in the default features; `default-features = false` still drops it.
- `secrets keygen -o` creates the identity file 0600 from the first syscall (O_CREAT|O_EXCL) and refuses to overwrite.
- `secrets encrypt`: positional `VALUE` removed (BREAKING). Plaintext comes from stdin (default; `--stdin` accepted for compatibility) or `--from-file`. A positional value, or `-` as an argv secret, is refused with a message that does not contain it. Usage errors for this subcommand print a fixed message echoing no argument.
- Leak fix: `exit_redacted` in `src/main.rs` used `std::env::args()`, which panics (printing the whole value, exit 134) on a non-UTF-8 argument. It now uses `args_os()` with `to_string_lossy()`. No other `std::env::args()` call remains in `src/`.
- CI runs the secrets-encryption test per PR; lint keeps the non-encryption leg.

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

## Evidence

- RED first: `cargo test --test falsification_893_secrets_encryption_default non_utf8` failed on the old `main.rs` (thread panicked at `library/std/src/env.rs`, `Err("SEKRIT\xFF")` printed).
- GREEN after the fix: the same test file, 10 passed, 0 failed.
- `cargo fmt --all` clean; `cargo clippy --all-targets -- -D warnings` clean in default features and with `--no-default-features --features cli`; `cargo test --lib cli::secrets` 6 passed, 0 failed.

- Round 4 RED first: new lib tests failed to compile (`emit_to`, `read_capped` absent) and `non_secrets_commands_keep_clap_errors`, `oversized_stdin_is_refused` were red against the round-3 code.
- Round 4 GREEN: `cargo test --lib secrets` 125 passed; `falsification_893_secrets_round3` 5 passed; `falsification_893_secrets_round4` 4 passed; `falsification_893_secrets_encryption_default` 10 passed.
- Mutation checks: reverting `argv_mentions_secrets` to the any-word match turns `mentions_predicate_ignores_secrets_words_outside_subcommand_position` and `non_secrets_commands_keep_clap_errors` red; disabling the over-cap refusal turns `read_capped_refuses_over_the_cap_and_stops_reading` and `oversized_stdin_is_refused` red. Both reverted.

- Round 5/6 RED first, `falsification_893_secrets_round5`: `flag_then_secret_then_secrets_does_not_leak` and `help_bogus_has_its_own_message` red (2 failed, 7 passed) before the `main.rs` change; GREEN after: 9 passed, 0 failed. Restoring `println!` at each of the view, view-no-marker, rekey, rotate and no-marker sites turns `full_stdout::view_rekey_rotate_full_stdout_exit_one` red (1 failed each); all reverted.

## Owed before merge

- 100% coverage receipt of the changed path
- pv ontology check
- clean-room run on the tagged commit
- quorum receipt (written by the quorum, not by the author)

IMPL-PMAT-893-RECEIPT-END
