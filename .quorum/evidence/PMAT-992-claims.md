# PMAT-992 (`forjar#993`) — Extracted Claim Manifest (`C1..C5`, `R1..R2`)

Base commit: `c4ce3a2c5a4d770a79d4dfdcb6295cd438a9e7da`
Head commit: `098e3709c1ca31d162c7af9917d8ad5fc7571fa5`
Branch: `fix/992-timeout-kill-by-syscall`

1. **C1 (`[code]`)**: `kill_process_group` (`src/transport/child_group.rs:55`, `Cargo.toml:141`) replaces external `Command::new("kill")` execution with `nix::sys::signal::killpg` (`Signal::SIGKILL`), converting `u32 -> pid_t` via `i32::try_from(pid)` gated on `raw > 0` so PGID `0` and out-of-range values are refused without `unsafe` code.
2. **C2 (`[code]`)**: `kill_pid` (`src/transport/child_group.rs:75`, `src/transport/pepita.rs:183`, `src/transport/mod.rs:35`) replaces `pepita::cleanup_namespace`'s external `kill -9` spawn with `pid_str.trim().parse::<u32>()` plus `kill(Pid::from_raw(raw), Signal::SIGKILL)` gated on `raw > 1`, refusing PID `0`, PID `1` (`init`), negative strings, and non-numeric pidfile contents.
3. **C3 (`[test]`)**: `tests/falsification_992_timeout_needs_no_kill_binary.rs:25` constructs a hermetic `PATH` directory omitting `kill` while retaining `bash` and `sleep`, proving `forjar apply --resource-timeout 2` terminates a `sleep 30` local task in `< 15s` (`2.44s` measured on `yoga`) and walking `src/` to verify zero remaining `Command::new("kill")` call sites.
4. **C4 (`[test]`)**: `src/transport/tests_timeout.rs:15` and `src/transport/tests_dispatch.rs:491` pin direct `SIGKILL` group delivery (`status.signal() == Some(9)`) and refusal of `0`, `1`, `2_000_000_000`, and `u32::MAX`.
5. **C5 (`[release]`)**: `Cargo.toml:3`, `Cargo.lock:1109`, `CHANGELOG.md:10`, `docs/roadmaps/releases.yaml`, and `docs/roadmaps/roadmap.yaml` bump `forjar` to `1.34.1`, book `v1.34.0`, declare `next.tag: v1.34.1`, and reconcile all `11` PRs merged on `main` since `v1.34.0` (including `#930` `PMAT-920` and `#978` `PMAT-913` synced at `098e3709`) alongside `PMAT-992`.
6. **R1 (`[doc]`)**: `docs/audits/dogfood-1.34.1-receipt.md` records `verdict: NO-GO on the nine-gate bar` (`A, C, D, E` PASS; `B, F, G, T, H` FAIL; `crux-1.34.1.md` omitted rather than fabricated), so merging `#993` stages `1.34.1` on `main` (`Cargo.toml:3`) without claiming nine-gate promotion clearance.
7. **R2 (`[scope]`)**: `tests/falsification_992_timeout_needs_no_kill_binary.rs:85` exercises local shell apply under a `kill`-less `PATH` rather than driving remote SSH/container timeout or `pepita::cleanup_namespace` (`src/transport/pepita.rs:183`) end-to-end.
