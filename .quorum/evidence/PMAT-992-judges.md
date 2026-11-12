# PMAT-992 (`forjar#993`) — Judges Claims Digest

### CONFIRMED

1. [code] Syscall process-group SIGKILL with safe pid_t bounds (`C1`)
   - evidence: `src/transport/child_group.rs:55` and `Cargo.toml:141` replace `Command::new("kill")` with `nix::sys::signal::killpg` (`Signal::SIGKILL`) behind `i32::try_from(pid)` and `raw > 0`, eliminating the external `procps` runtime dependency on `rust:1.95-slim` while keeping `#![forbid(unsafe_code)]` intact and adding zero new packages to `Cargo.lock:10`.

2. [code] Pepita namespace teardown via validated kill_pid syscall (`C2`)
   - evidence: `src/transport/pepita.rs:183` and `src/transport/mod.rs:35` replace unvalidated `Command::new("kill").args(["-9", pid])` with `pid_str.trim().parse::<u32>()` and `kill_pid` (`raw > 1`), refusing PID `0`, PID `1` (`init`), negative numbers, and corrupt pidfile flags without spawning an external binary.

3. [test] End-to-end procps-less PATH timeout falsifier and AST scan (`C3`)
   - evidence: `tests/falsification_992_timeout_needs_no_kill_binary.rs:25` builds a hermetic `PATH` symlink directory excluding `kill` while keeping `bash` and `sleep`, verifying `forjar apply --resource-timeout 2` aborts a `sleep 30` task in `2.31s` (`< 15s`) and scanning `src/` to confirm zero `Command::new("kill")` call sites remain.

4. [test] Unit boundary guards for PGID and special PID refusal (`C4`)
   - evidence: `src/transport/tests_timeout.rs:15` and `src/transport/tests_dispatch.rs:491` verify `kill_process_group` delivers `SIGKILL` (`status.signal() == Some(9)`) to a live child group while returning `false` for `0`, `2_000_000_000`, and `u32::MAX`, and verify `kill_pid` refuses `0` and `1`.

5. [release] Version 1.34.1 bump, release ledger booking, and 9-PR window changelog (`C5`)
   - evidence: `Cargo.toml:3`, `Cargo.lock:1109`, and `CHANGELOG.md:10` bump `forjar` to `1.34.1` and reconcile `PMAT-992` plus all 9 PRs merged on `main` since `v1.34.0`, passing all 9 release and timeout falsification targets (`53/53` tests) at `236018b2`.

### REFUTED

1. [doc] Dogfood 1.34.1 receipt records nine-gate NO-GO rather than tag promotion clearance (`R1`)
   - evidence: While `Cargo.toml:3` and `CHANGELOG.md:10` prepare the `1.34.1` cut, `docs/audits/dogfood-1.34.1-receipt.md` measures `5` of `9` dogfood gates failing (`B, F, G, T, H`, with `crux-1.34.1.md` intentionally absent) and prints `verdict: NO-GO on the nine-gate bar`.
   - corrected: All 3 lanes verified that no gate script, budget, or floor was weakened, all release-shape falsifiers (`53/53`) pass on `<host>` (`<SCRATCH>`), and `NO-GO` is explicitly disclosed so merging `#993` does not masquerade as nine-gate tag promotion approval.

2. [scope] Integration falsifier covers local transport only, not SSH or pepita end-to-end (`R2`)
   - evidence: `tests/falsification_992_timeout_needs_no_kill_binary.rs:85` invokes `forjar apply` on a local shell task under `PATH` without `kill`, without running an SSH/container timeout or `pepita::cleanup_namespace` (`src/transport/pepita.rs:183`) integration harness.
   - corrected: SSH and container worker timeouts traverse the identical `kill_worker_child_group -> kill_process_group` path (`src/transport/child_group.rs:75`) against the local client process group, and `kill_pid`'s `0`/`1`/`u32::MAX` guards are directly unit-tested in `src/transport/tests_timeout.rs:33`.
