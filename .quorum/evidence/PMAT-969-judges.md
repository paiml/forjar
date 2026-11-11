# Judges — ChildPidSlot CANCELLED_PID pre-spawn timeout race fix (PMAT-969 / forjar#932)

Round count and heads: see `PMAT-969-lanes.md`. Every claim met 3 independent refuters across Correctness, Failure Modes, and Contract/Falsifiers on `t2fw13`.

## CONFIRMED

1. [diff] C1 — `kill_worker_child_group` (`src/transport/child_group.rs:84`) and `record_child_pid` (`src/transport/child_group.rs:18`) serialize on `slot.lock()`, writing `CANCELLED_PID` on `None` so a late-spawning worker kills its child process group immediately.
   - evidence: `7/7` tests in `src/transport/tests_timeout.rs:95` passed in `0.32s` on `t2fw13`, and reverting the `CANCELLED_PID` branch in `src/transport/child_group.rs:20` left the `sleep 30` grandchild alive and failed `f20_i6_timeout_before_spawn_still_kills_late_child` (`FJ932_REV_KILLED`).
2. [diff] C2 — `CANCELLED_PID = u32::MAX` (`src/transport/child_group.rs:11`) is disjoint from valid Linux PIDs (`<= 4,194,304`), `kill_worker_child_group` guards `pid != CANCELLED_PID` before casting `pid as i32`, and `process_group(0)` (`src/transport/child_group.rs:38`) establishes PGID `== child.id()` before `spawn()` returns.
   - evidence: audited `src/transport/child_group.rs:11` through `src/transport/child_group.rs:96`, confirming zero risk of `kill(-1, SIGKILL)` (`u32::MAX as i32 == -1`) and poison-safe `unwrap_or_else(|e| e.into_inner())` recovery on both lock sites.
3. [diff] C3 — Splitting `src/transport/child_group.rs:1` (`96` lines) out of `src/transport/mod.rs:41` (`424` lines) preserves `pub(crate)` re-exports across all 5 transport backends while keeping `src/transport/tests_timeout.rs:95` (`118` lines at head) modular.
   - evidence: `wc -l` on `t2fw13` confirmed all three files sit well below `500` lines (`424`, `96`, `118`), and `cargo clippy --lib --tests -- -D warnings` passed with zero warnings.
4. [diff] C4 — `cargo fmt --check`, GitHub CI rollup (`SUCCESS` at `e5279139`), `docs/roadmaps/roadmap.yaml`, and `docs/audits/impl-PMAT-969-receipt.md` satisfy Gate A and clean-room checks.
   - evidence: `t2fw13` verified `FJ932_FMT_OK` and `FJ932_CLIPPY_OK`, `docs/roadmaps/roadmap.yaml` registers `PMAT-969`, and `docs/audits/impl-PMAT-969-receipt.md` ends with `IMPL-PMAT-969-RECEIPT-END`.

## REFUTED

1. [diff] R1 — Round 1 added the `CANCELLED_PID` logic inline inside `src/transport/mod.rs:41`, growing `mod.rs` from `497` to `513` lines and failing the 500-line `file-health` gate.
   - corrected: refuted during Round 1 verification and resolved at `e5279139` by extracting `src/transport/child_group.rs:1` (`96` lines), reducing `src/transport/mod.rs:41` to `424` lines.
2. [diff] R2 — Round 1 failed `cargo fmt --check` on `#[cfg(test)]` import ordering in `src/transport/mod.rs:43` and recorded that the post-`rustfmt` tree had not been re-compiled on `kfocus`.
   - corrected: `e5279139` fixed import ordering (`FJ932_FMT_OK`) and `agy` compiled and ran the post-`rustfmt` tree from scratch on `t2fw13` (`FJ932_CLIPPY_OK`, `7/7` `tests_timeout` green).
