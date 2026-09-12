# PMAT-533 — pmat measurements

## The published artifact

| what | measured |
|---|---|
| tag | `bb979f9e` → `20e80f645cc34688415843bd36f2cc66fad6f117` |
| crates.io | 1.29.0, created 2026-09-11T19:56:31.597251Z, 3,876,443 bytes, yanked=false |
| docs.rs | `{"doc_status":true,"version":"1.29.0"}` |
| GitHub release | published 2026-09-11T21:26:27Z, draft=false, prerelease=false, 14 assets |
| `/releases/latest` | v1.29.0, after being v1.25.2 for four releases |

## Gates after the tag

`GATE R PASS` — the full line is quoted verbatim in the release receipt, having
been truncated in its first version and caught by all three lanes.

`make dogfood-published VERSION=1.29.0` exits 0: gates C and D against what
crates.io actually serves, including **98 cookbook configs**.

`GATE T PASS 7 tagged release(s) since v1.25.0 reconcile with git and GitHub
and 42 ticket(s) carry their tag and say they shipped; 2 ticket(s) from 1 PR(s)
merged since v1.29.0 carry release:v1.30.0; due 2026-09-13T19:55:10Z`

## The ratchet

| check | measured | ceiling |
|---|---|---|
| CB-2110 | 49 | 49 |
| CB-2111 | 49 | 49 |
| CB-2112 | 35 | 35 |
| CB-2114 | 34 | 34 |
| CB-2115 | 43 | 43 |

Four tickets were minted during this record's work — PMAT-533, 534, 535 and the
booking's 531 before it — under the convention PMAT-521 established, and **no
ceiling was raised**.

## Vacuity

`pmat analyze vacuous-tests`: 400 of 19650 `#[test]` fns cannot fail (2.0%)
across 2164 parsed files. This PR adds no test — it is `kind: triage`, the diff
touching only `docs/audits/**` and the roadmap — so the falsification is
`not_applicable` with a reason rather than a pointer at someone else's green
test.
