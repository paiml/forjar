# Judges — Default `encryption` feature & argv-free `secrets encrypt` (PMAT-893 / forjar#895)

Round count and heads: Round 1 `66479df3` (`0/3 PASS`), Round 2 `b9a7e78b` (`3/3 PASS`); see `PMAT-893-lanes.md`. Every claim `C1..C4` and `R1..R2` met 3 independent refuters.

## CONFIRMED

1. [code] C1 — `Cargo.toml:191` enables `default = ["cli", "encryption"]` so default CLI builds ship `age` encryption while `--no-default-features` keeps library builds free of `age`.
   - evidence: `tests/falsification_default_features_trim.rs:120` and `tests/falsification_893_secrets_encryption_default.rs:80` (`10/10`) passed on `<host>`, and reverting `Cargo.toml:191` failed `8/10` tests (`FJ_895_REVERT_RC=101`).
2. [code] C2 — `src/cli/commands/subcmd_args.rs:200`, `src/cli/secrets.rs:60`, and `src/main.rs:25` remove positional secret argv, bound stdin/`--from-file` reads to `16 MiB`, and redact `secret*` clap errors including non-UTF-8 `args_os()`.
   - evidence: `positional_value_is_refused_and_never_echoed`, `clap_errors_on_secrets_encrypt_never_echo_value`, and `non_utf8_secret_argv_is_never_echoed_or_panics` in `tests/falsification_893_secrets_encryption_default.rs:80` all passed on `<host>`.
3. [code] C3 — `src/cli/secrets.rs:150` writes `keygen -o` files atomically at mode `0600` (`O_CREAT | O_EXCL`) refusing overwrite, and `emit_line` turns stdout write failures into clean exit `1` errors instead of panics.
   - evidence: `keygen_output_file_is_created_0600`, `keygen_output_refuses_to_overwrite_existing_file`, and broken-pipe tests in `tests/falsification_893_secrets_round5.rs:40` passed on `<host>`.
4. [doc] C4 — `CHANGELOG.md:8`, `README.md:120`, and `docs/book/src/24-secret-providers.md:40` update all user-facing `secrets encrypt` examples to stdin and `--from-file`.
   - evidence: Both `cargo clippy --all-targets -- -D warnings` and `cargo clippy --no-default-features --features cli --all-targets -- -D warnings` passed (`rc=0`) on `<host>`.

## REFUTED

1. [code] R1 — Round-1 `66479df3` bundled blocking CI workflow changes (with an unreachable `else` branch in the lint matrix) into the feature PR without separate gate sign-off under `RULES-GATES-1111`.
   - corrected: Commit `47ef31a9` (`docs/audits/impl-PMAT-893-receipt.md:20`) stripped all `.github/` edits out of `#895` (`git diff origin/main...b9a7e78b -- .github/` is empty) so code and gate changes are reviewed independently.
2. [test] R2 — Round 1 flagged `CB-200 = 652` against ceiling `651` before running a control measurement on plain `origin/main` with the same `pmat 3.42.0` binary.
   - corrected: Differential measurement on `<host>` confirmed plain `origin/main` (`dbd1ec3e`) also reports `652` below-A functions (`+0` delta introduced by `src/cli/secrets.rs:60` or `src/core/secrets.rs:100`).
