# PMAT-574 — the pmat instruments this cut was measured with

| tool | what it said |
|---|---|
| `scripts/release-goal.sh show` | `v1.31.0 ██████████ 55h/48h left=-7h · 5 merged, 2 tagged · due 2026-09-15T20:52:01Z basis=docs/roadmaps/releases.yaml:L86 window=v1.30.0..HEAD(9884334a)` — the cut is seven hours past its declared due instant, measured rather than excused |
| `scripts/release-goal.sh sync` | `already PMAT-547`, `labelled PMAT-557`, `labelled PMAT-560`, `already PMAT-562`, `labelled PMAT-564` — three of the five window tickets were missing `release:v1.31.0`, which gate T would have refused |
| `make release-check` | `GATE R PASS pre-tag: 6 PR(s) since v1.30.0 (GitHub reports 6 merged in that window, 0 of them after this HEAD) all carry receipt=ok; PENDING until the tag is cut` |
| `make dogfood-release` | nine gates, exit 0, on the committed tree at 9e3a4744 — `docs/audits/dogfood-1.31.0-receipt.md` |
| `pmat comply check` | before: CB-2112 37 (NO-ISSUE 24, TAIL-MISMATCH 10, ISSUE-CLOSED 3), CB-2114 36 NO-RELEASE, CB-2115 49 (ORPHAN-ROADMAP 27, ORPHAN-GITHUB 11, DRIFT 11). after: 34 / 34 / 43 in the worktree, and gate B measured 34 / 34 / 42 on the committed tree |
| `pmat work sync --check-only` | named the three issues with no roadmap row (#565, #572, #573) and the one row whose title had been truncated at mint (PMAT-559 vs #559) |
| `pmat work add --github-issue N` | minted PMAT-565, PMAT-572, PMAT-573 so each id's tail IS its issue number — the convention `cb21xx-baseline.json` says keeps the ratchet flat when new work is filed |
| `analyze_vacuous_tests` | see below |
| `pmat analyze vacuous-tests` | 432 of 19790 `#[test]` fns cannot fail (2.2%) across 2192 parsed files; 3 more skip silently when a fixture is missing |

## The vacuous scan, read honestly

2.2% is the same proportion the 1.30.0 cut measured (432 of 19736). This cut
adds no test, so the number could not have moved for a reason this branch
caused; it is recorded because a receipt that only reports the numbers that
moved is a receipt you cannot use to notice drift.

## The quality gate

`GATE B PASS comply clean; ruleset 13878864 requires [gate]; 14 gate script(s)
and 4 other tracked script(s) at 0 bashrs errors; ratchet CB-200 held; ratchet
CB-2110=49/49 CB-2111=49/49 CB-2112=34/35 CB-2114=34/34 CB-2115=42/43 held;
required check(s) [gate] reach a dogfood gate; legacy bashrs errors 1 <= 1`
