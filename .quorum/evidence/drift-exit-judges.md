# PMAT-562 — adjudicated claims

Two rounds of three sandboxed agy quorum lanes — the paiml-implement review
(1 FAIL, 1 PASS, 1 voided on an API 503) and the merge rail's second round
over the head that carried the rustls bump (2 PASS, 1 FAIL). Six
confirmations and five refutations, every one re-measured on this host
before it was acted on. The verdict held; what did not hold was the
branch's claim to have finished the documentation, and one of its own tests'
claim to be asserting what its name said.

## CONFIRMED

1. [no-ok-on-drift] That no path through `cmd_drift` prints a DRIFTED
   finding and returns Ok (lanes 1 and 3, both from reading the function).
   - evidence: the return is unconditional at `src/cli/drift.rs:381` and the
     unmeasured-only return at `src/cli/drift.rs:387`; the flag is bound to
     `_tripwire_compat` at `src/cli/drift.rs:291` and read nowhere. `--dry-run`
     returns before the scan; `--alert-cmd` and `--auto-remediate` run inside
     the `total_drift > 0` block at `src/cli/drift.rs:356` and fall through to
     the return. Re-run here: `a_drifted_file_resource_exits_1_without_any_flag`
     at `tests/falsification_drift_verdict_reaches_the_exit_code.rs:114`, rc=1.

2. [apply-unchanged] That `apply --abort-on-drift` is unaffected (lanes 1
   and 3).
   - evidence: it passed `true` for the flag before the change, so it already
     rejected on drift; the parameter is now ignored and the outcome is the
     same. The only callers that depended on Ok-over-drift were tests that
     pinned the defect: `src/cli/tests_check.rs:191` and `:311`,
     `src/cli/tests_drift.rs:256`, all four now asserting the reject.

3. [control] That a converged stack still exits 0 (both lanes; re-run here).
   - evidence: `tests/falsification_drift_verdict_reaches_the_exit_code.rs:132`;
     mutation M3 (reject unconditionally) kills exactly this case and no other,
     so the case measures the direction it names.

4. [json-too] That `--json` exits 1 on drift and its report carries
   `drift_count` (lane 3 measured; lane 1 read).
   - evidence: `tests/falsification_drift_verdict_reaches_the_exit_code.rs:168`;
     `run_single_check`-style stream merging in an older helper broke on the
     verdict line that now follows the report on stderr, fixed at
     `tests/falsification_ignore_drift_names_one_field.rs:117` to read the first
     JSON document.

5. [flag-kept] That an accepted, inert `--tripwire` is the right shape (lanes
   1 and 3 independently; lane 1 named the failure removing it causes).
   - evidence: clap rejects an unknown argument with exit 2 and a usage line,
     which would turn every fleet cron line into an error; the flag stays at
     `src/cli/commands/misc_args.rs:32` with the help text at
     `src/cli/commands/misc_args.rs:28` saying it changes nothing.

6. [contract] That `contracts/drift-verdict-exit-v1.yaml` and the CHANGELOG
   entry contain no false sentence (both counted lanes).
   - evidence: the CHANGELOG paragraph at `CHANGELOG.md:10` names the
     measurement, the flag's fate, `--json`, remediation and the unmeasured
     exit; each falsifier's mutation was run here (M1, M2, M3 in the pmat
     digest) rather than reasoned from the test body as the lanes did.

## REFUTED

1. [docs-done] That the book's every claim about drift's exit code was
   corrected in the first commit (this author, in the commit message of
   8e3af871).
   - corrected: lane 1 grepped and found ten sites still selling `--tripwire`
     as the exit-code switch or saying drift exits 10 —
     `docs/book/src/01-getting-started.md:173`, `06-cli.md:184`,
     `docs/specifications/forjar-spec.md:1293`, `examples/image_drift.rs:41`,
     the doc comments at `src/core/error.rs:66` and `:166`, and four more; the
     voided lane 2 found nine of the same. Fourteen sites fixed in 22a53170
     (four more surfaced grepping the cookbook chapter). Two dated audit
     records that quote 1.12.3's help verbatim were left as records.

2. [no-op-asserted] That `tripwire_is_a_no_op` asserted the flag changes
   nothing (this author, in the contract's FALSIFY-DRIFT-EXIT-002).
   - corrected: mutation M2 — let the flag append ` [tripwire]` to the error
     message — SURVIVED. The case compared stdout only, and the verdict line
     is on stderr. It compares stderr too now, at
     `tests/falsification_drift_verdict_reaches_the_exit_code.rs:160`, and the
     re-run kills it.

3. [ratchet-flat] That the branch leaves the CB-21xx ratchet where main has
   it (this author, before running gate B).
   - corrected: CB-2114 read 35 against a ceiling of 34 — the new roadmap row
     carried no `release:` and its issue was on no milestone. The convention
     the ceiling file records (issue first, tail is the id, milestone,
     `release:` written textually) was applied; 34 after. CB-2112 (36) and
     CB-2115 (49) are main's own debt, measured at 36 and 50 there.

4. [sandbox-green] That the acceptance command is green in every clone
   (lane 1 reported two `cli::tests_check_2` cases dying with NotFound).
   - corrected: both cases pass on this host and passed in lane 3's clone;
     neither is in the diff and neither reads anything the branch changed.
     Recorded as a sandbox artefact of lane 1's clone rather than a defect —
     and named here because a lane's red that the author explains away is the
     shape a reviewer should be able to check. The command to check it:
     `cargo test --lib -- cli::tests_check_2::tests::test_fj017_check_machine_filter`.

5. [lock-scope] That the rustls advisory bump (828052d9) changed the lock
   for rustls only (this author, in that commit's subject).
   - corrected: the merge rail's lane 3 (gemini-3.1-pro-high) read
     `Cargo.lock:3760` and found tempfile's dependency re-resolved from
     `getrandom 0.4.3` to `0.3.4` — admitted by tempfile 3.27.0's
     `>=0.3.0, <0.5`, but nothing asked for it and the receipt was silent.
     Reverted by hand in fe9fce31; `cargo metadata --locked` accepts the lock,
     `cargo audit` is clean, and the diff against main now touches
     `Cargo.lock:3154` (rustls 0.23.43 → 0.23.45) and nothing else in that file.

