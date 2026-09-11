# Dogfood receipt — forjar 1.29.0

verdict: GO — all nine gates (A B C D E F G H T) measured green in one `make dogfood-release` run, exit 0, on a clean tree; the 95% line floor is green at 96.43% inside llvm-cov; gate F's mutation arm found no `.rs` differing from `origin/main`, which is a measured zero and not a skip (PMAT-216, `cargo mutants` dying on this host, stays open for branches that do change source); gate T reports the cut in flight; gate B's new Arm 7 holds all five CB-21xx ceilings.

## The gates, as the scripts printed them

| gate | line |
|---|---|
| A | `GATE A PASS 12 of 12 merged PR(s) since v1.28.0 carry a harness receipt` |
| B | `GATE B PASS comply clean; ruleset 13878864 requires [gate]; 14 gate script(s) and 4 other tracked script(s) at 0 bashrs errors; ratchet CB-200 held; ratchet CB-2110=49/49 CB-2111=49/49 CB-2112=34/34 CB-2114=34/34 CB-2115=43/43 held; required check(s) [gate] reach a dogfood gate; legacy bashrs errors 1 <= 1` |
| C | `GATE C PASS 211 CLI name(s), 12 MCP tool(s), 12 HTTP verb(s); every declared name is live and every live name declared; matches docs/audits/surface_audit.csv; CLI/verb/MCP agree on the fixture` |
| D | `GATE D PASS 18 documented invocation(s) from README.md run or parse against the fixtures (1 pinned as known-broken above); version claims reconcile with Cargo.toml; 98 cookbook config(s) validate` |
| E | `GATE E PASS 12 of 12 merged PR(s) since v1.28.0 carry a quorum receipt` |
| F | `GATE F PASS line coverage 96.43% >= 95%; forjar-contracts 0 failed / 44 ignored (39 aprender-corpus annotations; both exactly as recorded); no .rs differs from origin/main, so there is nothing to mutate` |
| G | `GATE G PASS 40 contract(s) validate; pv lint 0 errors; 4 kernel(s) carry equations and kani harnesses; every contract names a falsifier; citations resolve; 12 verb(s) and 12 resource kind(s) reconciled against the corpus (11 unanchored, 2 ungoverned, both at their recorded ceiling)` |
| H | `GATE H PASS 12 of 12 behaviour bullet(s) under [1.29.0] reconciled in docs/audits/crux-1.29.0.md, each naming >= 3 of the 28 surveyed systems` |
| T | `GATE T PASS 6 tagged release(s) since v1.25.0 reconcile with git and GitHub and 26 ticket(s) carry their tag and say they shipped; 15 ticket(s) from 12 PR(s) merged since v1.28.0 carry release:v1.29.0; cut in flight: Cargo.toml is at 1.29.0` |

## Findings for the orchestrator

**Gate B was RED on main for part of this window, and not because of this
tree.** pmat 3.40 added six comply checks this repository has never satisfied,
between the 1.28.0 cut and this one. PMAT-521 recorded a ceiling per check and
made gate B own them in a new Arm 7; the line above is that arm holding.

**The check roster is not stable across local builds of the same pmat version.**
Measured within one day: 172 checks with the CB-2110..CB-2115 family, then 166
without it and CB-148 live again, then 172 again — `pmat --version` reading
3.40.0 throughout, with the banner going `commit: 5db342d5` → `commit: unknown`
→ `commit: d76e533e`. Arm 7 is red while the family is absent, which is correct
and is also a gate whose verdict depends on which local build is installed. The
baseline records the build banner for that reason. **This is the single largest
risk to the next cut**: the same gate can be green here and red on another
machine with a different pmat build and an identical version string.

**Gate F's mutation arm measured zero because no `.rs` differs from main.** This
release changes no source file: every one of its sixteen tickets is release
machinery, gates, scripts, tests or the record. That is a measured zero, not a
skip, and it is the same state 1.28.0 was cut in.

**Gate D validates 98 cookbook configs** against the built artifact, and from
this release the ledger row also names the cookbook COMMIT that was (PMAT-241).

## Determinism

Not asserted by a second run in this session. The receipt carries no timestamps
and no durations, so a re-run of the same tree produces the same file; that is
the property, and it was not measured here. `--twice` was not passed.

DOGFOOD-1.29.0-RECEIPT-END
