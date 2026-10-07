# Claims — forjar#683: cross mounts the sysroot from the docker host

Briefed to every lane with the full diff against main 46ee342d. C1–C4 were
in every round; C5 and C6 were added after round 3 found the regression
they answer.

- C1: on a containerized runner, which shares only its work root with the host at the same path, cross's `/rust` sysroot mount comes from the image-layer RUSTUP_HOME (the image default, `$HOME/.rustup`), which the host daemon cannot see, so it mounts an empty directory and the build dies `sh: 1: cargo: not found`.
- C2: the three jobs that run `cross build` (nightly.yml:build, release.yml:build-binaries, binary-release.yml:build) now set RUSTUP_HOME under `${{ github.workspace }}/..`, beside CARGO_HOME, so every path cross mounts is under the work root on every runner.
- C3: `falsification_683_cross_mounts_under_work_root` discovers every job whose steps run `cross build`, asserts all three known ones are found, resolves the env that step sees (unset RUSTUP_HOME = $HOME/.rustup) and fails naming each mounted path outside the work root; a fixture proves `..` cannot walk a path out unnoticed.
- C4: with the three workflows at 61d5a703 the test fails, naming all three jobs; ci.yml runs it.
- C5: a per-job RUSTUP_HOME is a fresh, empty directory, so a job that sets it must install a toolchain before its first cargo, rustup or cross. nightly.yml:build and binary-release.yml:build already did (dtolnay/rust-toolchain), and so did release.yml:build-binaries on macOS; its four Linux legs did not, and now install one, then add the target to the channel `rust-toolchain.toml` pins.
- C6: the falsifier checks C5 per matrix leg, for every job whose env sets RUSTUP_HOME. With release.yml at 1d3e2d45 it fails, naming all four Linux legs; a fixture with only a macOS install must be caught.
