# PMAT-520 — pmat measurements

## The pre-publish gate

`make dogfood-release` exits **0** on a clean tree. All nine gate lines are
quoted verbatim in `docs/audits/dogfood-1.29.0-receipt.md`.

| gate | headline |
|---|---|
| A | 12 of 12 merged PRs carry a harness receipt |
| B | comply clean; CB-200 held; CB-2110=49/49 CB-2111=49/49 CB-2112=34/34 CB-2114=34/34 CB-2115=43/43 held |
| C | 211 CLI names, 12 MCP tools, 12 HTTP verbs reconciled |
| D | 18 documented invocations; 98 cookbook configs validate |
| E | 12 of 12 merged PRs carry a quorum receipt |
| F | line coverage **96.43% ≥ 95%** |
| G | 40 contracts validate |
| H | 12 of 12 behaviour bullets reconciled |
| T | 15 tickets from 12 PRs carry `release:v1.29.0`; cut in flight |

## Vacuity

`pmat analyze vacuous-tests`: **400 of 19650 `#[test]` fns cannot fail (2.0%)
across 2164 parsed files; 3 more skip silently.** This cut adds no test; the
falsifier it names, `falsification_crux_gate_reads_the_release_section`, is not
in that list.

## The window, enumerated rather than counted from memory

Gate A lists all twelve PRs by number and receipt: #509, #510, #511, #513,
#514, #515, #516, #517, #518, #519, #523, #524. Fifteen tickets: PMAT-227,
228, 229, 230, 231, 232, 234, 235, 236, 237, 238, 239, 241, 521, 522.

The CHANGELOG said thirteen and sixteen until a review lane counted. The
thirteenth is this cut's own PR, which has not merged.

## The source footprint

Twelve `.rs` files differ from v1.28.0; **zero** under `src/`. They are all
falsification tests. That is why gate F's mutation arm measures a zero: there
is nothing to mutate, which is a measurement and not a skip.

## The ratchet, across four new tickets

PMAT-520, 521, 522 and 526 were minted during this window under the convention
PMAT-521 established — the GitHub issue first, the roadmap id's tail IS the
issue number, the issue on the release milestone, `release:` the bare version
string. **No ceiling was raised.** CB-2115 fell from 44 to 43 along the way.

## The tool moved underneath

`pmat --version` read 3.40.0 throughout, with three different builds installed:

| banner | roster |
|---|---|
| `commit: 5db342d5`, `worktree: clean` | 172 checks, CB-2110 family present |
| `commit: unknown` | 166 checks, family absent, CB-148 live again |
| `commit: d76e533e`, `worktree: clean` | 172 checks, family back |

172 − 6 = 166 exactly; CB-148 is in both rosters, printed as
`RETIRED — superseded by CB-2110` in the first. A lane called this arithmetic
impossible and it does reproduce as stated.
