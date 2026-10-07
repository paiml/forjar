# Judges — forjar#683: cross mounts the sysroot from the docker host

Lanes and their findings: see `PMAT-683-lanes.md`. Base workflow line numbers are at 61d5a703; release.yml lines for C5 are at the head.

## CONFIRMED

1. [cause] C1 — The aarch64 legs build with cross through the host daemon, with a private CARGO_HOME under the workspace and no RUSTUP_HOME, so the sysroot came from the runner image's default.
   - evidence: `.github/workflows/nightly.yml:103` sets only CARGO_HOME and `.github/workflows/nightly.yml:153` runs `cross build`. Job 112435767529 (a containerized runner) pulled the cross image and printed `sh: 1: cargo: not found`, exit 127. Every aarch64 leg of runs 36315352379, 36995023273, 37301751196, 37454046448 and 37511323051 on a containerized runner failed, and every one on a native runner passed. The mechanism is recorded at `tests/falsification_683_cross_mounts_under_work_root.rs:11`.
2. [fix] C2 — The same RUSTUP_HOME line sits beside CARGO_HOME in all three cross jobs.
   - evidence: base CARGO_HOME lines at `.github/workflows/nightly.yml:103`, `.github/workflows/release.yml:228` and `.github/workflows/binary-release.yml:128`; the new line follows each. `tests/falsification_683_cross_mounts_under_work_root.rs:166` asserts it for every discovered job.
3. [test] C3 — The test discovers jobs, not named steps, and resolves the env per layer (workflow, job, step); a missing RUSTUP_HOME resolves to `$HOME/.rustup`, as rustup does.
   - evidence: discovery at `tests/falsification_683_cross_mounts_under_work_root.rs:58`, resolution at `tests/falsification_683_cross_mounts_under_work_root.rs:125`, the discovery floor at `tests/falsification_683_cross_mounts_under_work_root.rs:151`, and the fixture at `tests/falsification_683_cross_mounts_under_work_root.rs:178`.
4. [falsified] C4 — Reverting the three workflows turns the path test red, naming all three jobs.
   - evidence: measured locally at 1d3e2d45: workflows at 61d5a703 gave "2 passed; 1 failed", listing binary-release.yml:build, nightly.yml:build and release.yml:build-binaries with RUSTUP_HOME = the runner image's `$HOME/.rustup`; restored, 3 passed. The failing assertion is `tests/falsification_683_cross_mounts_under_work_root.rs:166`.
5. [fix] C5 — release.yml's Linux legs install a toolchain into the empty per-job RUSTUP_HOME before their first rustup or cargo.
   - evidence: `.github/workflows/release.yml:269` (dtolnay/rust-toolchain with the leg's target, `if: contains(matrix.target, 'linux')`) and `.github/workflows/release.yml:275` (`rustup target add` on the pinned channel), both before "Install target prerequisites (Linux)" and `cargo metadata` at `.github/workflows/release.yml:303`. The macOS install is unchanged. Without them the Linux legs reached `rustup target add`, `cargo install cross` and `cargo metadata` with no toolchain: round 3's finding.
6. [falsified] C6 — The per-leg check fails on the previous release.yml and passes on this one.
   - evidence: `tests/falsification_683_cross_mounts_under_work_root.rs:277` walks each leg's steps, `tests/falsification_683_cross_mounts_under_work_root.rs:215` evaluates the `if:` forms these workflows use, and `tests/falsification_683_cross_mounts_under_work_root.rs:330` floors the job count at three. Measured locally at a86201fa: with release.yml at 1d3e2d45, "4 passed; 1 failed", naming x86_64-gnu, x86_64-musl, aarch64-gnu and aarch64-musl at step "Install target prerequisites (Linux)"; restored, 5 passed. The fixture at `tests/falsification_683_cross_mounts_under_work_root.rs:341` is caught.

## REFUTED

1. [lead] R1 — The brief's framing: the aarch64-gnu target is what is broken.
   - corrected: the failure follows the runner, not the target. In run 37511323051, aarch64-gnu failed on a containerized runner while aarch64-musl passed on a native runner. aarch64-gnu passed on native runners (36315352379, 36995023273, 37454046448) and failed on a containerized one (37301751196).
2. [lead] R2 — #611/#671 again: cross is off PATH.
   - corrected: job 112435767529 resolved `cross`, which pulled `ghcr.io/cross-rs/aarch64-unknown-linux-gnu:0.2.5`; the exit 127 is `sh: 1: cargo: not found` from inside cross's container, after the PATH fix had done its job.
3. [lane] R3 — Round 4: the toolchain install is outside what the ticket asked for.
   - corrected: it is required by the ticket's own change. RUSTUP_HOME under the work root is a new, empty directory, so without the install the release job's Linux legs have no toolchain at all (C5). The ticket's acceptance criteria now name it.
