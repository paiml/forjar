# PMAT-549 — the agy rounds

    lane=quorum width=3 writes=false sandbox=true          (both rounds)
    out_dir=.../paiml-implement/agy/PMAT-549/<session>/ph3     round one
    out_dir=.../paiml-implement/agy/PMAT-549/<session>/ph3r2   round two
    judges=3 per round, same schema, same sandbox, read-only

Every lane ran in its own clone, taken from the head under review. Each round
had three review lanes, then three judges. Each judge was given every lane's
findings verbatim and the kill rule. Round two used a new `out_dir`, so round
one's outputs were never overwritten. After every lane the clone was checked:
each was removed byte-identical, and the shared `.git/config` still named the
repository owner, not a lane identity.

    ran: true    rounds: 2    per round: 3 review lanes + 3 judges + 1 more lane
    round one:   FAIL, FAIL, do-not-implement-as-written; judges 3/3 FAIL; 4 of 8 claims killed
    round two:   FAIL, PASS, PASS; judges 3/3 PASS; 8 of 8 confirmed; teamwork PASS

Round one's extra lane was the CRUX: one lane surveyed how Nagios, Prometheus
and Terraform separate "could not measure" from "failed". The judges ruled on
its verdict like any other claim (see the crux digest).

ONE LANE REPORT WAS FALSE, AND THE LANE WAS NOT TO BLAME. The round-two
teamwork lane ended with "LANE ISOLATION VIOLATED — a shared ref changed:
refs/heads/PMAT-549-unmeasured-is-not-drift".
- The branch ref moved at 10:02:26Z, while the lane was still running. The
  orchestrator had committed f5bb1c01 from a different worktree.
- The reflog shows that commit as the only move, authored by the repository
  owner.
- The check compares shared refs before and after a lane and credits every
  change to the lane, so a concurrent commit by the orchestrator looks like a
  lane's write.
- The lane's own clone was removed byte-identical.
- The misattribution is filed as ISSUE_ISOLATION.
