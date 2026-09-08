# Quorum evidence — 1.27.0 fleet P0 — adjudicated claims

## CONFIRMED

1. [scope] DRIFT-SCOPE — `drift -f <config>` now checks only the machines that config declares, and a stack it does not declare is never opened, so the lock-only arm that reads the local filesystem is unreachable for it.
- evidence: the arm is the `_ =>` fallthrough after `let machine = config.and_then(|c| c.machines.get(name));` at src/cli/drift.rs:38, which resolves at the merge base and is the line the whole defect turns on. Filtering happens in the state-dir walk instead, so the stack is never opened and there is no arm to reach. Measured through the real binary on a two-stack fixture: before, `Checking faraway` and `DRIFTED: probe-two` from a config declaring only this host; after, `Checking noah-Lambda-Vector` alone and `No drift detected.`

2. [attribution] DRIFT-ROW-NAMES-MACHINE — every DRIFTED row now names the machine it came from, so an aggregated run can be attributed after the fact.
- evidence: the row was printed without the machine at src/cli/drift.rs:115 at the merge base, while the JSON branch three lines above it already carried `"machine": name`. That asymmetry is exactly how the operator read gx10's `bashrc` as yoga's: the text output, which is what a person reads, was the half missing the attribution. Measured after: `DRIFTED: probe-two on faraway (...)`.

3. [recovery] REFRESH-PERSISTS — `--refresh` re-checks an entry the lock records as failed AND writes the promotion down, so one run is a way back rather than a flag to pass for ever.
- evidence: `refresh_locks` at src/core/executor/refresh.rs:86 (merge base) builds the view the PLANNER reads; that view is discarded and the original locks are written, which is why the first version of this fix left `status: failed` on disk. Measured end to end against the real binary: apply `1 FAILED`, lock `status: failed`, `--refresh` `0 converged, 1 unchanged`, lock `status: converged`, plain apply `0 converged, 1 unchanged`, and the always-failing command run once in total.

4. [environment] CARGO-SHARED-PRELUDE — the cargo check and the cargo drift observable emit the same PATH repair the install action emits, so an installed crate is no longer reported missing.
- evidence: the check's cargo arm begins at src/resources/package_check.rs:32 at the merge base and asked `cargo install --list` with whatever PATH a non-interactive shell had. One shared emitter now precedes all three sites. Measured by EXECUTION, not by reading: the emitted check run against a stub toolchain reachable only through `$CARGO_HOME/bin` printed `missing:ripgrep` before and reports it installed after.

5. [falsification] MUTATION-KILLS — each fix is pinned by a case that fails when that one line is removed, and the cases were checked against the failure mode this repository shipped one day earlier.
- evidence: the drift suite lives at tests/falsification_drift_scopes_to_the_config_it_was_given.rs:275 and is a file this branch adds; commenting out the refusal call fails exactly one of its six cases. The cargo suite's text assertions search comment-stripped copies, because RULE 8 of the release-workflow shape gate was satisfied by the comment explaining it and had to be repaired in #484 the day before.

## REFUTED

6. [false-green] DRY-RUN-BYPASS — the branch introduced a guard against a false green and left the same false green reachable through `--dry-run`; two of three lanes found it independently and it reproduced.
- corrected: `cmd_drift_dry_run` calls the state-dir walk directly rather than through `collect_machine_locks` where the guard sat, so `drift --dry-run -m <undeclared>` printed `0 resource(s) would be checked` and exited 0. The refusal now runs in both doors. The four original cases missed it because none exercised `-m`; the acceptance criterion had no test, which is the more useful half of this finding.

7. [environment] CARGO-HOME-SHADOWED — the bootstrap block still hard-coded `$HOME/.cargo/bin`, which shadows the CARGO_HOME repair the same script had just made.
- corrected: measured with `CARGO_HOME=/opt/toolchain`, the prelude put `/opt/toolchain/bin` first and the hard-coded export then put `$HOME/.cargo/bin` ahead of it. `rustup-init` honours `CARGO_HOME`, so the shadowing directory is the one rustup did NOT install into. Both lines now use the same expression, and the unit test at src/resources/tests_package_b.rs:233 that matched the literal `.cargo/bin:$PATH` now asserts the CARGO_HOME form.

8. [scope] BUNDLED-TICKETS — one lane refused the branch for carrying three tickets, and the objection is accurate about PMAT-212's ticket text.
- corrected: answered rather than accepted, and the answer is recorded in the pull request body rather than left implicit. The operator declared four issues P0 with the fleet blocked; the self-hosted pool was measured at 14 of 16 runners busy, so three pull requests would serialise three CI passes behind that queue. Each ticket keeps its own commit, receipt, falsification test and roadmap row, so the per-ticket record survives a shared CI pass and a reviewer can still read the branch one ticket at a time.
