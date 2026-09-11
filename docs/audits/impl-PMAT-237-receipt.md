# Implementation receipt — PMAT-237 — a PR runs what its change can break, the release still runs everything

verdict: PASS — measured on the ten most recently merged PRs, one PR cost about 131 job-minutes (ledger-replay 34, dogfood 26, bench 18, the workspace suite 14, coverage 12, and the rest) on a fleet of twenty runners shared with every other paiml repository. The heavy jobs now run only when `scripts/ci/changed-class.sh` says the change can reach them; `make dogfood-release` is untouched and still runs everything. **Coverage is deliberately not gated**: the 95% floor is a property of the repository, not of one diff. The honest saving is three of ten PRs skipping seven jobs — 70% of the job-minutes for a record-only change — not the 80% a first reading of "eight touched no src/" suggests, and the script's own header carries the smaller number.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=agy-goal  w=1.00  basis=absent  note=fable-binding  effort=1[U]  (executed by self: workers and lanes may not edit .github/workflows)
  ph1.quorum  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate, three lanes, one returned — see below)
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_pr_lane_runs_what_the_change_can_break (13 cases), the registered mutation and the two wiring mutations"  claimed_exit=101(lane)  rerun_exit=0  log_path=docs/audits/logs/PMAT-237-class-census.log  sha256=recorded-in-the-log
  cmd="cargo llvm-cov --workspace --locked --summary-only --fail-under-lines 95 on this branch"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-237-class-census.log  sha256=recorded-in-the-log

## The measurement

| job | minutes on one PR |
|---|---|
| ledger-replay | 34 |
| dogfood | 26 |
| benchmark | 18 |
| ci / test | 14 |
| coverage | 12 |
| ci / lint, ci / coverage, examples-validate, the rest | ~27 |

Eight of the ten most recently merged PRs touched no `src/` at all. Only **three** are confined to the record, because the rest also changed scripts, tests or workflows — which are code. Three in ten, and 70% of the minutes for each of those three.

## The decision, and the direction it fails in

`scripts/ci/changed-class.sh` reads a file list and prints `code=true|false`. It is an **allow-list of the harmless**, never a deny-list of the heavy: an unclassified path, an unreadable diff and an empty list are all code. Being wrong that way costs CI minutes; being wrong the other way ships untested code, and those are not the same mistake.

It is a script rather than a YAML `if:` so a test can drive it. Thirteen cases do, in both directions.

**The harmless set is narrower than it first looks, and it was measured rather than guessed.** Every `CARGO_MANIFEST_DIR`-joined path under `docs/` or `.quorum/` in `tests/` and `src/` is read from the real tree, so each is code: `CHANGELOG.md`, `docs/audits/crux-*`, `docs/specifications/*`, `docs/book/*`, `docs/mcp-schema.json`, `docs/audits/surface_audit.csv`, `.quorum/enforce.json`, `README.md`, `contracts/**`. `every_record_path_a_test_reads_is_classified_as_code` re-derives that list from the tree, so a test that starts reading a new record file turns the suite red rather than blinding itself.

## Coverage is not gated

The 95% line floor is a standing property of the repository rather than of one diff, and the cheapest way to keep a floor is to measure it every time rather than to reason about when it could not have moved. It costs twelve of the 131 minutes, so the saving is 70% and not 79%. `coverage_is_never_gated` pins it: no class condition on that job, and the workflow still names `--fail-under-lines 95`.

Measured on this branch rather than asserted: `cargo llvm-cov --workspace --locked --summary-only --fail-under-lines 95` exits 0 at **95.69% lines** (92.80% regions). The floor holds, and the margin is narrower than the 96.38% the 1.27.0 receipt recorded and the 96.45% measured at the 1.28.0 cut — which is the kind of drift that only stays visible if the number is taken every time.

## The saving cannot become a hole

Three ways it could, and each is closed:

1. **A skipped job trusted.** `ci.yml`'s gate and `proofs.yml`'s aggregator each re-read the class and FAIL when a heavy job was skipped on a change the classifier called code.
2. **An unmeasured class.** If `classify` fails or is cancelled its output is empty, every heavy job's `if` is false, and they all skip. The gate now checks `needs.classify.result` and refuses an empty class; the aggregator defaults an absent class to code. **A review lane found this, and it was the worst outcome the design had.**
3. **A rename out of the source tree.** `git diff --name-only` reports a rename as its new path alone, so `src/thing.rs` → `docs/thing.rs` would list one harmless path while deleting a source file. `--no-renames` splits it into a delete and an add, and the deletion is code. **The same lane found this.**

## Falsification

Thirteen cases, and three mutations measured one at a time (`docs/audits/logs/PMAT-237-class-census.log`): the unclassified arm made harmless turns three cases red; a heavy job ungated turns its rule red; the gate's refusal text removed turns its rule red.

## Review record, and a lane that wrote to the repository

Three lanes were dispatched; **one returned**. Its two findings are the two holes above, both closed. It also refuted two claim wordings — that `README.md` and `contracts/**` are "under the harmless prefixes" (they are not, and the case list now says so) and that "nine cases drive the script" (six do; three test the wiring).

**One of the other lanes created files inside the operator's working tree and committed them to the working branch** while probing how the classifier handles a path with a space and a non-ASCII byte. The commit was `16c2af56 quote`, adding `docs/ä.md`; a later lane staged `docs/space file.md` and `docs/ünicode.md` and left an untracked `test`. All of it was reset and removed, the branch verified back at its own commit, and the round was not re-run: a second round carried the same risk, and the findings that mattered were already in hand and fixed. This receipt records the damage rather than the two lanes' silence, because the silence is what a reader would otherwise assume was a clean round.

Those paths can be tested by feeding strings to the classifier on stdin — it reads a file list and needs no files to exist — which is what the brief should have said and now says for any future round.

## Gaps, named

- The round is one lane deep. The thirteen cases and the three mutations are the evidence; two lanes are silent and one of them damaged the tree.
- A PR can edit `scripts/ci/changed-class.sh` itself, and the edited script is what its own `classify` job runs. The script is code, so every heavy job runs on such a PR, but a same-repo PR is trusted in this model and that is stated rather than solved.
- `docs/book/**` and `docs/specifications/**` are code because a test reads two files under them by name. A narrower read would widen the harmless set; it was not measured.
- The saving is three in ten today. It grows as the repository's record grows and shrinks if more tests read record paths — which is exactly what the self-deriving rule is there to make visible.

IMPL-PMAT-237-RECEIPT-END
