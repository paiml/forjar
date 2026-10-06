# Lanes — forjar#688

Two counted rounds, 14m budget each. Three lanes per round from two
families, none the author's model (claude-opus-5-5). Each lane was
read-only and had the full diff and the ticket.

Round 1, head c19c6151: NOT AGREED.

- claude-sonnet-5: FAIL. The JIDOKA dependency skip at
  `src/core/executor/machine_b.rs` counts the resource as failed and writes
  no run row, so the commit's "every failure path" claim was overstated;
  `Skipped` was declared and never written.
- agy gemini-3.1-pro-high, sandboxed: PASS.
- claude-haiku-4-5: PASS, no findings.

Answered by f17a11c5: the skip writes `Skipped { reason }`, `fail()`'s
comment names the three paths it covers, and the falsifier gained the skip
case (red with machine_b.rs at c19c6151).

Round 2, head f17a11c5: AGREED, 3/3 PASS.

- claude-sonnet-5: PASS, no findings.
- agy gemini-3.1-pro-high, sandboxed: PASS; its three notes restate the
  change (the Failed variant, the write in `fail()`, the tests).
- claude-haiku-4-5: PASS, no findings.

Noted, not changed: a dependency skip is `skipped` in meta.yaml while
apply's `resources_failed` counts it, so meta's `summary.failed` can be
lower than apply's failed count by the number of skips. The record now
says which resources failed and which were skipped for it; before, the
skip was absent.
