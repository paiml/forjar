# Lanes — quorum.yml's private CARGO_HOME (PMAT-588)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

Three agy lanes per round, three distinct models, read-only, diff and ticket
pasted inline, JSON-schema-constrained verdicts. No Claude lane this time: the
author is Claude and the gate counts distinct lanes; the three model tiers are
the independence available.

| round | head | agy gemini-3.1-pro-high | agy gemini-3.1-pro-low | agy gemini-3.6-flash-low |
|---|---|---|---|---|
| 1 | 30c05ff4 | FAIL | PASS | FAIL (lane error) |
| 2 | 4ec2a9d3 | PASS | FAIL | PASS |
| 3 | 3eaa29f3 | PASS | PASS | FAIL (lane error) |
| 4 | 3eaa29f3 | PASS | PASS | PASS |

## What moved the diff, by round

- **1** — pro-high: the lint exonerated every job in a file once ANY job set a
  private CARGO_HOME; it flagged a hosted job for sharing a file with a
  self-hosted one; and the Rust copy of the rule let a comment mentioning
  `CARGO_HOME:` exonerate a job. All three reproduced; the lint now decides per
  job, and the Rust test no longer carries a second parser.
- **2** — pro-low: the criterion said the test fails with quorum.yml restored to
  main's, but the test only asserted the patched tree. The negative control now
  lives in the test, on a copy of the real workflows.

## Lane errors (refuted by measurement, not by argument)

- Round 1, flash-low: "quorum.yml contains no cache step, so the test's
  `cachers > 0` floor would fail". The rust-cache step is at quorum.yml:76,
  unchanged by the branch and therefore outside the diff the lane was shown; the
  test passed on that head.
- Round 3, flash-low: "the self-test implements 4 cases, not 12". Running
  `scripts/lint-rust-cache-guard.sh --selftest` prints `OK (12 cases)` — four
  numbered originals and eight `case_scan` calls. The round was re-run on the
  same head (round 4) and agreed 3/3.
