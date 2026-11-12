# Implementation receipt — GH-667 (with GH-669, GH-936) — disk-budget never deletes what it cannot prove dead

verdict: PARTIAL — code, tests and the dogfood-guards set are green on kfocus; CB-200 reads 653 on this branch against 652 on plain main and ceiling 651 (this branch adds one function below grade A: `reaper.rs::reach_block`, A-, complexity 9), and the clean-room run has not been done.

## Identity

- tickets GH-667 (forjar#667), GH-669 (forjar#669), GH-936 (forjar#936), epic GH-668; PR #944; kind: code
- branch `lane-944` (PR head f65a1816) merged with `origin/main` @ dbd1ec3e (merge b709521d, no conflicts)
- files (PR): `src/resources/disk_budget/detect.rs`, `reaper.rs`, `reaper/tests.rs`, `tests/execution.rs`, `tests/reaper_cost.rs`, `tests/common/budget_harness.rs`, `tests/falsification_budget_unreachable.rs` (new); this lane adds `CHANGELOG.md`, `docs/roadmaps/roadmap.yaml` (rows GH-667 and GH-669 filled in: acceptance criteria, labels, status inprogress; no row added) and this receipt

## Why dogfood-guards was red on the PR head (five whys)

1. CI job `dogfood-guards` on f65a1816 (run 42893994302) exited 101 with `error: no test target named falsification_coverage_gate_mutation_scope`.
2. `cargo test --test <name>` was given a target that does not exist in the checked-out merge ref.
3. #973 (530762cc) deleted that test file but left its name in the `dogfood-guards` command in `.github/workflows/ci.yml`.
4. Nothing checked that every `--test` name a workflow passes names a file under `tests/`, so the deletion and the workflow edit could land separately.
5. The PR was green-checked before #973 and re-run after it, so it inherited main's broken workflow; it is red on plain main for the same reason from #973 until #982 (5b8f8123, "dogfood-guards stops naming the test #973 deleted"). Not caused by this PR.

Fix: none needed in this branch. #982 is an ancestor of the merged tree (`git merge-base --is-ancestor 5b8f8123 HEAD`), and every target the workflow now names was run on kfocus, all green (below).

## Evidence (kfocus, 32 threads, load 1.25 at start, via `fc-pre.sh pr-944`, rust toolchain of the host)

- `cargo test --locked` on the nine dogfood-guards targets named in ci.yml: all ok (4, 4, 19, 9, 4, 4, 9, 13, 14 passed), rc=0; run twice (before and after the roadmap/CHANGELOG edit), same verdicts.
- `cargo fmt --all -- --check`: rc=0 (twice).
- `cargo clippy --all-targets -- -D warnings`: rc=0.
- `cargo test --lib`: 13789 passed, 0 failed, 4 ignored.
- Integration targets: `falsification_budget_unreachable` 6 passed; `falsification_disk_budget` 11 passed; `falsification_budget_dry_run` 7 passed (it is listed as red on plain main; it was green here, not re-run on main); `falsification_cb21xx_ratchet_holds_the_ceiling` 9 passed; `falsification_no_tracked_file_is_gitignored` 3 passed; `falsification_quorum_anchors_release_shaped` 4 passed; `falsification_release_goals_are_measured` 18 passed; `falsification_quorum_gate_has_a_triage_shape` 8 passed; `falsification_release_goal_cut_books_the_tag` 2 passed.
- Red-then-green for the PR's own new tests was not re-measured by this lane (the PR commits carry them); not measured.

## Known reds, not fixed here

- `falsification_untyped_error_sites_ratchet::the_live_tree_holds_both_ceilings`: 790 against ceiling 778 on this branch, run twice (both rc=101), and 790 on a plain `origin/main` tree (dbd1ec3e). Not run by any CI job; the PR adds 0.
- CB-200 (`bash scripts/cb200-ratchet.sh`, pmat 3.42.0 on kfocus): 653 on this branch, 652 on plain `origin/main` dbd1ec3e, recorded ceiling 651. The diff of functions below grade A between the two trees: `detect.rs::prelude` B->C- (12->23), `reaper.rs::script` C+->C (19->23), `rule_block` B (13->14), and new `reaper.rs::reach_block` A- (complexity 9). The baseline file was not edited.

## Against the issues' acceptance text

- #667 asks for `tests/falsification_disk_budget_in_use.rs`; the PR instead carries the same cases as lib execution tests in `src/resources/disk_budget/tests/execution.rs` (a running-binary target survives, a dormant sibling goes, an exhaustive-match test over every `ReclaimKind` for the loop-level check). Equivalent coverage, different file; no file of that name.
- #669 and #936: covered by `tests/falsification_budget_unreachable.rs` (unreachable target deletes nothing; empty /proc and empty maps scans fail closed).

## Open

- CB-200 +1 from `reach_block` (above); the owner of the module should split it or lower complexity.
- Clean-room run: not done.

IMPL-GH-667-RECEIPT-END
