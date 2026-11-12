# Implementation receipt — PMAT-492 — apt waits for the lock on shared runners

verdict: PARTIAL — the workflow edits and the guard are measured on the build host, and `cargo test` now runs the guard, but the only real proof (a release or nightly leg waiting on a contended apt lock) needs a CI run that has not been made on this tree, and the new test is wired into no workflow.

## Identity

- ticket PMAT-492 (forjar#492, PR #951); kind: code (CI workflows and shell)
- branch `seat-s3/492`, main merged in at `dbd1ec3e` (0 commits behind, `git rev-list --count HEAD..origin/main`)
- files: `.github/workflows/release.yml`, `.github/workflows/binary-release.yml`, `.github/workflows/nightly.yml`, `scripts/apt-lock-timeout-guard.sh` (new), `scripts/apt-lock-timeout-guard-test.sh` (new), `tests/falsification_492_apt_lock_guard.rs` (new), `CHANGELOG.md`, `docs/roadmaps/roadmap.yaml` (row PMAT-492 added)
- diff of the three workflows against `origin/main`: 3 files, `-o DPkg::Lock::Timeout=600` added to 6 `apt-get update|install` calls

## Contract

Every `apt-get update` and `apt-get install` in `.github/workflows/*.yml` passes `-o DPkg::Lock::Timeout=N` with N a positive integer, so apt waits up to N seconds for the dpkg lock instead of failing at once. `scripts/apt-lock-timeout-guard.sh [dir]` judges each apt-get invocation on its own (continuation lines joined, `# comments` stripped, split on `&&`, `||`, `;`, `|`): exit 0 when every call carries it, 1 on any bare call or `Timeout=0`/empty/non-numeric value, 2 when it scanned nothing. `echo`/`printf` prose and YAML keys other than `run:` are not calls. `--self-test` runs `scripts/apt-lock-timeout-guard-test.sh`.

## Review round 1 (agy, three lanes at 77742fbe) and what changed

- lane 1, line-level grep passed a bare call with a trailing `# ...DPkg::Lock::Timeout` comment, `Timeout=0`, or `update && sudo apt-get install`: fixed by judging each invocation (cases `trailing-comment-names-the-option`, `timeout-zero`, `timeout-empty`, `timeout-not-a-number`, `chained-second-call-bare`, `semicolon-second-call-bare`).
- lane 2, `ls` plus unquoted `$files` and a shellcheck disable; a false positive on `echo "apt-get is missing; install musl-tools manually"`; unchecked `mktemp -d`: fixed with a bash array under `nullglob` and no suppression, command-position matching, `T=$(mktemp -d) || exit 2` (cases `space-in-dir-name`, `space-in-dir-name-good`, `echo-prose-is-not-a-call`, `echo-warning-prose`, `step-name-is-not-a-call`, `only-prose-no-calls`). Also found by the new cases: a single-file scan lost the `file:` prefix, so the whole-line comment filter never matched (`comment-line-ignored`), and a backslash continuation hid the subcommand (`continuation-line-bare`, `continuation-line-good`).
- lane 3, no `--self-test` and nothing in `cargo test` ran the guard: `--self-test` added, and `tests/falsification_492_apt_lock_guard.rs` runs the guard on the real tree, the case script, and `--self-test`. No workflow, `gate.needs` or required-check list was edited.

## Evidence (host gx10, tree synced to `~/src/fc-pre/pr-951`, through `fc-pre.sh`, rc=0)

RED, the 28-case script against the unchanged guard (commit da03460f), `bash scripts/apt-lock-timeout-guard-test.sh`, rc=1: 15 of 28 cases wrong (`trailing-comment-names-the-option`, `timeout-zero`, `timeout-empty`, `timeout-not-a-number`, `chained-second-call-bare`, `semicolon-second-call-bare`, `continuation-line-bare`, `space-in-dir-name`, `continuation-line-good`, `comment-line-ignored`, `echo-prose-is-not-a-call`, `echo-warning-prose`, `step-name-is-not-a-call`, `space-in-dir-name-good`, `only-prose-no-calls`). The full output is in that commit's message.

GREEN, with the new guard:

- `bash scripts/apt-lock-timeout-guard-test.sh`: rc=0, "PASS apt-lock-timeout-guard (28 cases ...)".
- `bash scripts/apt-lock-timeout-guard.sh`: rc=0, "6 apt-get call(s) over 20 workflow file(s)".
- `bash scripts/apt-lock-timeout-guard.sh --self-test`: rc=0.
- `cargo fmt --all -- --check`: rc=0 (after `cargo fmt --all`, which reformatted the new test file).
- `cargo clippy --test falsification_492_apt_lock_guard -- -D warnings`: rc=0.
- `cargo test --test falsification_492_apt_lock_guard`: 3 passed, 0 failed.
- Mutation: with `-o DPkg::Lock::Timeout=600` removed from the first `update` call in `release.yml` on the host copy, the same target ran 0 passed, 3 failed; the file was restored and the host tree removed afterwards.
- Targets that read the changed workflow files (`cargo test --test <name>`): `falsification_nightly_gate_is_tag_vs_head` 4 passed, `falsification_nightly_ships_every_release_linux_target` 1 passed, `falsification_nightly_windows_openssl_perl` 3 passed, `falsification_release_object` 4 passed, `falsification_release_workflow_fixed_paths` 2 passed, `install_sh_parity` 8 passed; all 0 failed.
- Known LAB red `falsification_release_workflow_shape` (forjar#858, no CI job runs it): 7 passed, 1 failed (`rule2_release_yml_is_the_only_v_star_tag_push_producer`) in round 0 at the earlier head; not re-run this round.

## Not measured

- `cargo test --lib` and full-crate `cargo clippy --all-targets`: not run; the round changes no `src/` file.
- `shellcheck` on the scripts: not installed on gx10.
- Whether any CI job runs `cargo test --test falsification_492_apt_lock_guard`: not measured, and no workflow was edited to make it so. Wiring a check into a workflow or required-check set is a gate change for the gate owner.
- Known gaps in the guard: a file whose last line ends in a backslash is not judged unless it is the last file scanned; `apt` (not `apt-get`), `apt-get upgrade|dist-upgrade|build-dep` and calls assembled from shell variables are not scanned.
- A real release, binary-release or nightly leg on a shared runner that waits on a held apt lock: not run. That CI run is the proof.

IMPL-PMAT-492-RECEIPT-END
