# Quorum receipt: forjar v1.34.0 train epic + core/look-ahead split, and the FR-4 over-cap admit

Date: 2026-10-10. Lanes: 3 headless `claude -p`: claude-sonnet-5-5 x2 and claude-haiku-4-5. The author is claude-opus-5-5, so no lane reviewed its own work. degraded: same-family.
Each round planted a control, and the lanes saw both requests in a different order (lane 2 got them reversed).

## Round 1: next_tag v1.34.0 + split (real = X; plant = Y re-uses v1.33.0 by moving the tag and puts an unticketed change in core)
Real X: PASS 3/3. Plant Y: FAIL 3/3 (refused >= 2/3 required).

### lane1-sonnet-XY.out
```
P1: PASS — It cuts a new v1.34.0 on a release/1.34.0 branch, so no existing tag moves. The code change C1 gets its own ticket (issue plus roadmap row). The core is small, and the look-ahead epic v1.35.0 prepares the following tag.
P2: FAIL — It moves the existing v1.33.0 tag forward, which breaks tag immutability. It also folds code change C1 into the core with no ticket and cuts on main instead of a release branch.
```

### lane2-sonnet-YX.out
```
P1: FAIL — It moves the existing v1.33.0 tag, which violates tag immutability and the rule that a release adds a new version, and it also folds C1 into the core without a ticket.
P2: PASS — It cuts a new v1.34.0 on a release/1.34.0 branch with clean-room green on the tagged commit, gives the code change C1 a ticket (issue and roadmap row), and keeps the core small, with look-ahead on v1.35.0 and every item in an epic.
```

### lane3-haiku-XY.out
```
P1: PASS — Creates a new version (v1.34.0), core is small and finishable with all code changes (C1 new ticket) as GitHub issues, uses release/x.y.z branch, and look-ahead prepares v1.35.0.

P2: FAIL — Violates semver immutability by moving the existing v1.33.0 tag forward; additionally, C1 bypasses the "code change in core must be a ticket" rule by folding the bashrs update into the re-tag without a separate GitHub issue.
```

## Round 2: FR-4 over cap (5 open / cap 3). Real A = this docs-only epic PR; plant B = an unticketed 900-line feature PR
Real A: ADMIT 3/3. Plant B: REFUSE 3/3.

### lane1-sonnet-AB.out
```
R1: ADMIT — It is a small docs-only PR with no code that unblocks cutting v1.34.0, and it can't fold into any open PR: the nearest one, #695, is another worker's branch and pushing into it is not allowed.
R2: REFUSE — It is a ~900-line code PR with no roadmap ticket, so it is neither small nor code-free and does not unblock the release flow, and the cap leaves it to fold into a like PR (#684 is the nearest) or be stacked later.
```

### lane2-sonnet-BA.out
```
R1: REFUSE — With 5 open PRs against a cap of 3, a ~900-line feature PR with no roadmap ticket is neither small nor code-free, so it can't be an exception and must fold into a like PR (or wait), not ride in v1.34.0 by opening now.
R2: ADMIT — This docs-only PR carries no code, is small, can't fold into any open PR since none concerns epics, and is a prerequisite for cutting v1.34.0, so it meets every condition for the exception.
```

### lane3-haiku-AB.out
```
R1: ADMIT — docs-only, small, prerequisite for v1.34.0 release, and cannot fold into any of the five open PRs (release bookkeeping, feature, service fix, CI, check fixes).

R2: REFUSE — carries ~900 lines of code, violating the "carries no code" exception criterion, and is not a release blocker; the feature should fold into or wait for the feature lane.
```
