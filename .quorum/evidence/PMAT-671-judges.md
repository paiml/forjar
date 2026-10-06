# Judges — forjar#671: the nightly aarch64 legs can run the cross they install

Lanes and their findings: see `PMAT-671-lanes.md`.

## CONFIRMED

1. [path] C1 — nightly.yml's "Install cross (aarch64 legs only)" step appends `$CARGO_HOME/bin` to `$GITHUB_PATH` after `cargo install cross --locked`, the same line release.yml carries for #611.
   - evidence: `.github/workflows/nightly.yml:138` beside `.github/workflows/release.yml:275`; the per-target assertion at `tests/falsification_611_cross_on_path.rs:152` is green on the branch for both aarch64 targets, and all three lanes confirmed it.
2. [discovery] C2 — The falsifier discovers every step of every workflow whose `run:` contains `cargo install cross` instead of reading one step by name, and the discovery itself must find both the release.yml and the nightly.yml step.
   - evidence: the scan at `tests/falsification_611_cross_on_path.rs:54` and the two required labels at `tests/falsification_611_cross_on_path.rs:126` and `tests/falsification_611_cross_on_path.rs:130`; the sonnet lane grepped the workflows and found exactly those two sites.
3. [replay] C3 — For each discovered step and each aarch64 target the test runs the step's own script with a fake `cargo` that installs into `$CARGO_HOME/bin`, then replays `$GITHUB_PATH` last-written-first and asks the next step's shell for `cross`.
   - evidence: the fake cargo at `tests/falsification_611_cross_on_path.rs:85`, the no-cross precondition at `tests/falsification_611_cross_on_path.rs:106`, the replay at `tests/falsification_611_cross_on_path.rs:119`, and the gnu and musl tests at `tests/falsification_611_cross_on_path.rs:162` and `tests/falsification_611_cross_on_path.rs:167`.
4. [falsified] C4 — With nightly.yml reverted to main the gnu and musl tests fail by name and discovery stays green; restored, all three pass.
   - evidence: measured on t2build at af12be69: branch 3 passed; reverted, "1 passed; 2 failed", both panicking at `tests/falsification_611_cross_on_path.rs:152` (cross installed but off PATH); restored, 3 passed and the tree clean.

## REFUTED

1. [scope] R1 — The first #611 falsifier guarded every cross install in the repo, so a missing `$GITHUB_PATH` line in any workflow would have gone red.
   - corrected: it read one step of release.yml by name, so nightly.yml's identical defect stayed red in the nightly and green in the test; the discovery at `tests/falsification_611_cross_on_path.rs:54` and the required labels at `tests/falsification_611_cross_on_path.rs:126` close that gap.
