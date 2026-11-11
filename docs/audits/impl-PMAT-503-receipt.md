# Implementation receipt — PMAT-503 — `cmd_build_with_load_flag_no_runtime` depends on the host's docker/podman

verdict: PASS — the runtime probe is injected (`cmd_build_load_with`), so the "no runtime" tests state that case themselves and their verdict no longer depends on which runtime the host has; one probe in them still spawns `tar`/`podman` from PATH against a path that does not exist (see Not covered); production behaviour is unchanged

## Identity

- ticket PMAT-503 (forjar#503); kind: test seam (one production function split, no behaviour change)
- PR #948, branch `g3/503-load-test-hermetic`, change commit 762515fd
- files: `src/cli/build_distribution.rs` (+11 −2), `src/cli/tests_build_image.rs` (+48 −23), `docs/roadmaps/roadmap.yaml` (row `GH-503` renamed to `PMAT-503`)

## Root cause (five whys)

1. On a host with docker on PATH and `docker load` failing, the workspace suite stopped at `cmd_build_with_load_flag_no_runtime`.
2. The test's name says "no runtime", but it never arranged that: it assumed the host had none.
3. `cmd_build_load` asked the host directly (`which_runtime("docker")`, then `"podman"`), with no way for a caller to answer instead.
4. So the test's verdict was a property of the machine, not of the code.
5. The probe was a hard-wired call, not a parameter.

## Contract and falsifier

Contract: `cmd_build_load_with(oci_dir, has_runtime)` consults only `has_runtime` to choose docker, then podman, then the no-runtime path; `cmd_build_load(oci_dir)` is `cmd_build_load_with(oci_dir, which_runtime)`.

Falsifiers: the `cmd_build_load` tests in `cli::tests_build_image`, which pass the probe as a closure.

## Evidence (kfocus, `nice -n 19`, 2026-11-11, `cargo test --lib tests_build_image`)

GREEN at 762515fd (forced rebuild from this tree; kfocus has docker on PATH):

    test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 13741 filtered out

RED is a compile failure, not a failing assertion: with `src/cli/build_distribution.rs` taken from main, the new tests do not build.

    error[E0425]: cannot find function `cmd_build_load_with` in module `crate::cli::build_distribution`

## Not covered

- The host-dependent failure that #503 reports (docker present, `docker load` failing) was not reproduced for this receipt; the evidence is that the tests no longer read the host.
- The docker and podman branches still shell out for real; only the choice of branch is injected.
- Three-lane review, not fixed here: the second assertion of `cmd_build_with_load_flag_no_runtime` passes a probe that reports podman, so it runs real `tar` and `podman load` on `/nonexistent-oci-dir`; it passes on any host because that path fails either way, but it does spawn host binaries. The same test still writes a `forjar.yaml` it no longer uses. No test passes a probe that reports both runtimes, so "docker before podman" is stated in the contract and not pinned by a test.

- Three-lane review of PR #948 at c946388 (agy, Gemini 4 Carbon): 3/3 PASS; the refuted claims are listed under `quorum.refuted_claims` in the receipt. The receipt's verdict was narrowed afterwards; the lanes' test findings are listed under Not covered. Quorum receipt: `.quorum/g3-503-load-test-hermetic.json`, written by agy and committed unedited.

IMPL-PMAT-503-RECEIPT-END
