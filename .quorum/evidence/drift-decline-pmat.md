# PMAT-564 — instruments, and what each said

    cargo test --workspace --no-fail-fast   340 targets at bd1664a8: 19,699
                                            passed, 88 failed — every one a
                                            doctest, all with `can't find crate
                                            for provable_contracts_macros`; a
                                            parallel `cargo check` in another
                                            worktree shared the target dir
                                            (the wrapper-vs-script split, known).
                                            `cargo test --doc` alone: 88 passed
    cargo clippy --all-targets -D warnings   clean
    cargo fmt --all -- --check               clean
    bash scripts/dogfood/surface.sh          GATE C PASS
    bash scripts/dogfood/docs.sh             GATE D PASS
    bash scripts/dogfood/contracts.sh        GATE G PASS
    bash scripts/dogfood/comply.sh           GATE B FAIL — main's CB-21xx debt
                                            (CB-2112 36>35, CB-2115 49>43), not
                                            worse here; CB-2114 at 34 with the
                                            row's release: and milestone set
    pv validate contracts/drift-declines-on-empty-scope-v1.yaml   Contract is valid.
    the RED proof                            src/ reverted with the test file
                                            in place: 5 of 6 cases red, the
                                            fully-inspected control green

Mutations over the COMMITTED tree, each restored with `git checkout HEAD --`,
`git status --porcelain` empty after each:

    M1  delete the `decline_on_empty_scope(cfg, &scan)?` call
        → 2 of 6 red: the regression and the --json case
    M2  pass declared_only = false from detect_drift_full_reported
        → 5 of 6 red: every case but the fully-inspected control (a.yaml's
          files are graded under b.yaml, and the census contradicts itself)
    M3  drop the `scan.total_unmeasured > 0` guard (the first cut WAS this)
        → 2 of 4 red in falsification_drift_unmeasured_is_not_drift: exit 2
          where forjar#549 requires 4
    M4  decline whenever declared > 0 (ignore inspected)
        → 2 of 6 red: the partially-locked control and the applied manifest
    M5  drop the image detector's `resources.contains_key(id)` guard
        → 1 of 8 red in tripwire::drift::tests_image_drift: the undeclared
          image is inspected
