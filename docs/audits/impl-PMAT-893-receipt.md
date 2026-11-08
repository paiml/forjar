# Implementation receipt — PMAT-893 — encryption is a default feature; `secrets encrypt` never takes the secret as an argument

verdict: pending-quorum

## Identity

- ticket PMAT-893 (forjar#893); kind: code; PR #895, branch `w2/895-stdin`
- files: `Cargo.toml` defaults, `src/cli/secrets.rs`, `src/cli/secrets_input_tests.rs`, `src/cli/commands/subcmd_args.rs`, `src/cli/dispatch_misc.rs`, `src/cli/dispatch_misc_b.rs`, `src/main.rs`, `tests/falsification_893_secrets_encryption_default.rs`, `tests/falsification_default_features_trim.rs`, `.github/workflows/{ci,lint}.yml`, `CHANGELOG.md`, `README.md`, `docs/roadmaps/roadmap.yaml`

## Changes

- `encryption` (age) is in the default features; `default-features = false` still drops it.
- `secrets keygen -o` creates the identity file 0600 from the first syscall (O_CREAT|O_EXCL) and refuses to overwrite.
- `secrets encrypt`: positional `VALUE` removed (BREAKING). Plaintext comes from stdin (default; `--stdin` accepted for compatibility) or `--from-file`. A positional value, or `-` as an argv secret, is refused with a message that does not contain it. Usage errors for this subcommand print a fixed message echoing no argument.
- Leak fix: `exit_redacted` in `src/main.rs` used `std::env::args()`, which panics (printing the whole value, exit 134) on a non-UTF-8 argument. It now uses `args_os()` with `to_string_lossy()`. No other `std::env::args()` call remains in `src/`.
- CI runs the secrets-encryption test per PR; lint keeps the non-encryption leg.

## Evidence

- RED first: `cargo test --test falsification_893_secrets_encryption_default non_utf8` failed on the old `main.rs` (thread panicked at `library/std/src/env.rs`, `Err("SEKRIT\xFF")` printed).
- GREEN after the fix: the same test file, 10 passed, 0 failed.
- `cargo fmt --all` clean; `cargo clippy --all-targets -- -D warnings` clean in default features and with `--no-default-features --features cli`; `cargo test --lib cli::secrets` 6 passed, 0 failed.

## Owed before merge

- 100% coverage receipt of the changed path
- pv ontology check
- clean-room run on the tagged commit
- quorum receipt (written by the quorum, not by the author)

IMPL-PMAT-893-RECEIPT-END
