# PMAT-920 (`forjar#930`) — Judges Claims Digest

### CONFIRMED

1. [code] Atomic lock publication via staging write, fsync, and hard_link (`C1`)
   - evidence: `src/core/state/process_lock.rs:115` and `src/core/state/lock_info.rs:45` write full `LockInfo` metadata (`pid`, `id`, `host`, `user`, `started_at`, `cmd`) to a PID+ID staging file, sync to disk, and publish `.forjar.lock` atomically via `std::fs::hard_link` so concurrent readers never observe an empty or half-written lock file; verified by `25/25` `process_lock` tests on `t2fw13`.

2. [code] Full-span apply & plan-apply lock guard with dry-run exemption (`C2`)
   - evidence: `src/cli/apply_lock_flags.rs:29` holds `ProcessLockGuard` across pre-validate, execution, outputs, generation, and global lock on both `src/cli/apply.rs:127` and `src/cli/apply_from_plan.rs:348` while returning `Ok(None)` when `dry_run` is true; perturbing `src/cli/apply_lock_flags.rs:36` (`if dry_run` -> `if true`) failed `src/cli/tests_fj_lock_c2.rs:40` (`FJ930_REV_KILLED` on `t2fw13`).

3. [code] Owner-ID-checked lock release and live-PID force-unlock refusal (`C3`)
   - evidence: `src/core/state/lock_info.rs:150` and `src/core/state/tests_fj_lock_c3_c5.rs:60` verify that `ProcessLockGuard::drop` only unlinks `.forjar.lock` when the on-disk owner `id` matches the guard's `id`, and `force_unlock` / `--force-unlock <ID>` refuses live local holders (`tests/falsification_state_integrity.rs:325` updated to test reaping against dead PID `4294967290`).

4. [code] Sorted per-machine lock scoping and flattened CLI lock flags (`C4`)
   - evidence: `src/core/state/lock_scope.rs:25` and `src/core/executor/helpers.rs:265` derive sorted per-machine `.forjar.lock` targets to prevent AB-BA deadlocks across concurrent multi-host applies, while `src/cli/commands/apply_lock_args.rs:7` and `src/cli/tests_fj_lock_c3_c5.rs:15` wire `--lock-timeout` and `--force-unlock [ID]` cleanly through `prepare_state_lock`.

### REFUTED

1. [diff] Round 1 inline edits violated file-health and the CLI flag-consumer guard (`R1`)
   - evidence: Earlier commits on `#930` grew `src/cli/apply.rs:125` (`490 -> 501` lines) and `src/cli/commands/apply_args.rs:690` (`704 -> 711` lines) past the 500-line `file-health` ratchet, and flattening `ApplyLockArgs` initially hid `lock_timeout` and `force_unlock` from `every_declared_flag_is_consumed_or_refused`.
   - corrected: Head `a60316a8` extracts `src/cli/apply_lock_flags.rs:9` (`42` lines) and `src/cli/commands/apply_lock_args.rs:7` (`17` lines), restoring `src/cli/apply.rs` to `499` non-trailing lines and `src/cli/commands/apply_args.rs` to `704` lines with `&ApplyLockArgs` passed directly to `prepare_state_lock`.

2. [doc] Unconditional non-overtake FIFO claim in lock_wait.rs and process_lock.rs (`R2`)
   - evidence: `src/core/state/lock_wait.rs:4` and `src/core/state/process_lock.rs:65` state unconditionally that only the head `.q.` ticket may attempt acquisition so a late arrival cannot overtake queued waiters, whereas `src/core/state/process_lock.rs:45` only creates a ticket when `!timeout.is_zero()` and line `67` sets `my_turn = ticket.is_none_or(lock_wait::is_head)`.
   - corrected: Mutual exclusion holds strictly for all callers via `hard_link` `EEXIST`, and FIFO arrival ordering holds among `--lock-timeout > 0` waiters, but an unqueued zero-timeout caller (`ticket == None`) does not consult `.q.` tickets and can win acquisition at the instant of unlock ahead of a polling waiter.
