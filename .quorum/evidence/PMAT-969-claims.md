# Claims — ChildPidSlot CANCELLED_PID pre-spawn timeout race fix (PMAT-969 / forjar#932)

1. C1: Mutual exclusion on `ChildPidSlot` (`src/transport/child_group.rs:18`, `src/transport/child_group.rs:84`) totally orders `kill_worker_child_group` vs `record_child_pid` so late-spawned children are killed immediately.
2. C2: `configure_process_group` (`src/transport/child_group.rs:34`) sets `process_group(0)` in `pre_exec` so PGID `== child.id()` exists before `record_child_pid` invokes `kill -9 -- -<pid>`, and `CANCELLED_PID = u32::MAX` (`src/transport/child_group.rs:11`) cannot collide with any valid Linux PID (`PID_MAX_LIMIT = 2^22`).
3. C3: Extracting `src/transport/child_group.rs:1` (`96` lines) from `src/transport/mod.rs:41` (`424` lines) preserves module visibility and passes `7/7` `src/transport/tests_timeout.rs:95` tests on `t2fw13`.
4. C4: Formatting, clippy, GitHub CI rollup (`SUCCESS`), and `PMAT-969` Gate A traceability pass cleanly.
5. R1: Round 1 inline edit in `src/transport/mod.rs` exceeded the 500-line `file-health` limit (`513` lines).
6. R2: Round 1 failed `cargo fmt --check` on `src/transport/mod.rs` import ordering and left the post-fmt tree uncompiled on `kfocus`.
