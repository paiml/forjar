# Implementation receipt — PMAT-913 — PR #972 (runner-smells baseline)

verdict: PARTIAL — PR #972 merges with main (5 conflict hunks in 4 workflow files, all `runs-on` lines) and the 30 integration targets CI runs pass on gx10; the new `runner-smells` job and baseline were not executed here, and the PR title and body name no PMAT ticket.

This file is a companion to `docs/audits/impl-PMAT-913-receipt.md` (PR #981). Gate A reads only `impl-<ticket>-receipt.md` from the ticket id (`scripts/dogfood/harness.sh`), so this file is read by no gate; it records PR #972 separately and leaves the PR #981 receipt untouched.

## Identity

- ticket PMAT-913, kind: ci; PR #972, branch `chore/runner-smells-baseline` (infra#3828)
- files in the PR against main after the merge: `.github/runner-smells.baseline` (new) and 13 workflow files, +66 −0 (`git diff --stat origin/main`, lane worktree)
- CHANGELOG.md gains one Unreleased entry; no source file changes

## Merge with main

- `git merge origin/main`: 11 commits behind, 4 conflicted files (`ci.yml`, `lint.yml`, `msrv.yml`, `proofs.yml`), 5 hunks, every one a `runs-on:` line against a `timeout-minutes:` line.
- Resolution: main's `runs-on` (ARM64 moved to X64 or to either pool, from #981) plus this PR's added `timeout-minutes` and draft-skip `if:` lines. Nothing else was edited.
- After the merge the PR diff is still +66 −0 across the same 14 files: no hunk of this PR is a duplicate of something now on main.

## Measured

- host gx10, `fc-pre.sh pr-972`, tree synced with `.git` from a clone of the merged commit abe6f6a6:
  - `cargo fmt --all -- --check`: rc 0
  - `cargo test --locked --no-fail-fast` over the 30 `--test` targets named in `.github/workflows/*.yml`: rc 0, 30 targets, 0 failed (includes the ones that read workflow files: `falsification_683_cross_mounts_under_work_root`, `falsification_722_cache_key_arch`, `falsification_hosted_jobs_do_not_cache_target`, `falsification_nightly_gate_is_tag_vs_head`, `falsification_nightly_windows_openssl_perl`, `falsification_pr_lane_runs_what_the_change_can_break`, `falsification_pr_lane_selects_the_gate_the_change_can_move`, `falsification_quorum_gate_reads_the_pushed_ref`, `falsification_quorum_gate_reads_the_receipt_store`, `falsification_version_matches_manifest`)

- re-run after the CHANGELOG entry and this file were added: `falsification_version_matches_manifest`, `falsification_crux_gate_reads_the_release_section`, `falsification_pr_lane_runs_what_the_change_can_break`, `falsification_gate_a_the_pr_and_its_commits_name_one_ticket`, `falsification_dogfood_harness_and_quorum`: rc 0, 0 failed.

## Not measured

- clippy and `cargo test --lib`: not run; the PR changes no Rust source.
- the `runner-smells` reusable workflow (paiml/.github, pinned by SHA) and whether the baseline counts equal the scanner's counts on this tree: executed by CI only.
- known LAB reds (forjar#858) were not part of the 30 targets and were not run.

## Open

- The PR title and body name no `PMAT-<n>`; Gate A files a merged PR under a ticket named in its branch, title or body (`scripts/dogfood/harness.sh`). The squash commit would carry none. Needs a title or body edit by someone who may write to the PR.
- The PR adds a `runner-smells` job to `ci.yml` that is not in the `gate` job's `needs`; whether it is a required check is a branch-protection fact not read here.
- The PR adds `if: draft == false` to the `lint.yml` jobs `clippy` and `file-health` and to an `audit.yml` job; a skipped job on a draft is a changed run condition, not an accept condition.

IMPL-PMAT-913-PR972-RECEIPT-END
