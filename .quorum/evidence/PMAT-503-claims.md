# PMAT-503 (`forjar#948`) — Extracted Claim Manifest (`C1..C3`, `R1..R2`)

Base commit: `5b8f8123be40731340e2a74a6e4ae365648cab45`
Head commit: `c9463886644fe380a311f66a6ea5a1bb9444af8a`
Branch: `g3/503-load-test-hermetic`

1. **C1 (`[code]`)**: `cmd_build_load` (`src/cli/build_distribution.rs:8`) delegates to `cmd_build_load_with(oci_dir, super::dispatch_misc_b::which_runtime)` (`line 14`), preserving identical production behavior.
2. **C2 (`[test]`)**: `cmd_build_with_load_flag_no_runtime` (`src/cli/tests_build_image.rs:364`) injects `|_| false` to test the `"--load requires docker or podman"` refusal hermetically on hosts where `docker` is on `PATH` without a daemon (`FJ948_REV_KILLED` on `t2fw13`).
3. **C3 (`[code]`)**: `build_distribution.rs` (`203` lines) and `tests_build_image.rs` (`472` lines) pass `cargo fmt --check`, `cargo clippy`, and `file-health`.
4. **R1 (`[doc]`)**: `docs/audits/impl-PMAT-503-receipt.md:3` claims the tests no longer touch host `PATH`, whereas the second check (`tests_build_image.rs:379`) still spawns `tar` and `podman load` from `PATH`.
5. **R2 (`[test]`)**: `cmd_build_with_load_flag_no_runtime` leaves an unused `forjar.yaml` write (`let _ = &path;`) and does not pin `docker`-before-`podman` precedence when both return `true`.
