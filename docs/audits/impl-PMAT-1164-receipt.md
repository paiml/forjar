# Implementation receipt — PMAT-1164 (PR #1212, gate T pagination) — post-merge

verdict: PARTIAL — the change is measured on its merge commit `0e209660`; one post-merge lane PASSED, but the post-merge quorum round recorded 2 of 3 lanes FAIL and 0 refuted claims, and no `.quorum/` receipt for the branch is on `main`. Gates A and E are not satisfied by this receipt.

## Identity

- ticket: `PMAT-1164` (issue `#1164`, `Closes #1164` in PR #1212; tracked under `GH-1213`, epic 990)
- pull request: `paiml/forjar#1212`, state `MERGED` at 2026-11-19T18:03:49Z (`gh pr view 1212`)
- head branch: `fix/gate-t-paginate-recut-gh1164`; base `main` @ `9bf45619`
- merge commit on `origin/main`: `0e2096600eb7bc2a9e9252dbbbb6eb50625e0f83` (`0e209660`)
- commit title: `gate T: read the release window past one 200-PR page (#1212)`

## Scope (measured on the merge commit)

`git show --numstat 0e209660`: 9 files, +178 / -18.

| file | +/- |
|---|---|
| `scripts/dogfood/lib/prpage.sh` (new) | +82 / -0 |
| `scripts/dogfood/lib/window.sh` | +8 / -15 |
| `tests/dogfood_gates_harness/mod.rs` | +21 / -0 |
| `tests/falsification_dogfood_pr_window_pagination.rs` (new) | +45 / -0 |
| `docs/roadmaps/roadmap.yaml` (row `PMAT-1164`) | +18 / -0 |
| `.github/workflows/ci.yml` (guard test added) | +1 / -1 |
| `scripts/dogfood/harness.sh` (comment) | +1 / -1 |
| `scripts/dogfood/quorum.sh` (comment) | +1 / -1 |
| `tests/release_goal_fixture/mod.rs` | +1 / -0 |

What the change does: `lib/prpage.sh` reads `merged:FROM..TO` and, when a slice is full (200 PRs), bisects it at its midpoint second and unions the halves by PR number. A read that cannot finish (gh error, invalid JSON, call cap of 40 exceeded, a one-second slice still full, or a missing child row) is `UNMEASURED`, not a pass.

## Gate status on the merge commit

- Gate T: the pagination falsifier (`falsification_dogfood_pr_window_pagination`, 2 tests) is wired into `ci.yml` Guard tests by this merge.
- Gate E: needs `.quorum/fix-gate-t-paginate-recut-gh1164.json` at `HEAD` with `judges >= 3`, `refuters_per_claim >= 3`, `claims_refuted >= 1`. Not on `main`. See the quorum section.
- Gates A and E were not run by this receipt.

## Post-merge review evidence (measured)

1. Post-merge lane, `gemini-4-hydrogen-medium`, lane 1, at `0e209660`:
   `/mnt/nvme-raid0/cop-inbox/handoff/agy/postmerge-forjar-1212-agy-hydrogen-lane1-0e20966.md`
   `VERDICT: PASS`. It reviewed the PR diff and found no gate weakening, no waiver, no ratchet change, and no non-test `unwrap`.

2. Post-merge quorum round, `gemini-4-neon-high`, directory
   `/mnt/nvme-raid0/cop-inbox/handoff/forjar-cop/backfill-1342/rounds-agy/1212/`:
   - lanes: `AGY-lane-1.out` PASS (32 lines); `AGY-lane-2.out` FAIL (19 claims, DEFECT items 17-19); `AGY-lane-3.out` FAIL (26 claims, DEFECT items 24-26).
   - refuters: `AGY-refuter-1..3.out`, each 71 `SURVIVED`, 0 `REFUTED`.
   - judges: `AGY-judge-1..3.out`, each 71 `SURVIVED`, 0 `REFUTED`; each ends `VERDICT: FAIL`.
   - the lane-2 and lane-3 DEFECT claims concern: a `git log` exit path in `prpage.sh:11-15`, a partial-page coverage check in `prpage.sh:48-51`, a loosened test assertion at `falsification_dogfood_pr_window_pagination.rs:31,37-38` (`assert_says("453 of 453")` and the `UNMEASURED` cases), and `prpage.sh:74-75` using `print m + 86400` rather than an integer format.
   - Judges voted SURVIVED on all 71 claims. None was refuted.

3. Pre-merge review lanes (18 files, head commits `53843da`..`0cef38d`): recorded in the prior receipt on branch `origin/dogfood/1213-receipts-1212` (commit `5f96428f`). Verdicts were checked against the files in `/mnt/nvme-raid0/cop-inbox/handoff/agy/review-forjar-1212-*.md`: 6 FAIL at `53843da`, `fce67c2`, `352c30d`; PASS at `0cef38d` (lane 1 and lane 2 and sign-off), with a `VERDICT: PASS` at `2c802ba` and `6b66c79`.

4. Mutation run at `fce67c24`: `/mnt/nvme-raid0/cop-inbox/handoff/agy/mutants-forjar-1212-fce67c2.md` (measured in that file; not re-run here).

## Why this is PARTIAL and not PASS

- The post-merge round is a real round, but it records 0 refuted claims and 2 of 3 lanes FAIL. A quorum receipt built from it cannot satisfy gate E's anti-vacuity floor (`claims_refuted >= 1`). Writing a PASS receipt from it would be a fabricated verdict.
- The lane-2 and lane-3 defects were not resolved on `main` by any measured source in this worktree.
- No `.quorum/` receipt for `fix/gate-t-paginate-recut-gh1164` exists on `main`.

## Not measured here

- `cargo test`, `cargo fmt`, `cargo clippy`, and the gate scripts were not run for this receipt. The 2 pre-merge falsification tests and the lane-2 / lane-3 defect claims are taken from the cited files, not re-executed.
- Gate A and gate E results were not produced.
