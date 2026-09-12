# Implementation receipt — PMAT-547 — every CI job runs on the fleet

verdict: PASS — thirty-two runner declarations across seventeen workflow files moved from GitHub-hosted runners to `[self-hosted, clean-room]`, the two that needed more than a label were rebuilt around what the fleet can actually do, three comments the move made false were rewritten, and a falsification test that PARSES the workflows keeps the count from growing back. Four cases, four killers. Six legs remain on GitHub because the fleet has no macOS and no Windows runner; they are counted, not hidden.

## What was measured

PR #545, run 34685410794. The runner read back from the API rather than from the workflow file, because a workflow that says `ubuntu-latest` is a claim and `runner_name` is what ran it:

```
dogfood-surface  runner=GitHub Actions 1000366553  labels=ubuntu-latest  success
classify         runner=GitHub Actions 1000366551  labels=ubuntu-latest  success
doctests         runner=GitHub Actions 1000366556  labels=ubuntu-latest  success
```

Against the jobs already on the fleet in the same run — `ci / lint` on `intel-clean-room-12`, `ci / coverage` on `intel-clean-room-3`, `ci / test` on `intel-clean-room-8`. Both kinds were green. Nothing about a green check says which machine produced it, which is exactly why this went unnoticed.

`gh api orgs/paiml/actions/runners`: 25 runners, **every one Linux**, labels `self-hosted` 25, `build` 23, `clean-room` 20, `intel` 16, `yoga` 5, `gx10` 4. Zero macOS, zero Windows.

Log §1–§3.

## What changed

Thirty-two declarations across seventeen files, converted by script so every site is the same edit, then checked by parsing. `[self-hosted, clean-room]` is the label this repository's other ten jobs and `release.yml`'s four Linux targets were already using, so the target is the repo's own convention rather than a new one.

`lint.yml`'s matrix was `os: [ubuntu-latest, macos-latest]`. A nested list under `os:` is valid YAML and works, but it makes the runner of a leg something a reader has to assemble. It is now four explicit `include:` legs — same coverage, one unambiguous value each.

Log §4, §7.

## The two moves that needed more than a label

**`nightly.yml` built aarch64 natively.** `cargo build --release --target aarch64-unknown-linux-gnu` works on a hosted ARM runner and cannot work on an x86_64 fleet host: no aarch64 linker, and `vendored-openssl` needs a cross C toolchain. `release.yml` has been building that same target on `[self-hosted, clean-room]` all along with `cross build`, so the leg takes the proven path rather than a new one.

**`binary-release.yml` was pinned to `ubuntu-22.04` for glibc 2.35.** A binary linked against a newer glibc refuses to start on an older host, and that pin was the only thing holding the floor. The pin is gone and the fleet's glibc is not something the workflow could know before it ran — so it measures it. A new step records the builder's glibc and the highest `GLIBC_` version the binary demands, to the log and the job summary, on every release. Verified locally against a real forjar binary: builder 2.35, binary demands at most `GLIBC_2.34`. It never fails the build, because a release that raises the floor is a decision for a reader and not for a step.

Log §5.

## The comments that became false

Three, rewritten rather than left. The one that mattered: `ci.yml` justified its real-sudo test by GitHub's **documented** hosted-runner environment — that the `runner` user is non-root with passwordless sudo. GitHub documents nothing about the fleet, so the citation does not survive the move. `FORJAR_REQUIRE_SUDO_TESTS=1`, which turns a skip into a panic naming the missing capability, is the half that was ever load-bearing and is now the only half. Unmeasured is not the same as satisfied, and the comment now says so.

`release.yml:208` mentions `macos-latest` and is still true; it was left alone.

Log §6.

## Falsification

`tests/falsification_every_ci_job_runs_on_the_fleet.rs` parses the workflow YAML, so a hosted label in a comment is prose and one inside a matrix is a finding.

**It caught a site this change's own converter missed.** `nightly.yml:73` read `runner: ubuntu-24.04-arm   # native ARM64 build (GA, free for public repos)`; the converter matched a runner value to end-of-line, so the trailing comment made the value not match, and the script reported success while a hosted ARM runner stayed. That is the argument for parsing over grepping, made by the change's own tooling against itself.

Four cases, four killers, no collateral:

| case | killed by |
|---|---|
| `no_linux_job_asks_github_for_a_runner` | M1 — one Linux job back to `ubuntu-latest` |
| `the_platforms_the_fleet_cannot_serve_are_exactly_these` | M2 — a seventh hosted leg added |
| `a_fleet_job_names_a_pool_and_not_just_self_hosted` | M3 — `runs-on: self-hosted` alone |
| `the_parser_finds_the_runners_that_are_there` | M4 — `runner_labels` returns nothing |

M4 is the vacuity guard and it is not decoration: a `runner_labels` that returns an empty vector makes the other three cases trivially true over a repository running entirely on GitHub. Every mutation ran against the committed workflows at `92cb6a1a`, and the tree was restored after each.

All 94 cases of the eight workflow-reading suites are green. `actionlint` findings across the workflow directory fall 22 → 15, none added.

Log §8.

## Gaps, named

- **Six legs still run on GitHub.** Two macOS in `lint.yml`, two macOS and one Windows in `nightly.yml`, two macOS in `release.yml` — seven legs across four values, counted as `{lint.yml:macos-latest: 2, nightly.yml:macos-latest: 2, nightly.yml:windows-latest: 1, release.yml:macos-latest: 2}`. The fleet has no such hardware. They can be removed only by dropping this project's coverage of those platforms, which is a decision for a person, or by adding a mac and a Windows box to the fleet. The test makes adding a seventh fail.
- **The conversion is not proven on the fleet by this branch.** Whether all thirty-two jobs actually pass on `clean-room` is decided by CI on this PR and by the workflows that only run on a schedule or a tag — `nightly.yml`, `binary-release.yml`, `mutation.yml`, `stress.yml`, `proofs.yml`. The `cross` path and the guarded `musl-tools` install are both copied from `release.yml`, which already runs on the fleet, but a scheduled workflow will not be observed until it next fires.
- **The glibc floor is recorded, not enforced.** Nothing fails if a future fleet image raises it. Making it a floor needs a number someone is willing to commit to, and this branch does not invent one.
- **`actionlint` is still not run in CI.** `.github/actionlint.yaml` makes a local run useful; wiring it into the PR lane is separate work.

IMPL-PMAT-547-RECEIPT-END
