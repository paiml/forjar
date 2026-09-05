# pmat lane — PMAT-159 — analyze_vacuous_tests

Tool: `pmat 3.37.0`, `pmat analyze vacuous-tests`.

## How it was run, and why not per file

The four touched test paths were each tried directly and the tool refused, by
design and correctly:

```text
$ pmat analyze vacuous-tests -p tests/falsification_sudo_transport_closefrom_emulated.rs
Error: git ls-files failed in tests/falsification_sudo_transport_closefrom_emulated.rs
       — cannot enumerate tracked files, so the scan would have no denominator
```

That refusal is the same rule this repo applies to its own guards: a scan with no
denominator cannot report a rate. So the analysis was run over the repository and
the result filtered to the touched paths, which keeps the denominator honest and
is printed below.

## Raw output

```text
$ pmat analyze vacuous-tests -p . -f summary
360 of 19307 #[test] fns cannot fail (1.9%) across 2099 parsed file(s); 4 more skip silently when a fixture is missing
  [no-failure-mode] crates/forjar-contracts/src/kernels/batchnorm_tests2.rs:6  test_batchnorm_avx2_parity_training
  [no-failure-mode] crates/forjar-contracts/src/kernels/batchnorm_tests2.rs:58  test_batchnorm_avx2_parity_inference
  [no-failure-mode] crates/forjar-contracts/src/kernels/flash_attention_tests.rs:118  test_flash_single_element
  [no-failure-mode] crates/forjar-contracts/src/kernels/flash_attention_tests.rs:216  test_flash_avx2_parity
  [no-failure-mode] src/cli/bootstrap_cmd.rs:226  test_resolve_pub_key_default
  [no-failure-mode] src/cli/output.rs:129  test_stdout_writer_does_not_panic
  … and 353 more
  [silent-skip] crates/forjar-contracts/src/query/query_tests_coverage.rs:252  coverage_map_enrichment  if ! root . parent () . is_some_and (| p | p . join ("aprender") . exists ())
  [silent-skip] src/core/resolver/tests_template.rs:94  test_fj062_secret_inner  if std :: env :: var ("FORJAR_SECRET_TEST_KEY") . is_err ()
  [silent-skip] src/core/store/tests_convergence_runner.rs:316  detect_container_runtime_finds_something  if rt . is_none ()
  [silent-skip] src/core/tests_secrets.rs:214  test_fj200_load_identity_env_inner  if std :: env :: var ("FORJAR_AGE_KEY") . is_err ()
```

Counts from the JSON form of the same run: `tests_examined` 19307,
`files_parsed` 2099, 360 vacuous (353 no-failure-mode, 7 tautology), four
conditional skips, and `skipped.unmeasured_tests` 0 — nothing went unread.

## Result for the touched paths

Filtering the 360 vacuous entries and the four silent skips to the four paths
this branch touches:

```text
tests/falsification_sudo_transport_closefrom_emulated.rs   0
tests/falsification_sudo_transport_survives_closefrom.rs   0
tests/falsification_390e_nested_shell_strictness.rs        0
src/core/codegen/tests_sudo.rs                             0
```

vacuous_tests_in_touched_paths = 0. Nothing to accept.

## The limit this lane has, stated rather than implied

The zero is a true reading of what the tool measured, and it is NOT a claim that
every touched test can fail. `tests/falsification_sudo_transport_survives_closefrom.rs`
skips and returns when the host has no passwordless sudo, which is exactly the
`[silent-skip]` shape the tool reports elsewhere — and it was not reported,
because the guard is behind a `require_sudo!` macro rather than a literal `if
… { return; }` in the token stream the detector reads. The quorum reached the
same conclusion by other means and killed claim B3 over it. That is why this
receipt's falsification target is the EMULATED test, which needs no privilege
and runs everywhere, and not the live one.
