# Claims — forjar 1.33.0 release PR, rc.1 promoted + #611 (PMAT-652)

Counts and heads live in `PMAT-652-lanes.md`.

- **C1** release.yml gains exactly one line, `echo "$CARGO_HOME/bin" >> "$GITHUB_PATH"`, inside the aarch64 branch of "Install target prerequisites (Linux)", after the `cargo install cross` block; nothing else in any workflow changes
- **C2** tests/falsification_611_cross_on_path.rs RUNS that step's own script with a fake cargo that installs cross into $CARGO_HOME/bin, replays GITHUB_PATH into the next step's PATH, and asserts cross resolves, for the gnu and musl aarch64 legs
- **C3** the test is RED without the line and GREEN with it
- **C4** the rest of the diff is the 1.33.0 version bump (Cargo.toml, Cargo.lock), its CHANGELOG section and the PMAT-652 roadmap row; rc.1 code is promoted unchanged

## The measured symptom

forjar#611: build-binaries sets a private CARGO_HOME (infra#430). On a runner
without cross, `cargo install cross` landed in $CARGO_HOME/bin, off PATH, and
the build step died with `cross: command not found`. Release run 36415517563
(v1.33.0-rc.1) failed both aarch64 linux legs twice on yoga-build for this
reason; the release stayed a draft with 0 assets, as v1.32.0 did.
