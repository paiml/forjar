# PMAT-522 — pmat measurements

## Vacuity

`pmat analyze vacuous-tests` over the whole tree: **400 of 19650 `#[test]` fns
cannot fail (2.0%) across 2164 parsed files; 3 more skip silently when a fixture
is missing.** None of the five cases in
`tests/falsification_comply_count_cannot_run_inside_itself.rs` appears.

The scan does not catch the vacuity that mattered here. One of those cases
asserted on the SCRIPT'S TEXT rather than its behaviour — a test of spelling,
which runs, can fail, and proves nothing about the cap. Three review lanes
caught it and the analyzer did not, which is worth knowing about the analyzer.

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
| cap with a stubbed `ps` reporting 1 | 513 — the script's own fork fails, which is the proof it took effect |
| `ps` unable to answer | exit 4, no count: the cap fails CLOSED |

## The ratchet, still held

| check | measured | ceiling |
|---|---|---|
| CB-2110 | 49 | 49 |
| CB-2111 | 49 | 49 |
| CB-2112 | 34 | 34 |
| CB-2114 | 34 | 34 |
| CB-2115 | 43 | 43 |

CB-2115 was lowered to 42 on a WORKING-TREE measurement taken mid-edit and put
back to 43, which is what the committed tree measures. Gate B was red against
its own baseline in between, and a review lane named it. The rule is now in the
baseline: a ceiling may only be lowered from a measurement of the committed
tree, because that is the tree the gate reads.

Three tickets have been minted under the new convention — issue first, id tail
= issue number, on the release milestone, bare `release:` — and no ceiling has
been RAISED.

## bashrs

`bashrs lint scripts/ratchets/comply-count.sh`: **0 errors**.

## Gates on this branch

`GATE B PASS … ratchet CB-2110=49/49 CB-2111=49/49 CB-2112=34/34 CB-2114=34/34
CB-2115=43/43 held …`

`GATE T PASS 6 tagged release(s) … 14 ticket(s) from 11 PR(s) merged since
v1.28.0 carry release:v1.29.0; due 2026-09-12T16:07:14Z`

`GATE A PASS 11 of 11 merged PR(s) since v1.28.0 carry a harness receipt`

## The roster changed underneath this work

At 18:34, while this branch was being finished, all six CB-21xx checks left the
comply roster. `pmat --version` read 3.40.0 before and after.

| | 14:00 | 18:34 |
|---|---|---|
| banner | `commit: 5db342d5`, `worktree: clean` | `commit: unknown` |
| roster size | 172 | 166 |
| CB-2110..CB-2115 | present | absent |
| CB-148 | "RETIRED — superseded by CB-2110" | live and passing |

The binary at `~/.cargo/bin/pmat` has an mtime of 18:34 the same day. Gate B is
RED under it, correctly — a ceiling cannot be asserted against a tool that does
not run the check — and Arm 7 now names the cause as a TOOL rather than
accusing an id of rotting.

This also broke one falsification case that named `CB-2110` directly. It
discovers a countable check from the installed tool now, which is the right
dependency: the subject of that case is the guard, not the roster.
