# pmat MCP lane — #408

`analyze_vacuous_tests` over the whole worktree: 17832 tests examined. One
`no-failure-mode` finding sits in a file this branch touches
(`src/cli/tests_cov_args_extra.rs::test_cov_lock_merge_args_construct`) — a
PRE-EXISTING clap-struct constructor test in a file the branch edited only
to follow the renamed key-argument help; named and accepted in the receipt.
ZERO findings in `src/core/key_source.rs`, the four lock verbs, or
`tests/falsification_e13_signing_key_argv.rs`.

Falsification, tests kept and production reverted:

- all of `src/` reverted to the parent: **10 of 10** first-cut tests RED,
  each for its own reason (`left: 1 right: 0` on the file-ref case; exit 0
  with a signature written on the missing-file case; no warning on the
  inline case; help without the indirect forms).
- the verifier's `key_source::resolve` line alone deleted:
  `verify_sig_must_resolve_a_key_ref` RED, the other ten green — which is
  the gap that test was written to close.

`falsification_e13_signing_key_argv`: 10 of 10 green on the branch, plus
`falsification_e13_key_file_hygiene` (the mode-warning and help cases, split
out when the first file crossed the 500-line budget): 2 of 2. Full lib
suite green; clippy `-D warnings` 0; fmt clean — counts in `gates`.
