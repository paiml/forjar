# PMAT-503 (`forjar#948`) — Judges Claims Digest

### CONFIRMED

1. [code] Behavior-preserving cmd_build_load_with runtime-probe seam (`C1`)
   - evidence: `src/cli/build_distribution.rs:8` forwards `cmd_build_load(oci_dir)` directly to `cmd_build_load_with(oci_dir, super::dispatch_misc_b::which_runtime)` (`src/cli/build_distribution.rs:14`), keeping runtime selection and `tar | <runtime> load` execution unchanged in production.

2. [test] Hermetic no-runtime refusal test on docker-installed daemonless hosts (`C2`)
   - evidence: `src/cli/tests_build_image.rs:375` calls `cmd_build_load_with(dir.path(), |_| false)` and asserts `"docker or podman"` without mutating process-global `PATH`; perturbing `has_runtime("docker")` to `true` on `t2fw13` failed the test (`FJ948_REV_KILLED`).

3. [code] Formatting, clippy, and file-health compliance (`C3`)
   - evidence: `src/cli/build_distribution.rs` (`203` lines, base `194`) and `src/cli/tests_build_image.rs` (`472` lines, base `447`) pass `cargo fmt --check` and `cargo clippy --lib --tests -- -D warnings` cleanly on `t2fw13` under the 500-line ceiling.

### REFUTED

1. [doc] Receipt claim that tests never touch host PATH (`R1`)
   - evidence: `docs/audits/impl-PMAT-503-receipt.md` line `3` states `"the tests... no longer depend on what is on the host's PATH"`, whereas `src/cli/tests_build_image.rs:379` (`|n| n == "podman"`) enters the runtime branch (`build_distribution.rs:27`) and spawns `tar` and `podman` from `PATH`.
   - corrected: Only the `"docker or podman"` refusal branch (`|_| false`) is completely isolated from `PATH`; the second check passes across hosts because `/nonexistent-oci-dir` causes `tar`/`podman` to fail with an I/O or exit-status error rather than `"docker or podman"`.

2. [test] Unused fixture file and unpinned docker-over-podman tie-break (`R2`)
   - evidence: `src/cli/tests_build_image.rs:366` still writes `forjar.yaml` and discards it via `let _ = &path;`, and no test invokes `cmd_build_load_with(..., |_| true)` to pin `impl-PMAT-503-receipt.md` line `21` (`"chooses docker, then podman"`).
   - corrected: Both are harmless test-hygiene residuals; the `#503` failure mode on daemonless `docker` builders (`kfocus`, `t2fw13`) is resolved and falsified by `FJ948_REV_KILLED`.
