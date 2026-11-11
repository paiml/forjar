# Implementation receipt — PMAT-492 — apt waits for the lock on shared runners

verdict: PARTIAL — the workflow edits and the guard are measured on the build host, but the only real proof (a release or nightly leg waiting on a contended apt lock) needs a CI run that has not been made on this tree.

## Identity

- ticket PMAT-492 (forjar#492, PR #951); kind: code (CI workflows and shell)
- branch `seat-s3/492`, main merged in at `dbd1ec3e` (0 commits behind, `git rev-list --count HEAD..origin/main`)
- files: `.github/workflows/release.yml`, `.github/workflows/binary-release.yml`, `.github/workflows/nightly.yml`, `scripts/apt-lock-timeout-guard.sh` (new), `scripts/apt-lock-timeout-guard-test.sh` (new), `CHANGELOG.md`, `docs/roadmaps/roadmap.yaml` (row PMAT-492 added)
- diff against `origin/main` before gate A: 5 files changed, 34 insertions, 6 deletions (`git diff origin/main --shortstat`)

## Contract

Every `apt-get update` and `apt-get install` in `.github/workflows/*.yml` passes `-o DPkg::Lock::Timeout=600`, so apt waits up to 600 s for the dpkg lock instead of failing at once. `scripts/apt-lock-timeout-guard.sh [dir]` exits 0 when every such call carries it, 1 on a bare call, 2 when it scanned nothing.

## Evidence (host gx10, tree synced to `~/src/fc-pre/pr-951`, through `fc-pre.sh`, rc=0)

- `cargo fmt --all -- --check`: rc=0.
- `bash scripts/apt-lock-timeout-guard.sh`: rc=0, "6 apt-get call(s) over 20 workflow file(s)".
- `bash scripts/apt-lock-timeout-guard-test.sh`: rc=0, "PASS apt-lock-timeout-guard (real ok, mutant=1, empty=2)": the real tree passes, a planted bare `sudo apt-get install` exits 1, an empty directory exits 2.
- Integration targets that read the changed workflow files (`cargo test --test <name>`):
  - `falsification_nightly_gate_is_tag_vs_head`: 4 passed, 0 failed
  - `falsification_nightly_ships_every_release_linux_target`: 1 passed, 0 failed
  - `falsification_nightly_windows_openssl_perl`: 3 passed, 0 failed
  - `falsification_release_object`: 4 passed, 0 failed
  - `falsification_release_workflow_fixed_paths`: 2 passed, 0 failed
  - `install_sh_parity`: 8 passed, 0 failed
  - `falsification_release_workflow_shape`: 7 passed, 1 failed (`rule2_release_yml_is_the_only_v_star_tag_push_producer`). This is the known LAB red (forjar#858); no CI job runs it. It asserts on v* tag-push producers, which this diff does not touch. It was run once on this tree and not on plain `main`, so "same red on main" is not measured here.

## Not measured

- `cargo clippy --all-targets -- -D warnings` and `cargo test --lib`: not run; the diff changes no Rust file.
- `shellcheck` on the two scripts: not installed on gx10.
- The guard is referenced by no workflow and no Rust test (`grep -rn apt-lock-timeout-guard` finds only the two scripts), so nothing in CI runs it; it is a manual check today.
- A real release, binary-release or nightly leg on a shared runner that waits on a held apt lock: not run. That CI run is the proof.

IMPL-PMAT-492-RECEIPT-END
