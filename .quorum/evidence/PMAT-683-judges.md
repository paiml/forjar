# Judges — forjar#683: cross mounts the sysroot from the docker host

Lanes and their findings: see `PMAT-683-lanes.md`. Workflow line numbers are at base 61d5a703.

## CONFIRMED

1. [cause] C1 — The aarch64 legs build with cross through the host daemon, with a private CARGO_HOME under the workspace and no RUSTUP_HOME, so the sysroot came from the runner image's default.
   - evidence: `.github/workflows/nightly.yml:103` sets only CARGO_HOME and `.github/workflows/nightly.yml:153` runs `cross build`. Job 112435767529 (yoga-build3) pulled the cross image and printed `sh: 1: cargo: not found`, exit 127. Every aarch64 leg of runs 36315352379, 36995023273, 37301751196, 37454046448 and 37511323051 on a yoga-build* runner failed, and every one on an intel-clean-room-* runner passed. The mechanism is recorded at `tests/falsification_683_cross_mounts_under_work_root.rs:11`.
2. [fix] C2 — The same RUSTUP_HOME line sits beside CARGO_HOME in all three cross jobs.
   - evidence: base CARGO_HOME lines at `.github/workflows/nightly.yml:103`, `.github/workflows/release.yml:228` and `.github/workflows/binary-release.yml:128`; the new line follows each. `tests/falsification_683_cross_mounts_under_work_root.rs:166` asserts it for every discovered job.
3. [test] C3 — The test discovers jobs, not named steps, and resolves the env per layer (workflow, job, step); a missing RUSTUP_HOME resolves to `$HOME/.rustup`, as rustup does.
   - evidence: discovery at `tests/falsification_683_cross_mounts_under_work_root.rs:58`, resolution at `tests/falsification_683_cross_mounts_under_work_root.rs:125`, the discovery floor at `tests/falsification_683_cross_mounts_under_work_root.rs:151`, and the fixture at `tests/falsification_683_cross_mounts_under_work_root.rs:178`.
4. [falsified] C4 — Reverting the three workflows turns the test red, naming all three jobs.
   - evidence: measured on t2build: workflows at 61d5a703 gave "2 passed; 1 failed", listing binary-release.yml:build, nightly.yml:build and release.yml:build-binaries with RUSTUP_HOME = the runner image's `$HOME/.rustup`; restored, 3 passed. The failing assertion is `tests/falsification_683_cross_mounts_under_work_root.rs:166`.

## REFUTED

1. [lead] R1 — The brief's framing: the aarch64-gnu target is what is broken.
   - corrected: the failure follows the runner, not the target. In run 37511323051, aarch64-gnu failed on yoga-build3 while aarch64-musl passed on intel-clean-room-13. aarch64-gnu passed on intel-clean-room-11 (36315352379) and intel-clean-room-13 (36995023273, 37454046448), and failed on yoga-build (37301751196).
2. [lead] R2 — #611/#671 again: cross is off PATH.
   - corrected: job 112435767529 resolved `cross`, which pulled `ghcr.io/cross-rs/aarch64-unknown-linux-gnu:0.2.5`; the exit 127 is `sh: 1: cargo: not found` from inside cross's container, after the PATH fix had done its job.
