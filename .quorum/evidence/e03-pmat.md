# pmat MCP lane — #405

`analyze_vacuous_tests` over the whole worktree: 17766 tests examined. Eight
`no-failure-mode` findings sit in files this branch touches
(`src/cli/tests_cov_args_extra.rs`, `src/cli/tests_cov_lock.rs`); every one is
a PRE-EXISTING constructor/smoke test that the branch edited only to delete
its references to the withdrawn `--pq` and `lock-verify-hmac` surfaces. They
are named and accepted in the receipt with that reason rather than deleted
here — thinning coverage tests is its own change. ZERO findings in the
falsification file or in any production file the branch touches.

Falsification, tests kept and production reverted:

- all of `src/` and `docs/` reverted to the parent: **5 of 6 RED**, each with
  the exact payload the issue quotes (`"valid": true, "signer": "root@prod"`;
  `both_valid: true`; `{"verified":1,"unsigned":0}`; "unrecognized subcommand
  'digest'"). The sixth, `lock_verify_sig_rejects_a_one_byte_mutation`, is a
  labelled regression guard, green on both trees.
- `digest --verify`'s algorithm guard alone reverted (echo
  `recorded.algorithm`): `a_forged_algorithm_name_is_neither_believed_nor_echoed`
  **RED** with `"valid": true, "algorithm": "ed25519-dsse"`.
- `current_hash == recorded.blake3_hash` mutated to `true`:
  `digest_verify_fails_on_a_one_byte_mutation` **RED**.

`withdrawn()` tightened during the quorum: it now requires clap's message to
quote the withdrawn name. Full lib suite green; clippy `-D warnings` 0; fmt
clean — exact counts in the receipt's `gates`.
