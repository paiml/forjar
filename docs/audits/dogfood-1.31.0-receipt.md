# Dogfood receipt — forjar 1.31.0

verdict: GO — all nine gates (A B C D E F G H T) measured green in one `make dogfood-release`
run, exit 0, on a committed tree at 9e3a4744. No gate was red during the run; the three that
were red while the cut was being prepared are recorded in `docs/audits/logs/PMAT-574-cut.log`
with the change that cleared each.

## The gates, as the scripts printed them

| gate | line |
|---|---|
| A | `GATE A PASS 5 of 5 merged PR(s) since v1.30.0 carry a harness receipt` |
| B | `GATE B PASS comply clean; ruleset 13878864 requires [gate]; 14 gate script(s) and 4 other tracked script(s) at 0 bashrs errors; ratchet CB-200 held; ratchet CB-2110=49/49 CB-2111=49/49 CB-2112=34/35 CB-2114=34/34 CB-2115=42/43 held; required check(s) [gate] reach a dogfood gate; legacy bashrs errors 1 <= 1` |
| C | `GATE C PASS 211 CLI name(s), 12 MCP tool(s), 12 HTTP verb(s); every declared name is live and every live name declared; matches docs/audits/surface_audit.csv; CLI/verb/MCP agree on the fixture` |
| D | `GATE D PASS 18 documented invocation(s) from README.md run or parse against the fixtures (1 pinned as known-broken above); version claims reconcile with Cargo.toml; 98 cookbook config(s) validate` |
| E | `GATE E PASS 5 of 5 merged PR(s) since v1.30.0 carry a quorum receipt` |
| F | `GATE F PASS line coverage 96.43% >= 95%; forjar-contracts 0 failed / 44 ignored (39 aprender-corpus annotations; both exactly as recorded); no .rs differs from origin/main, so there is nothing to mutate` |
| G | `GATE G PASS 43 contract(s) validate; pv lint 0 errors; 4 kernel(s) carry equations and kani harnesses; every contract names a falsifier; citations resolve; 12 verb(s) and 12 resource kind(s) reconciled against the corpus (11 unanchored, 2 ungoverned, both at their recorded ceiling)` |
| H | `GATE H PASS 3 of 3 behaviour bullet(s) under [1.31.0] reconciled in docs/audits/crux-1.31.0.md, each naming >= 3 of the 28 surveyed systems` |
| T | `GATE T PASS 8 tagged release(s) since v1.25.0 reconcile with git and GitHub and 53 ticket(s) carry their tag and say they shipped; 5 ticket(s) from 5 PR(s) merged since v1.30.0 carry release:v1.31.0; cut in flight: Cargo.toml is at 1.31.0` |

## Findings for the orchestrator

**Gate F's mutation arm is vacuous for this cut, and says so.** No `.rs` differs
from origin/main — the cut changes a version string, a CHANGELOG heading, an
audit document and the ledger — so there is nothing to mutate. The gate prints
that rather than reporting a mutation pass it did not earn. The three behaviours
this release ships were each mutation-tested on their own branch (#563, #569,
#568) and those receipts are what carries that evidence.

**Gate B measured CB-2115 at 42; one earlier working-tree reading said 43.** The
first draft of this receipt explained that gap as "the receipts this commit had
not yet added", and a review lane refuted it: a markdown file cannot move
ORPHAN-ROADMAP, ORPHAN-GITHUB or DRIFT. What is measured is that
`comply-count.sh` returned 43 at 05:00Z, that gate B returned 42 on the committed
tree, and that a re-measurement at 06:40Z returned 42 with a composition that
sums (24 + 8 + 10). `pmat comply` reads GitHub live and stamps every run with its
own snapshot timestamp, so those are two measurements of a moving source rather
than one number and an error. Two ceilings could be lowered from the committed
measurement; this cut lowers neither, because the tree that carries these numbers
is the one the NEXT commit makes.

**Three reds were cleared before this run, not during it.** Gate H's rows did not
match the keys it greps (backticks inside the key span), gate B's ratchet was
three checks over its ceilings, and the commit-msg hook refused a cut commit
whose ticket said `completed`. Each is in `docs/audits/logs/PMAT-574-cut.log`
with the measurement either side. A receipt that reported only the green run
would be reporting the easy half.

**What this receipt does not cover.** The tag, the GitHub release, crates.io,
docs.rs and the cookbook bump all happen after this commit merges;
`make release-check` (gate R) is the instrument for those, and the ledger
booking PR records what it measured. Nothing is published until that gate is
green too.

DOGFOOD-1.31.0-RECEIPT-END
