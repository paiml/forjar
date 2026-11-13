# Claims — Default `encryption` feature & argv-free `secrets encrypt` (PMAT-893 / forjar#895)

Extracted from PR `#895` (`fix/secrets-encryption-default`, Round 1 `66479df3` -> Round 2 `b9a7e78b`) and attacked across 3 lanes:

- **C1 (`encryption` enabled in default Cargo features while `default-features = false` stays minimal)**: `Cargo.toml:191` sets `default = ["cli", "encryption"]` while library builds with `--no-default-features` exclude `age` (`tests/falsification_default_features_trim.rs:120`).
- **C2 (Argv-free `secrets encrypt` via bounded stdin/`--from-file` & clap error redaction)**: `src/cli/commands/subcmd_args.rs:200` and `src/cli/secrets.rs:60` remove positional `VALUE`, read bounded (`<= 16 MiB`) input from stdin or `--from-file`, and `src/main.rs:25` redacts clap parse errors across `secret*` invocations including non-UTF-8 `args_os()` (`tests/falsification_893_secrets_encryption_default.rs:80`).
- **C3 (`keygen -o` atomic `0600` `O_EXCL` creation & panic-free `emit_line` stdout writes)**: `src/cli/secrets.rs:150` creates identity files with `0o600` (`create_new(true)`) refusing overwrite, and replaces `println!` with `emit_line` so broken pipes exit `1` cleanly (`tests/falsification_893_secrets_round5.rs:40`).
- **C4 (Documentation, cookbook alignment & revert kill)**: `CHANGELOG.md:8`, `README.md:120`, and `docs/book/src/24-secret-providers.md:40` document stdin/`--from-file` usage, and reverting `Cargo.toml:191` drives `8/10` tests RED (`FJ_895_REVERT_RC=101`).
- **R1 (Round-1 `66479df3` bundled blocking `.github/workflows/` edits without gate sign-off)**: Round 1 modified CI workflow files alongside product code in violation of `RULES-GATES-1111`; split out in `47ef31a9`.
- **R2 (Round-1 `CB-200` count `652` misattributed before `origin/main` differential check)**: Plain `origin/main` (`dbd1ec3e`) measures the exact same `652` functions on `pmat 3.42.0` (`+0` delta from `#895`).
