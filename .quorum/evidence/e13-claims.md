# Quorum evidence — #408 (CRUX audit E13) — adjudicated claims

## CONFIRMED — 8 claims survived refutation

1. [probe] (explains-symptom) The only invocation that actually signed anything put the secret itself on the command line, where every local user reads it out of `ps`.
   - evidence: `cmd_lock_sign` at src/cli/lock_merge.rs:166 took `key: &str` and hashed it verbatim at src/cli/lock_merge.rs:186 (`blake3(content ++ key)`); the flag's help at src/cli/commands/lock_core_args.rs:187 read "path to key file or inline", and the file half did not exist — a path was hashed as a string. Same shape on `lock-verify-sig --key` (src/cli/lock_security.rs:34), `lock-rotate-keys --old-key/--new-key` (src/cli/commands/lock_core_args.rs:253 and :257) and `lock-verify-chain --key`. Pinned by `key_file_ref_must_resolve_to_the_files_contents` in tests/falsification_e13_signing_key_argv.rs, RED on main (`left: 1 right: 0`).

2. [design] One resolver, `core::key_source`, is the only place a key argument becomes key material, and an unreadable or empty source is an error rather than a fallback to the spec string.
   - evidence: at base `cmd_lock_sign` (src/cli/lock_merge.rs:166) and `cmd_lock_verify_sig` (src/cli/lock_security.rs:34) took `key: &str` straight from clap and hashed it at src/cli/lock_merge.rs:186 and src/cli/lock_security.rs:22. Now `file:<PATH>` reads the file (trimmed), `env:<VAR>` reads the variable (the environment is not in `ps`), and anything else is the literal — still accepted so existing signatures keep verifying, warned on every use, and removed in 2.0.0 (`INLINE_KEY_REMOVAL_VERSION`). The fallback was the quiet half of the bug: unfixed, `--key file:/nope` printed "Signed 1 lock file(s)" and left every lock signed with a key nobody holds. Pinned by `missing_key_file_must_fail_without_writing_a_signature`, `empty_key_file_must_fail_without_writing_a_signature` and `unset_env_ref_must_fail_without_writing_a_signature`.

3. [probe] (explains-symptom) The first cut left the VERIFIER's own resolve call unpinned — deleting it kept all ten tests green.
   - evidence: `verify_machine_sig` at src/cli/lock_security.rs:11 computes `blake3(content ++ key)` from whatever string it is handed; every first-cut case signed AND verified with the same spec form, so a verifier that hashed the literal `file:<path>` still agreed with a signer that did the same. Found by mutation, not by reading. `verify_sig_must_resolve_a_key_ref` crosses the forms — sign with the material, verify by naming a file — and also pins the REASON an unreadable key file fails, because exit 1 alone cannot tell "your key file is missing" from "your signature is bad". With the resolve line in src/cli/lock_security.rs removed, that one test fails and the other ten pass; restored, 11/11.

4. [design] Every flag that takes key material is covered — sign, verify-sig, rotate-keys (both keys), verify-chain — and each one's `--help` documents the indirect forms.
   - evidence: `lock-verify-chain` at src/cli/lock_chain.rs:87 was the fourth flag and the one command whose help nobody had asserted; `help_must_document_the_indirect_forms` now loops over all four, and `rotate_keys_must_resolve_both_key_refs` and `verify_chain_must_resolve_a_key_ref` pin the two the first cut had covered by code but not by test.

5. [design] The deprecation names its removal version and fires once per inline key, not once per process.
   - evidence: the flag's help at src/cli/commands/lock_core_args.rs:187 read "path to key file or inline" while the file half did not exist; `inline_key_must_warn_with_a_named_removal_version` asserts the warning carries `2.0.0`; `rotate_keys_must_warn_for_each_inline_key` asserts two warnings for `--old-key k1 --new-key k2`, one naming each flag, because a single unattributed warning would leave the operator guessing which of two arguments to fix.

6. [probe] (explains-symptom) The book's entry point taught the unsafe invocation while its reference chapters taught the safe one.
   - evidence: docs/book/src/01-getting-started.md prescribed `--key my-signing-key` in four places — the exact argv form this ticket deprecates — while 06-cli and 14-state-safety showed `file:`/`env:`. All four corrected; a new reader now never sees a literal key on a command line.

7. [agy] (partially-explains) A key file readable by other users is the argv leak one directory over, and nothing said so.
   - evidence: at base the key never touched a file at all — src/cli/lock_merge.rs:186 hashed the argv string — and the first cut's `read_key_file` in src/core/key_source.rs read and trimmed the file and never looked at its mode; ssh refuses such a key outright ("UNPROTECTED PRIVATE KEY FILE"). Taken: `warn_if_shared` warns with the mode and `chmod 600` when `mode & 0o077 != 0` — warns rather than refuses, so a fleet already signing from a shared file gets a release to fix its modes rather than a broken pipeline. Pinned by `a_world_readable_key_file_is_warned_about`, RED with the call removed; a 0600 file must not warn.

8. [agy] (partially-explains) Two withdrawal tests asserted exit 1 and "no signature written" but not WHY, so any initialisation failure would have satisfied them.
   - evidence: `empty_key_file_must_fail_without_writing_a_signature` and `unset_env_ref_must_fail_without_writing_a_signature` in tests/falsification_e13_signing_key_argv.rs checked `code == 1` and the absence of `lock.sig`; a missing state dir prints exit 1 and writes nothing too. Taken: each now requires the key source's own reason on stderr ("is empty" / "is not set").

## REFUTED — 3 claims killed

1. [design] refuted 1/1 — Accepting the inline literal at all, even with a warning, defeats the fix.
   - corrected: Removing it outright would silently invalidate every signature on every fleet signed before this change, because `blake3(content ++ key)` over the literal (src/cli/lock_merge.rs:186, src/cli/lock_security.rs:22) IS the recorded signature; a verify that could no longer be given the same bytes would fail with no path forward. The literal stays, warns on every use with the flag named and the removal version stated, and goes in 2.0.0. This is the shape `ssh` took for `-o PasswordAuthentication` and `git` for `credential.helper store`: deprecate loudly, remove on a major.

2. [design] refuted 1/1 — `file:`/`env:` prefixes are ambiguous with a literal key that happens to start with `file:` or `env:`.
   - corrected: Such a literal was ALREADY the unsafe form — the only form src/cli/commands/lock_core_args.rs:187 documented — and is exactly what the deprecation is retiring; an operator whose key begins with `env:` types `env:KEY` after putting it in `KEY`, which is what they should have been doing. Recorded as a known limit of the transition period rather than a design defect.

3. [agy] refuted 1/1 — `env:<VAR>` leaks the key to every child process because `read_key_env` never `remove_var`s it.
   - corrected: The variable was in the process environment before forjar started — the operator put it there, which is the same contract `COSIGN_PASSWORD` and `docker login --password-stdin`'s `DOCKER_PASSWORD` rely on — and `std::env::remove_var` is unsound in a multi-threaded process (it is `unsafe` from Rust 2024 for exactly that reason). Children of a signing process inheriting the signing process's environment is the operator's model, not a leak forjar introduced; recorded as a known limit with the mitigation (`env -u KEY` around any child, or `file:` with 0600) rather than a soundness bug added to fix a hypothetical one.
