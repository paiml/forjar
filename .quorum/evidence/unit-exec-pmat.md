# PMAT-560 — instruments, and what each said

    cargo test --workspace --no-fail-fast   at 8337cab5 (the GREEN commit):
                                            339 targets, 19,801 passed, 0 failed.
                                            The first run failed one case,
                                            planner::tests_hash's golden
                                            desired-state hash: two new Resource
                                            fields move every recorded hash, the
                                            #403/#406 fleet migration — repinned
                                            with an upgrade note, not waved through.
                                            At a9fd3024, the final code head:
                                            339 targets, 19,805 passed, 0 failed
    cargo clippy --all-targets -D warnings   clean
    cargo fmt --all -- --check               clean
    bashrs lint (the emitted apply script)   0 errors after the probe used brace
                                            groups and a variable name without
                                            `exec` (SC2031 and SEC016 fired on the
                                            first cut)
    bash scripts/dogfood/contracts.sh        GATE G PASS (41 contracts)
    pv validate contracts/forjar-unit-exec-parity-v1.yaml   Contract is valid.
    the RED proof                            8 of 10 cases red on 1.30.0's
                                            service.rs, the fixture-fidelity
                                            case and the undeclared control green
    systemctl show -p ExecStart --value ssh.service   the real systemd 249 line
                                            the fake host reproduces

Mutations over the COMMITTED tree at a9fd3024, fourteen cases, each restored
with `git checkout HEAD --`, `git status --porcelain -- src tests` empty after:

    MA  check_script stops extending with the exec assertions
        → 9 of 14 red; the five survivors are the undeclared control, the
          state-query case, the apply case, the fixture-fidelity case and the
          space-in-path case whose check exits 0 either way
    MB  re-hash the DECLARED exec_start before the digest assertion
        → 2 red: the live-program digest case and the stderr case
    MC  emit the probe for an undeclared service
        → 1 red: the undeclared control
    MD  drop the apply tail
        → 2 red: the apply case and the stderr case
    ME  split the systemctl line on a bare space again
        → 2 red: the space-in-path case and the run.sh-evil exit-0 case
    MF  the divergence marker to stdout only
        → 1 red: the stderr case
