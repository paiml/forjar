# PMAT-521 — pmat measurements

## Vacuity

`pmat analyze vacuous-tests` over the whole tree: **400 of 19650 `#[test]` fns
cannot fail (2.0%) across 2164 parsed files; 3 more skip silently when a fixture
is missing.** None of the nine cases in
`tests/falsification_cb21xx_ratchet_holds_the_ceiling.rs` appears in that list.

The scan runs over the tree rather than one path because a path argument makes
it refuse: it enumerates tracked files to get a denominator.

## The counts, each by its own reproduction command

| check | measured | ceiling |
|---|---|---|
| CB-2110 | 49 | 49 |
| CB-2111 | 49 | 49 |
| CB-2112 | 34 | 34 |
| CB-2114 | 34 | 34 |
| CB-2115 | 43 | 43 |

CB-2115 was recorded at 44 and measured 43 after the two new tickets were bound
to the release milestone, so the ceiling was LOWERED to what the tree measures.
A ratchet's baseline may only shrink, and it should sit where the tree is.

`bash scripts/ratchets/comply-count.sh CB-9999` — a rotted id — exits 1 saying
so, rather than printing the zero that would read as perfection.

## bashrs

`bashrs lint scripts/dogfood/comply.sh` and
`bashrs lint scripts/ratchets/comply-count.sh`: **0 errors** each.

## Gate B, before and after

On `origin/main`: `GATE B FAIL pmat comply check reports failing check(s) other
than CB-200`, naming all five.

At HEAD: `GATE B PASS comply clean; … ratchet CB-200 held; ratchet CB-2110=49/49
CB-2111=49/49 CB-2112=34/34 CB-2114=34/34 CB-2115=43/43 held; …`.

Both in `docs/audits/logs/PMAT-521-gate-b.log`.

## Quality gates at commit

Pre-commit format, complexity, clippy and SATD passed on every commit of this
branch. TDG baseline: 825 files analysed, average 93.7, 442 A+ and 276 A.
