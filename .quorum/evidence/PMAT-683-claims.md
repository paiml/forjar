# Claims — forjar#683: cross mounts the sysroot from the docker host

Briefed to every lane at head d2210701 (one commit on main 61d5a703).

- C1: on a containerized runner, which shares only its work root with the host at the same path, cross's `/rust` sysroot mount comes from the image-layer RUSTUP_HOME (/home/runner/.rustup), which the host daemon cannot see, so it mounts an empty directory and the build dies `sh: 1: cargo: not found`.
- C2: the three jobs that run `cross build` (nightly.yml:build, release.yml:build-binaries, binary-release.yml:build) now set RUSTUP_HOME under `${{ github.workspace }}/..`, beside CARGO_HOME, so every path cross mounts is under the work root on every runner.
- C3: `falsification_683_cross_mounts_under_work_root` discovers every job whose steps run `cross build`, asserts all three known ones are found, resolves the env that step sees (unset RUSTUP_HOME = $HOME/.rustup) and fails naming each mounted path outside the work root; a fixture proves `..` cannot walk a path out unnoticed.
- C4: with the three workflows at 61d5a703 the test fails, naming all three jobs; ci.yml runs it.
