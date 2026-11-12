# Implementation receipt — PMAT-608 — github_release missing-asset message

verdict: PASS for the change as written; build and test were not run by the author of this receipt (see Not measured).

## Identity

- ticket PMAT-608 (forjar#608, epic #696); kind: code
- PR #910, branch `s-forjar-3/608`, change head 71b162b
- files: `src/resources/github_release.rs` (+54 −2). The #456 `build.rs` hunk was dropped from this PR (it collides with #909), so #456 is not closed here.

## Before / after

- Before: the asset-lookup pipeline died silently under `pipefail` when no asset matched, so the "no asset matching" message never printed.
- After: the pipeline ends in `|| true` so a missing asset reaches its message; a curl failure aborts and surfaces as a curl failure, not as "no asset".

## Falsifier

- `fj608_missing_asset_prints_its_message` (fake curl, empty asset list): red on main, green on the branch (per the PR body).
- `fj608_curl_failure_surfaces_not_no_asset`: a failing curl must not print the no-asset message; fails if `|| true` swallows the curl error.
- Revert path: restore the old pipeline line.

## Not measured

- No compile or test run for this receipt. Author evidence is the PR body: `cargo test --lib github_release` 26 pass on intel; full suite not run. The tests added after that body (curl failure) were not re-run here.
- CI on the pushed head is the execution of record.
- The `.quorum/` review bundle is written by agy and is not part of this commit.

IMPL-PMAT-608-RECEIPT-END
