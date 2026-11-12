# PMAT-920 (`forjar#930`) — Extracted Claim Manifest (`C1..C4`, `R1..R2`)

Base commit: `5b8f8123be40731340e2a74a6e4ae365648cab45`
Head commit: `a60316a82ff1d6e653cd6d17be03514f9f8b07e3`
Branch: `forjar-cop/fj-lock-001-c1c2`

1. **C1 (`[code]`)**: `.forjar.lock` is published atomically via tempfile write + `sync_all` + `hard_link` in `src/core/state/process_lock.rs` (`LockInfo` in `src/core/state/lock_info.rs`), eliminating empty/half-written lock files.
2. **C2 (`[code]`)**: `hold_apply_locks` (`src/cli/apply_lock_flags.rs:29`) holds per-machine locks across pre-validate gates, execution, outputs, generation, and global lock on both `cmd_apply_scoped` (`src/cli/apply.rs:127`) and `cmd_apply_from_plan` (`src/cli/apply_from_plan.rs:348`), and takes no lock when `dry_run` is true.
3. **C3 (`[code]`)**: Lock release checks owner `id` before unlinking, and `force_unlock` / `--force-unlock <ID>` refuses to remove a lock held by a live local PID (`src/core/state/lock_info.rs:150`, `tests/falsification_state_integrity.rs:325`).
4. **C4 (`[code]`)**: `lock_scope.rs` and `executor::lock_targets` (`src/core/executor/helpers.rs:265`) scope state locks per targeted machine in deterministic sorted order while exposing `--lock-timeout <DUR>` and `--force-unlock [ID]` via `ApplyLockArgs` (`src/cli/commands/apply_lock_args.rs:7`).
5. **R1 (`[diff]`, Round 1 regression)**: Round 1 grew `src/cli/apply.rs` (`501`) and `src/cli/commands/apply_args.rs` (`711`) past `file-health` and broke `every_declared_flag_is_consumed_or_refused`; refuted and fixed at `a60316a8`.
6. **R2 (`[doc]`, FIFO fairness scope)**: `src/core/state/lock_wait.rs:4` and `src/core/state/process_lock.rs:65` state unconditionally that a late arrival cannot overtake queued waiters, whereas zero-timeout callers (`timeout.is_zero()`) bypass `.q.` ticket enqueue (`src/core/state/process_lock.rs:45`) and can acquire on unlock ahead of polling waiters.
