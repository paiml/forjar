# PMAT-565 — instruments, and what each said

    cargo test --workspace --no-fail-fast   341 targets at a0070731: 19,795
                                            passed, 0 failed (run alone; the
                                            shared target dir was quiet)
    cargo clippy --all-targets -D warnings   clean, before and after the
                                            repair/migrate change
    cargo fmt --all -- --check               clean
    cargo check --all-targets                clean over the 137 + 3 literal
                                            patches (a misplaced created_by
                                            would not compile)
    pv validate contracts/lock-names-its-writer-v1.yaml   Contract is valid.
    the RED proof                            the suite compiled against the
                                            new field with no behaviour:
                                            5 of 5 red (four on the stale
                                            generator, one on the missing
                                            --restamp flag)

Mutations over the COMMITTED tree, each restored with `git checkout HEAD --`:

    M1  serialise `lock` instead of `stamped_for_write(lock)` in save_lock
        → 6 of 6 red, every case in the suite (an earlier draft of this
          file said 5 of 6 before the mutation had been run; it was run
          and the number is the measurement)
    M2  drop the `created_by.is_none()` guard (roll the creator every write)
        → the set-once case red
    M3  write on --dry-run in lock_restamp
        → the restamp case red (dry-run must write nothing)
    M4  restore the bare fs::write in lock_repair (minimal lock)
        → exactly the repair case red

Gate B: FAIL on main's CB-21xx debt, equal or better here (the PMAT-565 row
carries release: 1.31.0 and its issue is on the milestone). Gates C, D, G:
run on the 564 head this branch stacks on; G re-run here after the contract
stopped using a word the ratchet reads as governing a verb.
