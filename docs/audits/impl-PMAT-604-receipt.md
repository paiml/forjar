# Implementation receipt — PMAT-604 — the v1.32.0 booking, and Coverage and Benchmarks on tagged releases only

verdict: PASS — v1.32.0, tagged at b12a8392 and published to crates.io on 2026-09-21, has its ledger row: cut 2026-09-21T09:27:03Z, seven PRs, eight tickets, the dogfood and crux receipts, and paiml/forjar-cookbook a8e758ec, whose Cargo.toml admits 1.32.0 and whose Cargo.lock pins it. v1.33.0 is declared, due 2026-09-23T09:27:03Z. In the same PR, Coverage and Benchmarks move to a v* tag push and workflow_dispatch only, and ci.yml opts sovereign-ci's coverage job into `coverage_on: tag` together with the tag trigger that makes it fire.

## Identity

- ticket PMAT-604 (forjar#604, milestone 1.33.0); kind: code, because this PR touches `.github/` and adds a test
- branch `PMAT-604-book-v1.32.0`, from `main` @ b12a8392
- companion PR: paiml/forjar-cookbook#23 (squash a8e758ec), merged before the cut ran
- one PR and one CI run for all of it, on the operator's instruction to stop sending serial PRs to a saturated fleet

## Part 1 — the booking

### The row, as `scripts/release-goal.sh cut v1.32.0 --next v1.33.0` wrote it

```
  - tag: v1.32.0
    cut: 2026-09-21T09:27:03Z
    prs: [570, 577, 583, 593, 596, 599, 603]
    tickets: [PMAT-565, PMAT-566, PMAT-567, PMAT-576, PMAT-579, PMAT-592, PMAT-598, PMAT-601]
    dogfood: docs/audits/dogfood-1.32.0-receipt.md
    crux: docs/audits/crux-1.32.0.md
    cookbook: a8e758ec068a18cb0fbd0046c681227476eb77ce
next:
  tag: v1.33.0
  due: 2026-09-23T09:27:03Z
```

### Gate T on this branch's HEAD

    GATE T v1.32.0 cookbook a8e758ec068a18cb0fbd0046c681227476eb77ce requires forjar 1.32 and locks 1.32.0 ok
    GATE T v1.32.0 cut 2026-09-21T09:27:03Z: 7 PR(s), 8 ticket(s) labelled release:v1.32.0 ok
    GATE T FAIL 3 commit(s) reached HEAD since v1.32.0 and GitHub reports no merged PR containing any of them: work bypassed review, or the window is UNMEASURED — either way this gate cannot pass over it

The first two lines are the booking: the new row reconciles with git and GitHub, and the cookbook arm reads both files at the named commit. The third line is by design and is not waived here. It refuses to pass while this branch's own commits are in no merged PR (`scripts/dogfood/lib/window.sh`, PMAT-229), and every booking branch hits it until the branch merges. It is quoted because a receipt that dropped the red line would be reporting the easy half.

### Gate B on this branch's HEAD

    GATE B PASS comply clean; ruleset 13878864 requires [gate]; 14 gate script(s) and 4 other tracked script(s) at 0 bashrs errors; ratchet CB-200 held; ratchet CB-2110=49/49 CB-2111=49/49 CB-2112=33/35 CB-2114=33/34 CB-2115=40/43 held; required check(s) [gate] reach a dogfood gate; legacy bashrs errors 1 <= 1

CB-2115 was 41 when the release gate ran and is 40 here, which fits issue #604 getting its row. No ceiling is lowered. `pmat comply` reads live GitHub, so a ceiling moves only on a measurement of the merged tree.

### The cookbook commit

Before the cut, paiml/forjar-cookbook#23 bumped `forjar = { version = "1.32", default-features = false }` and locked 1.32.0. Its checks (check, test, coverage, docs, validate-recipes, score) were green when it merged. The cut takes `cookbook:` from `git ls-remote` on the cookbook's master, so the order mattered: merging first is what made the cut record a8e758ec. Both files were read back from that commit through the API, not from the local checkout:

    Cargo.toml  forjar = { version = "1.32", default-features = false }
    Cargo.lock  name = "forjar" / version = "1.32.0"

### What else the booking closes, and what it leaves

- `PMAT-566` → `status: completed`. It merged in #603 and stayed `inprogress` there, because the commit-msg hook refuses a `Pmat-Ticket:` that names a completed row.
- The cut moved `release:v1.32.0` to `release:v1.33.0` on PMAT-526, PMAT-528, PMAT-529 and PMAT-594, since no PR in v1.32.0's window names them. It moved the LABELS only. Their `release:` fields (1.30.0, 1.30.0, 1.30.0, 1.32.0) still agree with their GitHub milestones, so CB-2115 has no DRIFT to report. They now disagree with the labels, which is how 526/528/529 already stood after the 1.31.0 booking. The documented order is milestone first, field second, and moving four milestones on GitHub is outside this booking, so this is recorded here rather than done.
- `PMAT-604` carries `release: 1.33.0` and the 1.33.0 milestone, because a booking PR merges into the NEXT window.

### The release itself — measured 2026-09-21

- tag `v1.32.0` → b12a8392 on origin (annotated tag 2f466428)
- crates.io 1.32.0 is live, published 2026-09-21T09:34:52Z by `make publish-from-tag TAG=v1.32.0`. Publishing is manual by design, behind a human credential.
- docs.rs `/crate/forjar/1.32.0` → 200
- the GitHub Release is still a draft prerelease. When this was written, release.yml run 35583365432 had built three of its six binaries and was still building the rest, so `releases/latest` resolves to v1.31.0. Gate R is the instrument for that arm and is not claimed green here.

## Part 2 — Coverage and Benchmarks on tagged releases only

Operator decision 2026-09-21: "YES, coverage on tags release only" (paiml/aprender#3676).

| file | before | after |
|---|---|---|
| `.github/workflows/coverage.yml` | push to main/master, every PR | `push: tags: ['v*']`, `workflow_dispatch` |
| `.github/workflows/bench.yml` | push to main/master, every PR | `push: tags: ['v*']`, `workflow_dispatch` |
| `.github/workflows/ci.yml` | sovereign-ci coverage on every event | `coverage_on: tag` AND `tags: ['v*']` on its own `push:` |

**Why ci.yml needs two edits.** paiml/.github#74 says so in the input's own description: a `push:` filtered to branches never fires for a tag, so with the input alone coverage would run on manual dispatch only and never on a release. The input was read from the ref ci.yml pins (`sovereign-ci.yml@main`, line 54). It exists, defaults to `always`, and its gate prints `coverage: NOT MEASURED` for a skip on a PR and treats a skip on a v* tag push as RED.

**The heavy jobs still run on the tag.** coverage.yml and bench.yml gate their work on the `changed-class` action. On a tag push `github.event.before` is all zeros, `git rev-parse --verify` fails, the file list is empty, and the classifier's rule for an unmeasured change is `code=true`. So on a release every job runs.

**Nothing ships unmeasured.** Gate F (`scripts/dogfood/coverage.sh`) enforces the 95% line floor and runs `cargo mutants` before every tag, and this PR does not touch it. The 1.32.0 release ran it at 96.44%.

### The falsifier

`tests/falsification_coverage_runs_on_tagged_releases_only.rs` reads parsed YAML, not text, so a comment cannot satisfy it (the forjar#567 lesson). Two tests drive its predicates with fabricated trigger blocks: seven that must be refused, including the exact shape the old files had, plus three opt-in halves, so a predicate that accepts everything goes red. Each declared mutation was applied to a backup-protected copy of the workflow, the test run, and the file restored byte-identical (checked with `cmp`):

| mutation | result |
|---|---|
| add `pull_request:` back to coverage.yml | 3 passed, 1 failed |
| add `branches: [main]` under bench.yml's `push:` | 3 passed, 1 failed |
| delete `coverage_on: tag` from ci.yml | 3 passed, 1 failed |
| delete `tags: ['v*']` from ci.yml's `push:` | 3 passed, 1 failed |
| none (restored tree) | 4 passed |

Every other test that reads these workflows was re-run against the new triggers and passes unchanged, nine in all: `falsification_tool_jobs_run_where_the_tools_are`, `falsification_hosted_jobs_do_not_cache_target`, `falsification_every_ci_job_runs_on_the_fleet`, `falsification_pr_lane_runs_what_the_change_can_break`, `falsification_pr_lane_selects_the_gate_the_change_can_move`, `falsification_ci_runs_doctests`, `falsification_quorum_gate_reads_the_pushed_ref`, `falsification_no_workflow_leaves_a_toolchain_override`, `falsification_lint_refuses_a_toolchain_override`. The set is every file under `tests/` naming `coverage.yml`, `bench.yml` or `ci.yml`, plus the two toolchain-override tests from #596 and #603. `scripts/dogfood/comply.sh` also reads ci.yml, and gate B passes above.

IMPL-PMAT-604-RECEIPT-END
