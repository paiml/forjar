# Implementation receipt — PMAT-557 — the v1.30.0 booking

verdict: PASS — v1.30.0, tagged 2026-09-13 and never declared, has its ledger row: cut 2026-09-13T20:52:01Z, ten PRs, eleven tickets, the dogfood and crux receipts, and paiml/forjar-cookbook 60acf9c9, the first cookbook commit that locks forjar 1.30.0. Gate T goes from FAIL on main to PASS on this branch. The same PR repairs what gate T and gate B found on the way: a receipt without its end marker, three shipped rows still open, three labels on the wrong release, and five open issues with no roadmap row. It adds no code.

## Identity

- ticket PMAT-557 (forjar#557, milestone 1.31.0); kind: triage
- branch `PMAT-557-book-v1.30.0`, from `main` @ bfac33cf
- companion PR: paiml/forjar-cookbook#21 (squash 60acf9c9)

## What was measured

    $ bash scripts/dogfood/tagged.sh                       # origin/main
    GATE T FAIL v1.30.0 is reachable from HEAD, at or above the floor v1.25.0,
    and has no row in docs/roadmaps/releases.yaml

## The row

```
  - tag: v1.30.0
    cut: 2026-09-13T20:52:01Z
    prs: [532, 536, 538, 539, 541, 543, 544, 545, 551, 556]
    tickets: [PMAT-240, PMAT-520, PMAT-531, PMAT-533, PMAT-534, PMAT-535, PMAT-537, PMAT-540, PMAT-542, PMAT-549, PMAT-555]
    dogfood: docs/audits/dogfood-1.30.0-receipt.md
    crux: docs/audits/crux-1.30.0.md
    cookbook: 60acf9c948f51d346b088e784e15fd0341be3575
next:
  tag: v1.31.0
  due: 2026-09-15T20:52:01Z
```

Written by `scripts/release-goal.sh cut v1.30.0 --next v1.31.0`, except the
cookbook field. The script copied 0be3e1ec from the previous row, and that
commit locks forjar 1.29.0. The v1.30.0 cut never bumped the cookbook, so no
commit locking 1.30.0 existed until paiml/forjar-cookbook#21 made one.

## Gate T, in the order it refused

| run | refusal | repair |
|---|---|---|
| 1 | v1.30.0 has no row | the cut |
| 2 | `dogfood-1.30.0-receipt.md` does not end with `DOGFOOD-1.30.0-RECEIPT-END` | marker appended; the receipt ends on a complete sentence and has one `verdict:` line |
| 3 | the cookbook at 0be3e1ec LOCKS 1.29.0 | cookbook#21, then the row names 60acf9c9 |
| 4 | PMAT-562 merged since v1.30.0 without `release:v1.31.0` | `release-goal.sh sync` |
| 5 | — | GATE T PASS: 8 releases, 53 tickets, 2 in the open window |

## Gate B's CB-21xx ratchet

| check | main | this branch | ceiling |
|---|---|---|---|
| CB-2112 | 37 (ISSUE-CLOSED 3) | 34 | 35 |
| CB-2114 | 34 | 34 | 34 |
| CB-2115 | 53 | 45 | 43 |

PMAT-555, PMAT-547 and PMAT-562 shipped. Marking them completed clears
ISSUE-CLOSED and the matching ORPHAN-ROADMAP findings. #558, #559, #561, #566
and #567 get rows minted from their issues, carrying `release: 1.32.0` on a new
milestone, which clears five ORPHAN-GITHUB findings without growing
CB-2114. CB-2115 reaches 42 when the rows for #560, #564 and #565 land with
PRs #568, #569 and #570. Until then gate B stays red, and the 1.31.0 cut waits
for that.

## Routing and dispatch

| phase | route (route.sh, verbatim) | executor |
|---|---|---|
| 1 booking | orchestration — `release-goal.sh cut`, sync, textual row edits | self |
| 2 review | `route=agy-quorum w=1.00 basis=absent effort=1[U]` | delegate, quorum ×3 |
| 3 receipt, push, PR | `route=self w=100.00 basis=absent` | self |

| dispatch | agent | turns | maxTurns | lanes / conversations |
|---|---|---|---|---|
| PMAT-557/ph2.delegate | a98bcbc8 | 18 | no | conv-b1c1f2ee FAIL, conv-ef7beb3c PASS, conv-670f4dad PASS |

slots used: 1 of 3 · denials: 0.

## Verification (claimed vs re-run)

| check | lanes | re-run here |
|---|---|---|
| `tagged.sh` | UNMEASURED (gh 401 in the sandbox) | PASS |
| `release-goal.sh window v1.30.0` | UNMEASURED | matches the row |
| cut = tag creatordate | all three: yes (git) | yes |
| b3b04c1f's commit message false | lane 1: yes | no — it promises the replacement that d68fa31a makes |
| CB-21xx counts | not run | 34 / 34 / 45 |

## Gaps, named

- **The skill's kind gate and this repository's disagree.** The
  paiml-implement `kind-gate.sh` refuses a triage branch that touches
  `docs/roadmaps/releases.yaml`. Since PMAT-226, `scripts/quorum-gate.sh`
  admits the release ledger on the triage rail. The repository's rule governs
  the merge. The skill's rule is stale and is recorded here rather than worked
  around.
- **Gate B is still red here** (CB-2115 45 > 43) until the three code PRs merge.
- **The cookbook was bumped after the tag.** Gate T requires the cookbook to be
  bumped as part of the cut, before the tag, and the 1.30.0 cut did not do
  that. This booking names the commit that locks 1.30.0 and cannot make it
  predate the tag.

IMPL-PMAT-557-RECEIPT-END
