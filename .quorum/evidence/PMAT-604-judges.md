# Judges — the v1.32.0 booking (PMAT-604)

Round count and heads: see `PMAT-604-lanes.md`.

## CONFIRMED

1. [ledger] C1 — The v1.32.0 row records the seven PRs and eight tickets that git and GitHub report for the window, the tag's own creation instant as the cut, and a cookbook commit whose manifest admits 1.32.0 while its lock pins exactly 1.32.0.
   - evidence: the Claude lane re-derived the PR list twice, the cut from the tag object, and both cookbook files at a8e758ec, and every value matched the row.
2. [trigger] C3 — On a tag push the classifier receives an all-zero previous commit, cannot resolve a base, and falls to its rule that an unmeasured change is code, so every job gated on the classifier still runs when a release is tagged.
   - evidence: the only job-level condition the test accepts is the classifier's, pinned as a constant at `tests/falsification_coverage_runs_on_tagged_releases_only.rs:119` and read from the parsed workflow rather than its text.
3. [opt-in] C4 — The sovereign-ci input and the caller's own tag trigger are each necessary and together sufficient, and all three lanes added that the trigger fires the whole workflow on a tag, which the receipt now states as a cost.
   - evidence: the test refuses a file with only one of the two edits, asserting both from parsed fields at `tests/falsification_coverage_runs_on_tagged_releases_only.rs:195`, and it requires the tag list to be exactly the one pattern at `tests/falsification_coverage_runs_on_tagged_releases_only.rs:168`.
4. [ledger] C6 — The roadmap edits complete the one ticket the last pre-tag PR could not complete itself, open this booking's ticket on the next milestone, and move release labels only off tickets that were outside the measured window.
   - evidence: a lane claimed the fourth label move landed on another row; mapping each roadmap hunk to the id line that owns it puts both hunks inside PMAT-594, so that finding was rejected by measurement.

## REFUTED

1. [prose] C2 — The dogfood receipt claimed that tree identity carries all nine gate results from the branch head to the tagged commit, but three of those gates read git history and live pull request state, and their window did not yet contain the last pull request.
   - corrected: gates A and E were re-run at the tagged commit over the release's full window and both passed seven of seven, and the paragraph now names every gate that reads anything from outside the tree.
2. [test] C5 — The first version of the opt-in check accepted a tag list carrying a negation beside the pattern, any reusable workflow in the ci job, and a job condition that skips on a tag, each of which keeps coverage off a release while looking opted in.
   - corrected: the opt-in check at `tests/falsification_coverage_runs_on_tagged_releases_only.rs:144` now requires the sovereign-ci workflow and the exact tag list, and a new job-gate check at `tests/falsification_coverage_runs_on_tagged_releases_only.rs:122` refuses any condition but the classifier's, with eight mutations measured.
