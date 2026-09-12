# PMAT-547 — adjudicated claims

One round of three sandboxed agy quorum lanes: 3/3 FAIL, 22 findings. Seven
confirmations and six refutations, every one re-measured before it was acted on.

The conversion held. What did not hold was the branch's confidence about its own
blast radius: it broke a suite it had never run, its parser had four holes a lane
named and a fifth the fix introduced, and two of its numbers were the script's
report rather than a measurement.

## CONFIRMED

1. [broke-a-suite] That this branch broke
   `tests/falsification_hosted_jobs_do_not_cache_target` (lane 2).
   - evidence: re-run on the branch as the lanes saw it, two cases were red —
     `the_scan_reaches_the_real_coverage_job` and
     `every_hosted_coverage_job_reduces_debug_info`, both on
     `scan.hosted_coverage_jobs > 0`. That suite guards a HOSTED coverage job
     against caching `target/`, the 70.70 GiB ENOSPC of #386; moving
     `coverage.yml` to the fleet emptied its denominator. The orchestrator had
     run eight workflow-reading suites and not this one. The guard is
     `tests/falsification_hosted_jobs_do_not_cache_target/main.rs:105` and the
     denominator it now uses is
     `tests/falsification_hosted_jobs_do_not_cache_target/scan.rs:243`, a count
     of coverage jobs on ANY runner -- so the debug-info knob that shrank the
     tree 70.70 GiB to 23 GiB keeps its guard after the move instead of losing
     it along with the hosted label.

2. [object-form] That `runs-on: { group: …, labels: [ubuntu-latest] }` hid a
   hosted runner from the parser (lane 2, `asserted`).
   - evidence: now `measured` by fixture. `push_labels` matched only
     `Value::String` and `Value::Sequence`, so the object form contributed no
     labels at all and the job was invisible. The case is
     `tests/falsification_every_ci_job_runs_on_the_fleet.rs:439`, and it failed
     against the parser as written before the `Value::Mapping` arm was added.

3. [case] That `hosted_label`'s prefix check was case-sensitive and would pass
   `macOS-latest` (lane 2, `asserted`).
   - evidence: now `measured`. GitHub accepts `macOS-Latest` and
     `Ubuntu-Latest`; the check now lowercases first. The fixture driving both
     spellings is at
     `tests/falsification_every_ci_job_runs_on_the_fleet.rs:459`.

4. [matrix-key] That a matrix key not named `runner` or `os` hid a hosted
   runner, and that an expression nested inside a `runs-on` LIST never reached
   the matrix (lane 2, `asserted`).
   - evidence: both now `measured` by fixture. `matrix_labels` hardcoded two key
     names and `is_expr` matched only a bare `Value::String`. The parser now
     reads every matrix key except `include`/`exclude`, and descends into lists.
     The two cases are at
     `tests/falsification_every_ci_job_runs_on_the_fleet.rs:479` and `tests/falsification_every_ci_job_runs_on_the_fleet.rs:500`.

5. [cross-glibc] That the new glibc step measured the wrong machine on a
   cross-built leg (lane 1, `asserted`).
   - evidence: `ldd --version` reports the glibc of the host running the step,
     and the aarch64 binary is built inside a container on an x86_64 host, so
     the builder line described a machine the binary never linked against. The
     step now claims a builder only on `x86_64-unknown-linux-gnu`. Run against a
     real binary on both paths: x86_64 prints `builder: ldd (Ubuntu GLIBC
     2.35-0ubuntu3.11) 2.35`, aarch64 prints `not applicable — … was
     cross-built`. The DEMANDED floor, read out of the binary with `strings`, is
     `GLIBC_2.34` on both and was always correct.

6. [delegated] That a job calling a reusable workflow declares no `runs-on` and
   is skipped by every case in the new test (lane 3).
   - evidence: three such jobs — `ci.yml:ci` and `nightly-bench.yml:bench` to
     `sovereign-ci.yml`, `pr-gate.yml:authorize` to `pr-gate.yml`, all in
     `paiml/.github`. No test here can read where they run. They are counted
     instead, by
     `tests/falsification_every_ci_job_runs_on_the_fleet.rs:298`, so adding a
     fourth is a decision someone makes on purpose rather than a silent gap.

7. [thirty-three] That the branch moved 33 declarations and not 32 (lane 2,
   `measured`).
   - evidence: re-derived independently by counting hosted-Linux runner labels
     that exist at `0bb6cb57` and no longer exist at HEAD, through the same
     parser: 33 across 17 files. The 32 was what the conversion script reported;
     the 33rd is the site the script missed and the test caught.

## REFUTED

1. [six-legs] That seven macOS/Windows legs remain — the branch said six, in the
   receipt, the log, the commit message, the test's doc comment and
   `.github/actionlint.yaml` (lanes 1 and 3, `measured`).
   - corrected: seven. Counted from the parsed YAML:
     `{lint.yml:macos-latest: 2, nightly.yml:macos-latest: 2,
     nightly.yml:windows-latest: 1, release.yml:macos-latest: 2}`. `grep` prints
     six LINES because `lint.yml` contributes one line and two legs, and the
     count was taken before this branch rewrote that matrix into explicit legs.
     The test's expected map had 7 the whole time, which is why nothing was red,
     and the receipt contradicted ITSELF in one bullet. Corrected before either
     lane reported, by counting rather than by reading.

2. [group-is-a-label] That reading the object form's `group` alongside its
   `labels` is correct — this receipt's own first fix for CONFIRMED 2.
   - corrected: a runner GROUP is not a runner label. Reading it made a group
     named `ubuntu-runners` report as a hosted runner, and the fleet control
     caught it on the first run:
     `left: [("build", "ubuntu-latest"), ("build", "ubuntu-runners")]`. The arm
     reads `labels` only, and the control is
     `tests/falsification_every_ci_job_runs_on_the_fleet.rs:522`. A fix that ships with a control finds its own defects
     in one test run instead of one release.

3. [silent-guard] That the `musl-tools` guard "proceeds silently" to an obscure
   failure (lane 1).
   - corrected: it never proceeded silently — the `else` branch wrote to stderr.
     The lane's underlying point stands though, so the message is now a
     `::warning::` annotation naming what is missing and what to do. Refusing
     outright was considered and rejected: `release.yml` builds musl targets on
     this same pool with a weaker guard, so a hard failure here would refuse a
     configuration that is known to work.

4. [cache-degrades] That `actions/cache` "degrades on clean-room runners" and
   should be removed from self-hosted jobs (lane 1).
   - corrected: `actions/cache` talks to GitHub's cache service over the network
     and works from a self-hosted runner. This branch changes nothing about it,
     and `falsification_hosted_jobs_do_not_cache_target`'s own control
     `self_hosted_coverage_jobs_are_exempt` records the deliberate decision that
     a fleet box caching `target/` is fine because its disk is ours. A real
     question, but not a finding against this diff.

5. [sudo-breaks] That moving `ci.yml`'s job breaks its real-sudo test if the
   fleet has no passwordless sudo (lane 1).
   - corrected: that is the DESIGNED behaviour, not a regression.
     `FORJAR_REQUIRE_SUDO_TESTS=1` turns a skip into a panic naming the missing
     capability, which is why the comment above it was rewritten to drop the
     citation to GitHub's documented environment and keep only the switch. If
     the fleet cannot give that step sudo, this repository should be told
     loudly. CI on this PR is what measures it.

7. [arch] That `[self-hosted, clean-room]` chooses an architecture (this
   branch, in every one of its 33 conversions).
   - corrected: it does not. Measured by runner GROUP rather than by label:
     group 1 "Default" is sixteen `intel-clean-room-*`, every one `X64`; group 3
     "gpu-nodes" is four gx10 boxes that ALSO carry `clean-room` and are every
     one **ARM64**. forjar reaches only the X64 group today because groups 3 and
     5 are `visibility=selected` — an access-control accident, not a property of
     the label, and one checkbox away from handing an
     `x86_64-unknown-linux-gnu` build an ARM64 runner. Fifty declarations across
     eighteen files now say `[self-hosted, clean-room, X64]`; seventeen of them
     were already on the fleet before this branch with the same unstated
     assumption. The case is
     `tests/falsification_every_ci_job_runs_on_the_fleet.rs:253`.

6. [threshold] That `the_parser_finds_the_runners_that_are_there` is a real
   guard at a floor of 20 (this branch).
   - corrected: measured at 48, so 28 labels of slack — more than half the fleet
     jobs could have been deleted before it noticed. The floor is now 40. A
     vacuity guard whose margin is larger than the thing it guards is decoration.
