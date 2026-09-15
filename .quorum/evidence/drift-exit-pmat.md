# PMAT-562 — instruments, and what each said

    cargo test --workspace --no-fail-fast   339 targets, 19,779 passed, 2 failed
                                            at 8e3af871 — the two were
                                            cli::tests_drift's FJ-017 cases,
                                            which PINNED the fail-open default
                                            (`Ok(())` over a detected drift);
                                            inverted in d1347afe, then green
    cargo clippy --all-targets -D warnings   clean
    cargo fmt --all -- --check               clean
    bash scripts/dogfood/surface.sh          GATE C PASS (211 CLI names live)
    bash scripts/dogfood/docs.sh             GATE D PASS (18 README invocations)
    bash scripts/dogfood/contracts.sh        GATE G PASS (41 contracts, citations resolve)
    bash scripts/dogfood/comply.sh           GATE B FAIL — the CB-21xx ratchet:
                                            CB-2112 36 > 35, CB-2115 49 > 43.
                                            Measured on origin/main in a
                                            detached worktree: 36 and 50. Both
                                            are main's debt (ISSUE-CLOSED 2 on
                                            PMAT-547/PMAT-555 rows that were
                                            merged today and not yet booked;
                                            five open issues #557–#561 with no
                                            roadmap row). This branch is not
                                            worse on any of the three. CB-2114
                                            read 35 before the roadmap row
                                            carried `release: 1.31.0` and the
                                            issue was put on the milestone;
                                            34 after, which is the ceiling.
    pmat analyze vacuous-tests               not run on this branch; the new
                                            suite drives the binary and each
                                            case has a control that fails on
                                            the opposite verdict
    pv validate contracts/drift-verdict-exit-v1.yaml   Contract is valid.
    PRINT_HASH=1 bash scripts/quorum-gate.sh   the diff hash in the receipt

Mutations over the COMMITTED tree, each restored with `git checkout HEAD --`:

    M1  restore `_tripwire_compat && ` before `total_drift > 0`
        → 3 of 5 red (the regression, --json, tripwire-no-op); control green
    M2  let --tripwire decorate the error message (" [tripwire]")
        → SURVIVED the first suite: the no-op case compared stdout only and the
          verdict line is on stderr. The test now compares stderr too
          (46c1e7cb); re-run: 1 of 5 red, the no-op case. A mutation that
          survives is a test that was not asserting what its name says.
    M3  reject unconditionally (`Err` when total_drift == 0 too)
        → 1 of 5 red, the converged control
