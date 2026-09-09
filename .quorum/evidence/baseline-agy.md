# Quorum evidence — PMAT-219 — agy lanes

Three review-only lanes through `paiml-agy-delegate`, `--mode plan`, sandboxed, `agy` 1.1.28, exit 0 on all three, `partial=false`.

Two things the delegate did that changed the result:

**It pinned the base by SHA rather than by name.** Local `main` was behind `origin/main`, so a `--shared` clone resolving `origin/main` would have shown the lanes an extra commit — the recorded PMAT-044 defect. The reviewed diff was exactly `3a8b415e..836554f6`.

**It forbade cargo in the lanes** after measuring 213G of build output against 220G free, citing the incident where a lane filled the disk. It then said so in its receipt rather than letting the absence pass unremarked, and flagged that every digest-identity claim was reasoned from source and never executed. That flag is why the crux got measured instead of adjudicated between two confident lanes.

It also reported one brief item as UNCOVERED, then re-read the tree itself and explained that the miss was a token mismatch rather than a coverage gap — the lanes had addressed the item under a sibling function name. Reporting the artifact verbatim and the correction separately is the right shape: the reduce output is not edited to look complete.
