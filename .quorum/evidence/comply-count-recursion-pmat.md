# PMAT-522 — pmat measurements

## Vacuity

`pmat analyze vacuous-tests` over the whole tree: **400 of 19650 `#[test]` fns
cannot fail (2.0%) across 2164 parsed files; 3 more skip silently when a fixture
is missing.** None of the four cases in
`tests/falsification_comply_count_cannot_run_inside_itself.rs` appears.

## The guard, measured

| invocation | exit | stdout |
|---|---|---|
| `COMPLY_COUNT_ACTIVE=1 … CB-2110` | 3 | *(empty)* |
| `… CB-2110` | 0 | `49` |

An empty stdout on the refusal is the point: a `0` would read as the check
reporting no findings.

## The units

| | |
|---|---|
| processes, this account | 232 |
| threads, this account | 2,424 |
| threads at the peak of one comply run | 2,486 |
| threads one comply run costs | ~134 |
| cap applied | threads + 512 |

## The ratchet, still held

| check | measured | ceiling |
|---|---|---|
| CB-2110 | 49 | 49 |
| CB-2111 | 49 | 49 |
| CB-2112 | 34 | 34 |
| CB-2114 | 34 | 34 |
| CB-2115 | 42 | 42 |

CB-2115 fell from 43 to 42 when PMAT-522 was minted under the new convention —
issue first, id tail = issue number, on the release milestone, bare `release:` —
so the ceiling was lowered again. Three tickets, three measurements, no ceiling
raised.

## bashrs

`bashrs lint scripts/ratchets/comply-count.sh`: **0 errors**.

## Gates on this branch

`GATE B PASS … ratchet CB-2110=49/49 CB-2111=49/49 CB-2112=34/34 CB-2114=34/34
CB-2115=42/42 held …`

`GATE T PASS 6 tagged release(s) … 14 ticket(s) from 11 PR(s) merged since
v1.28.0 carry release:v1.29.0; due 2026-09-12T16:07:14Z`

`GATE A PASS 11 of 11 merged PR(s) since v1.28.0 carry a harness receipt`
