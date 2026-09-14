# Dogfood receipt — forjar 1.30.0

verdict: GO — all nine gates (A B C D E F G H T) measured green in one `make dogfood-release` run,
exit 0, on a committed tree at 825f04b1. Three of them were RED first, on this
branch, and each red is recorded in `docs/audits/logs/PMAT-555-cut.log` with the change that cleared it.

## The gates, as the scripts printed them

| gate | line |
|---|---|
| A | `GATE A PASS 9 of 9 merged PR(s) since v1.29.0 carry a harness receipt` |
| B | `GATE B PASS comply clean; ruleset 13878864 requires [gate]; 14 gate script(s) and 4 other tracked script(s) at 0 bashrs errors; ratchet CB-200 held; ratchet CB-2110=49/49 CB-2111=49/49 CB-2112=34/35 CB-2114=34/34 CB-2115=42/43 held; required check(s) [gate] reach a dogfood gate; legacy bashrs errors 1 <= 1` |
| C | `GATE C PASS 211 CLI name(s), 12 MCP tool(s), 12 HTTP verb(s); every declared name is live and every live name declared; matches docs/audits/surface_audit.csv; CLI/verb/MCP agree on the fixture` |
| D | `GATE D PASS 18 documented invocation(s) from README.md run or parse against the fixtures (1 pinned as known-broken above); version claims reconcile with Cargo.toml; 98 cookbook config(s) validate` |
| E | `GATE E PASS 9 of 9 merged PR(s) since v1.29.0 carry a quorum receipt` |
| F | `GATE F PASS line coverage 96.42% >= 95%; forjar-contracts 0 failed / 44 ignored (39 aprender-corpus annotations; both exactly as recorded); no .rs differs from origin/main, so there is nothing to mutate` |
| G | `GATE G PASS 40 contract(s) validate; pv lint 0 errors; 4 kernel(s) carry equations and kani harnesses; every contract names a falsifier; citations resolve; 12 verb(s) and 12 resource kind(s) reconciled against the corpus (11 unanchored, 2 ungoverned, both at their recorded ceiling)` |
| H | `GATE H PASS 9 of 9 behaviour bullet(s) under [1.30.0] reconciled in docs/audits/crux-1.30.0.md, each naming >= 3 of the 28 surveyed systems` |
| T | `GATE T PASS 7 tagged release(s) since v1.25.0 reconcile with git and GitHub and 42 ticket(s) carry their tag and say they shipped; 10 ticket(s) from 9 PR(s) merged since v1.29.0 carry release:v1.30.0; cut in flight: Cargo.toml is at 1.30.0` |

## Findings for the orchestrator

**Gate B went RED on this branch and the growth was mine.** CB-2114 measured 36
against a ceiling of 34 and CB-2115 49 against 43, because six GitHub issues were
filed during this window — three of them the follow-ups PMAT-549 disclosed — and
none of them had the three things the ratchet's own baseline says a new ticket
needs: a roadmap row whose id tail is the issue number, a milestone, and a
`release:` field. That is the ratchet doing exactly the job it was written for,
against the author of the cut, within an hour of the cut starting.

Repaired rather than raised: #546 #547 #550 #552 #553 #554 have rows and the
1.31.0 milestone; #555 has 1.30.0; PMAT-549's row said `planned` while its issue
was closed, which is what made it an ORPHAN-ROADMAP, and it now says `completed`
with `release: 1.30.0`. Measured after: CB-2112 34 (ceiling 35), CB-2114 34 (34),
CB-2115 42 (43). Two of the three can be lowered and this receipt does not lower
them — a ceiling may only be lowered from a measurement of the COMMITTED tree,
and the tree that carries these numbers is the one this receipt is committed in.

**Gate T went RED for a reason worth keeping.** `PMAT-549 merged since v1.29.0
and its roadmap row does not carry release:v1.30.0`. The label is written when a
PR MERGES, not when the cut is made, and #551 merged into this window a few hours
before the cut started. `scripts/release-goal.sh sync` is the only writer and it
labelled the one ticket that was missing while reporting the other nine as
already correct — which is the shape a sync should have: nine `already`, one
`labelled`.

**Gate H went RED because the CHANGELOG's bullets were the wrong SHAPE.** The
window's entries were written as markdown list items (`- **Title.**`) and gate H
counts only a bold span that OPENS a paragraph. It was right to refuse: the
release would otherwise have shipped nine behaviour changes with no crux
comparison while a gate that exists to require one printed nothing. Converting
the nine to paragraphs made them visible, and `docs/audits/crux-1.30.0.md` now
carries a row for each, naming at least three of the 28 surveyed systems.

**What gate F did NOT measure.** `no .rs differs from origin/main, so there is
nothing to mutate` — this cut changes no Rust. The mutation arm is therefore
vacuous here by construction, and says so rather than reporting a pass it did
not earn.

**The crux audit's provenance is different from 1.29.0's and says so.** The
1.29.0 comparison was surveyed by an `agy` quorum lane. This one was written by
the release orchestrator from documentation memory, every third-party claim
marked `[X]`. Recorded in the audit's own Method section, because the difference
between "a lane surveyed this" and "the author wrote it" is a provenance claim,
and this repository's whole thesis is that unstated provenance is where false
records come from.
