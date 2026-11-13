# Implementation receipt — PMAT-639 — forjar --version names the build commit

verdict: PASS for the change as written; build and test were not run by the author of this receipt (see Not measured).

## Identity

- ticket PMAT-639 (forjar#639, epic #915); kind: code
- PR #908, branch `s-forjar-3/639`, change head 8eae934
- files: `build.rs`, `build_git.rs`, `src/main.rs`, `tests/falsification_build_git_rerun.rs`, `tests/falsification_version_matches_manifest.rs`

## Before / after

- Before: `forjar --version` printed the crate version only (`forjar 1.33.0`); a binary could not be tied to a commit.
- After: `build.rs` embeds `FORJAR_GIT_SHA` (the env var, else the git short sha, else `unknown`), and `forjar --version` prints `forjar 1.33.0 (d064828)`. The build reruns when the branch ref moves, including in a linked worktree, so the sha does not go stale after a commit.

## Falsifier

- `tests/falsification_version_matches_manifest.rs` (extended): the version line carries the sha; red on main, green on the branch (per the PR body).
- `tests/falsification_build_git_rerun.rs`: the rerun-if-changed set names the branch ref in a plain checkout and in a linked worktree. If it names neither, the test fails.
- Revert path: drop the `build.rs` hook; `--version` returns to the crate version alone.

## Not measured

- No compile or test run for this receipt. Author evidence is the PR body: built and tested on intel, `cargo test --lib resources::` 782 pass, 1 fail (`disk_budget` test, fails identically on main). Full suite not run.
- CI on the merged head is the execution of record.
- The `.quorum/` review bundle is written by agy and is not part of this commit.

IMPL-PMAT-639-RECEIPT-END
