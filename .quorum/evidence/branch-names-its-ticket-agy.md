# PMAT-535 — the agy round

One round, three quorum lanes, `writes=false`, sandboxed, dispatched in one
message. `out_dir` keyed by ticket AND session id, `--not-before` pinned to the
dispatch instant so a stale lane set cannot be reduced into a verdict.

Each lane got a different brief — the rule; the trailer reading and the tests;
the receipt and the log as documents — and all three carried the standing
instruction to quote any sentence a reader could check and find false. Every
refutation in the judges digest came from that instruction, including the one
that mattered most: the arm's stated reason was wrong and the true reason was
stronger.

Two operational findings from the round itself:

- **A `git clone --shared` is unusable for a sandboxed lane.** Its
  `objects/info/alternates` points into the parent repository, which the sandbox
  does not reach; lane 3 got `object directory … does not exist` on every git
  command and honestly marked three findings UNMEASURED. A full clone next time.
- **The delegate was cut off by a usage limit before writing its receipt.** All
  three `lane-*.json` were already on disk with `exit 0`; they were read
  directly and nothing was taken from the delegate's own summary, because it
  never wrote one.
