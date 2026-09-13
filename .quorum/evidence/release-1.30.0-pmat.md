# PMAT-555 — the tools, and what they measured

## The gate run this receipt rests on

One `make dogfood-release`, exit 0, on the committed tree. Nine gates:

```
GATE A PASS 9 of 9 merged PR(s) since v1.29.0 carry a harness receipt
GATE B PASS comply clean; 14 gate script(s) at 0 bashrs error(s); CB-21xx ratchet holding
GATE C PASS 211 CLI name(s), 12 MCP tool(s), 12 HTTP verb(s); every declared name is live
GATE D PASS 18 documented invocation(s) from README.md run or parse against the fixtures
GATE E PASS 9 of 9 merged PR(s) since v1.29.0 carry a quorum receipt
GATE F PASS line coverage 96.42% >= 95%
GATE G PASS 40 contract(s) validate; pv lint 0 errors; 4 kernel(s) carry equations and kani harnesses
GATE H PASS 9 of 9 behaviour bullet(s) under [1.30.0] reconciled
GATE T PASS 7 tagged release(s) reconcile with git and GitHub; 10 ticket(s) from 9 PR(s) carry release:v1.30.0
```

The full lines are in `docs/audits/dogfood-1.30.0-receipt.md`; the three that
were RED first are in `docs/audits/logs/PMAT-555-cut.log` with the change that
cleared each.

## pmat

`pmat comply check --format json`, before and after the bookkeeping repair:

```
before: CB-2112 35  CB-2114 36  CB-2115 49   (ceilings 35 / 34 / 43)
after:  CB-2112 34  CB-2114 34  CB-2115 42
```

`pmat work sync --check-only` named the orphans by number, which is how the
three the author had just created (#552 #553 #554) were told apart from the
eight that predate this window.

`pmat work sync --direction github-to-yaml` was NOT used, and the baseline says
why: measured dropping 20 of 57 `kind:` fields and reflowing 369 lines. `kind:`
is what the paiml-implement kind-gate reads, so the documented writer silently
removes a gate's input. The `release:` fields in this diff are written textually.

## What was not run, and why

No `cargo test` beyond what the gates run: this cut changes no Rust, which gate
F reports as `no .rs differs from origin/main, so there is nothing to mutate`.
`make dogfood-published VERSION=1.30.0` cannot run before the version exists on
crates.io; it is the step immediately after `make publish-from-tag TAG=v1.30.0`
and it measures the PUBLISHED artifact rather than this tree.
