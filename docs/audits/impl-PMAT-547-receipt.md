# Implementation receipt — PMAT-547 — every CI job runs on the fleet

verdict: PASS — **thirty-three** `ubuntu-` runner declarations across **seventeen** workflow files moved from GitHub-hosted runners to `[self-hosted, clean-room, X64]`, and no `ubuntu-` declaration remains anywhere in `.github/workflows/`. The two that needed more than a label were rebuilt around what the fleet can actually do, three comments the move made false were rewritten, and a falsification test that PARSES the workflows keeps the count from growing back. Thirteen cases, seven killers.

Three numbers, because three rounds of review lanes each disputed a different
reading of one of them. Measured on `origin/main` and on this head:

| | main | this head |
|---|---|---|
| `ubuntu-` declarations (`runs-on`, matrix `runner:`, matrix `os:`) | 33 | **0** |
| fleet declarations (`self-hosted`) | 14 | 48 |
| `macos-`/`windows-` declarations | 7 | 7 |

So: **33 MOVED** (hosted → fleet). The **14** already on the fleet —
`release.yml`'s ten and `proofs.yml`'s four — did not move; they gained the `X64`
label. 33 + 14 = 47 and the head carries 48, so **one fleet declaration is new**.
The 7 macOS and Windows legs are unchanged and are the expected map that
`the_platforms_the_fleet_cannot_serve_are_exactly_these` asserts.

An earlier draft of this receipt said "thirty-four moved", which was this author
counting 48 − 14 and calling the difference a move. A declaration that was already
on the fleet and gained a label did not move, and a declaration this branch created
was never anywhere else. The three rows above are the numbers; the prose no longer
carries any count the table does not.

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

Thirty-three declarations across seventeen files. Thirty-two were converted by script so every site is the same edit; the thirty-third was `nightly.yml`'s `runner: ubuntu-24.04-arm`, which the script missed and the parsing test caught. The count in the first draft of this receipt was 32, taken from what the script reported and never retaken. `[self-hosted, clean-room]` is the label this repository's other ten jobs and `release.yml`'s four Linux targets were already using, so the target is the repo's own convention rather than a new one.

`lint.yml`'s matrix was `os: [ubuntu-latest, macos-latest]`. A nested list under `os:` is valid YAML and works, but it makes the runner of a leg something a reader has to assemble. It is now four explicit `include:` legs — same coverage, one unambiguous value each.

Log §4, §7.

## The architecture nobody had named

`clean-room` spans BOTH architectures. Measured by runner GROUP rather than by
label: group 1 "Default" is sixteen `intel-clean-room-*`, every one `X64`; group
3 "gpu-nodes" is four gx10 boxes that also carry `clean-room` and are every one
**ARM64**. Groups 3 and 5 are `visibility=selected` and forjar is not in them, so
every job this branch ran landed on an intel box and nothing is wrong today —
but that is an access-control accident, not a property of the label. Adding
forjar to `gpu-nodes` is one checkbox, and a job building
`x86_64-unknown-linux-gnu` handed an ARM64 runner uploads an artifact that is
WRONG rather than missing.

Forty-eight declarations across eighteen files now say `[self-hosted, clean-room,
X64]` — forty `runs-on:` and eight matrix `runner:` entries. A plain string grep
returns FIFTY; two of those are prose in comments (`binary-release.yml` lines 5
and 176), which is the difference between grepping and parsing that this whole
ticket is about, found in this receipt by a review lane.
FOURTEEN of them were already on the fleet before this branch — `release.yml`'s ten
and `proofs.yml`'s four — carrying the same unstated assumption. `bench.yml` is NOT
among them: both of its jobs read `runs-on: ubuntu-latest` on `main` and this
branch moved them, which an earlier draft of this receipt had the wrong way round.

Log §10.

## The two moves that needed more than a label

**`nightly.yml` built aarch64 natively.** `cargo build --release --target aarch64-unknown-linux-gnu` works on a hosted ARM runner and cannot work on an x86_64 fleet host: no aarch64 linker, and `vendored-openssl` needs a cross C toolchain. `release.yml` has been building that same target on `[self-hosted, clean-room]` all along with `cross build`, so the leg takes the proven path rather than a new one.

**`binary-release.yml` was pinned to `ubuntu-22.04` for glibc 2.35.** A binary linked against a newer glibc refuses to start on an older host, and that pin was the only thing holding the floor. The pin is gone and the fleet's glibc is not something the workflow could know before it ran — so it measures it AND refuses what the pin would have refused. A new step records the builder's glibc and the highest `GLIBC_` version the binary demands, and exits 1 when that exceeds `GLIBC_CEILING=2.35` — the floor 22.04 held, to the log and the job summary, on every release. Verified locally against a real forjar binary: builder 2.35, binary demands at most `GLIBC_2.34`. It also EXITS 1 above GLIBC_CEILING=2.35, the floor ubuntu-22.04 enforced before the pin was removed: an earlier version of this sentence said it never fails the build, and two review lanes called that a weakened gate.

Log §5.

## The comments that became false

Three, rewritten rather than left. The one that mattered: `ci.yml` justified its real-sudo test by GitHub's **documented** hosted-runner environment — that the `runner` user is non-root with passwordless sudo. GitHub documents nothing about the fleet, so the citation does not survive the move. `FORJAR_REQUIRE_SUDO_TESTS=1`, which turns a skip into a panic naming the missing capability, is the half that was ever load-bearing and is now the only half. Unmeasured is not the same as satisfied, and the comment now says so.

`release.yml:208` mentions `macos-latest` and is still true; it was left alone.

Log §6.

## Falsification

`tests/falsification_every_ci_job_runs_on_the_fleet/` parses the workflow YAML — split into `main.rs` (the seven invariants), `controls.rs` (five fixtures) and `scan.rs` (the parser), because one file reached the 500-line gate at 539 lines and CI's `file-health` ratchet refused it, so a hosted label in a comment is prose and one inside a matrix is a finding.

**It caught a site this change's own converter missed.** `nightly.yml:73` read `runner: ubuntu-24.04-arm   # native ARM64 build (GA, free for public repos)`; the converter matched a runner value to end-of-line, so the trailing comment made the value not match, and the script reported success while a hosted ARM runner stayed. That is the argument for parsing over grepping, made by the change's own tooling against itself.

Thirteen cases. Seven are invariants, six are controls a review lane's findings
turned into fixtures, one counts the jobs whose runner this repository does not
choose, and one pins the architecture:

| case | killed by |
|---|---|
| `no_linux_job_asks_github_for_a_runner` | M1 — one Linux job back to `ubuntu-latest` |
| `the_platforms_the_fleet_cannot_serve_are_exactly_these` | M2 — an EIGHTH hosted leg added (seven already stand) |
| `a_fleet_job_names_a_pool_and_not_just_self_hosted` | M3 — `runs-on: self-hosted` alone |
| `the_parser_finds_the_runners_that_are_there` | M4 — `runner_labels` returns nothing |
| `a_fleet_job_names_the_architecture_it_needs` | M5 — one fleet job drops its `X64` |
| `a_fleet_job_that_runs_cargo_keeps_its_registry_to_itself` | M6 — one fleet job loses its private `CARGO_HOME` |
| `no_linux_job_asks_github_for_a_runner` | M7 — `runs-on: Ubuntu-latest` in audit.yml |

M4 is the vacuity guard and it is not decoration. Re-run at `607e229b` after the round grew the suite to ten cases, and re-aimed at `push_labels` because the round rewrote `runner_labels`, **M4 kills six of the ten, not one** — every case that reads through the parser. The survivors are exactly the cases that assert an ABSENCE, and a blinded parser reports nothing hosted, so they pass while measuring nothing. RE-MEASURED AT THIS HEAD, where the suite is thirteen: M4 kills six of the thirteen — the four controls, `the_parser_finds_the_runners_that_are_there` and `the_platforms_the_fleet_cannot_serve_are_exactly_these` — and seven survive: the four absence-assertions, `a_fleet_job_that_runs_cargo_keeps_its_registry_to_itself`, `controls::a_fleet_job_is_not_mistaken_for_a_hosted_one` and `controls::a_mixed_case_linux_label_is_still_a_linux_label`, which reads `linux_label` directly and never touches the parser. That is the failure `the_parser_finds_the_runners_that_are_there` exists to catch. An earlier draft of this receipt said "four single kills, no collateral": true of a four-case suite, false of this one. M1 and M2 do each kill exactly one case. **M3 kills TWO** — measured: dropping the pool from `[self-hosted, clean-room, X64]` to a bare `self-hosted` drops the architecture with it, so `a_fleet_job_names_a_pool_and_not_just_self_hosted` and `a_fleet_job_names_the_architecture_it_needs` both die. An earlier draft said three single kills; a review lane ran the mutation and found two. M7 kills one, and it is `no_linux_job_asks_github_for_a_runner`, the case that owns the rule — which is the whole point of adding `linux_label`; the control beside it is a unit test on `linux_label` and no workflow mutation can reach it. Every mutation ran against the committed tree and the tree was restored after each.

### The case that owns the rule was blind to case, and a lane found it

`scan::hosted_label` lower-cases before it matches — the receipt above says so and
a control pins it. The two call sites in `main.rs` did NOT: they filtered
`hosted_sites()` with a bare `label.starts_with("ubuntu-")`.

Measured, by putting `runs-on: Ubuntu-latest` into `audit.yml`:

```
before: the_platforms_the_fleet_cannot_serve_are_exactly_these ... FAILED   (11 passed, 1 failed)
after:  no_linux_job_asks_github_for_a_runner ... FAILED                    (12 passed, 1 failed)
```

The suite never failed open — a mixed-case Linux label was caught either way — so
nothing could have shipped. What was wrong is WHICH case caught it: the platform
census fired and reported a Linux runner as "a platform the fleet cannot serve",
while `no_linux_job_asks_github_for_a_runner`, the case whose doc comment reads
*this is the whole ticket*, stayed green. A green invariant that cannot see its
own violation is the shape this repository keeps finding, and it was in the suite
written to catch that shape.

`scan::linux_label` lower-cases, both call sites use it, and
`controls::a_mixed_case_linux_label_is_still_a_linux_label` pins it. M7 is the
killer above: with the fix in place the mutation kills the case that owns the rule,
and only that one.

### A fleet job cannot share the runner user's ~/.cargo, and that is part of the move

24 fleet jobs that run cargo now declare

```yaml
CARGO_HOME: ${{ github.workspace }}/../cargo-home-${{ github.job }}
```

**This exposure ARRIVED WITH THE MOVE and is not a separate fix.** On a
GitHub-hosted runner every job has a private `~/.cargo` by construction, so a job
that had never left GitHub had never been on the shared surface. On the fleet,
sixteen clean-room runners share one `~/.cargo` while an hourly reaper deletes
`registry/src` entries by mtime, under live builds — paiml/infra#430 — and the
failure it produces is a cargo error naming a file that vanished mid-build:
`No such file or directory (os error 2)`. Moving a cargo job onto a shared
registry and leaving it there is not a smaller change than this one; it is the
same change with the consequence unhandled.

`proofs.yml:ledger-replay` had already hit it and already carried this exact
line, with a comment naming infra#430. It was one job's footnote; this branch
makes it the rule, and six jobs that were on the fleet BEFORE this branch
(`proofs.yml:kani`, release.yml's four, `bench`) carried the same exposure
unnoticed until the census that this ticket's parser made possible.

The path is outside the workspace, so it survives the checkout clean: a cache
per (runner, job), not a cold download per run. `a_fleet_job_that_runs_cargo_
keeps_its_registry_to_itself` is the invariant and M6 is its killer — one fleet
job loses its private `CARGO_HOME` and that one case dies while the other eleven
stay green.

### What the review round changed here

A lane claimed four shapes could hide a hosted runner from this parser, all
`asserted`. Each is now a fixture and a case, and two of the four were real:

| shape | was it hiding one |
|---|---|
| `runs-on: { group: …, labels: [ubuntu-latest] }` | YES — `push_labels` matched only `String` and `Sequence` |
| `macOS-Latest` / `Ubuntu-Latest` | YES — the prefix check was case-sensitive |
| a matrix key not named `runner`/`os` | YES — both names were hardcoded |
| an expression inside a `runs-on` LIST | YES — `is_expr` looked only at a bare string |

Fixing the first one introduced a defect of its own, which the fifth control
caught immediately: reading `group` as well as `labels` made a runner group
called `ubuntu-runners` report as a hosted runner. `a_fleet_job_is_not_mistaken_for_a_hosted_one`
is why that lasted one test run.

A second lane found that jobs calling a REUSABLE workflow declare no `runs-on`
at all, so every case above passes over them silently. There are three, their
runner is chosen by `paiml/.github`, and no test here can read it —
`the_jobs_that_delegate_their_runner_are_exactly_these` counts them instead.

**A lane also found a suite this branch BROKE.**
`tests/falsification_hosted_jobs_do_not_cache_target` guards against a hosted
coverage job caching `target/` — the 70.70 GiB ENOSPC that killed the runner's
Worker in #386 — and its vacuity guard requires at least one hosted coverage
job. Moving `coverage.yml` to the fleet made that zero and two cases went red.
The fix is not to lower the guard: the CACHE rule is about a disk we do not own
and stays hosted-only, but the debug-info knob that shrank the tree 70.70 GiB →
23 GiB applies to any machine, and it would have been dropped along with the
hosted label. `Scan` now counts coverage jobs on any runner, the debug-info
assertion uses that denominator, and the rule that lane 1 called an exemption
stays an exemption.

All 13 cases of the new suite and all 10 of `falsification_hosted_jobs_do_not_cache_target`
are green, with the eight workflow-reading suites. `actionlint` findings across
the workflow directory fall 22 → 12, none added. An earlier draft said 15 and
claimed none was added, and both halves could not be true: three of the fifteen
were SC2016 inside the `Record the glibc floor` step this branch WROTE. They are
markdown backticks in a `printf` FORMAT string and must reach the step summary
literally, so they carry a named `# shellcheck disable=SC2016` with that reason
rather than a receipt sentence explaining them away. Measured after: 12, which is
the count without the step at all. Found by a review lane.

Log §8.

## Gaps, named

- **Seven legs still run on GitHub.** Two macOS in `lint.yml`, two macOS and one Windows in `nightly.yml`, two macOS in `release.yml` — seven legs across four `(file, label)` pairs, counted as `{lint.yml:macos-latest: 2, nightly.yml:macos-latest: 2, nightly.yml:windows-latest: 1, release.yml:macos-latest: 2}`. The fleet has no such hardware. They can be removed only by dropping this project's coverage of those platforms, which is a decision for a person, or by adding a mac and a Windows box to the fleet. The test makes adding an eighth fail.
- **The conversion is not proven on the fleet by this branch.** Whether all thirty-three jobs actually pass on `clean-room` is decided by CI on this PR and by the workflows that only run on a schedule or a tag — `nightly.yml`, `binary-release.yml`, `mutation.yml`, `stress.yml`, `proofs.yml`. The `cross` path and the guarded `musl-tools` install are both copied from `release.yml`, which already runs on the fleet, but a scheduled workflow will not be observed until it next fires.
- **The glibc floor is ENFORCED again, at 2.35.** An earlier version of this line
  said "recorded, not enforced — nothing fails if a future fleet image raises it",
  and that was a weakened gate disclosed rather than a gate. Two review lanes said
  so. The number nobody wanted to invent did not need inventing: 2.35 is the floor
  `ubuntu-22.04` enforced before the pin was removed, so restoring it restores what
  was there rather than choosing something new. The step exits 1 above it, naming
  the target and the demanded version. Measured today: the binary demands 2.34, so
  the ceiling passes with room and fires the day the fleet moves past what the pin
  allowed.
- **`actionlint` is still not run in CI.** `.github/actionlint.yaml` makes a local run useful; wiring it into the PR lane is separate work.
- **The architecture pin is a guess about capacity, not about need.** Every runner forjar can reach is X64, so `[self-hosted, clean-room, X64]` costs nothing today. If forjar is ever given the ARM64 `gpu-nodes` group deliberately — to build an aarch64 artifact natively instead of under `cross` — the pin is what has to be relaxed, one job at a time, and `a_fleet_job_names_the_architecture_it_needs` is where the decision gets written down.

IMPL-PMAT-547-RECEIPT-END
