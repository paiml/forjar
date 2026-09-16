# Implementation receipt — PMAT-574 — the forjar 1.31.0 cut

verdict: PASS — 1.31.0 is cut: Cargo.toml/Cargo.lock at 1.31.0, the CHANGELOG's three behaviour paragraphs under `[1.31.0]`, one crux row per bullet naming >= 3 surveyed systems, and the ledger repaired until gate B's arm 7 held on the committed tree (CB-2112 34/35, CB-2114 34/34, CB-2115 42/43). `make dogfood-release` exited 0 with all nine gates green on a committed tree at 9e3a4744; the receipt is `docs/audits/dogfood-1.31.0-receipt.md` and the reds cleared before it are in `docs/audits/logs/PMAT-574-cut.log`. Nothing is published by this PR: the tag, crates.io, the cookbook bump and the booking row follow the merge.

## What is being released

Three PRs since v1.30.0 change behaviour, and all three are one question:
what is a caller entitled to conclude from a run that printed something and
exited 0?

| PR | ticket | behaviour |
|---|---|---|
| #563 | PMAT-562 | `forjar drift` exits 1 on any DRIFTED line, on every run, with no flag |
| #569 | PMAT-564 | `forjar drift` declines — exit 2, the count named — when it inspected none of the resources it was asked about, and never grades a resource from a manifest it was not given |
| #568 | PMAT-560 | a `service` is converged only while the loaded unit executes the declared program (`exec_start` / `exec_sha256`) |

Two of them (PMAT-562, PMAT-564) are the two signatures of paiml/infra#605,
found by an autonomous infra run against the live fleet on 2026-09-15; the
third is the defect that run was looking for when it found them. The window's
other two tickets change no behaviour: PMAT-547 (every CI job runs on the
fleet) and PMAT-557 (the v1.30.0 ledger booking).

`scripts/release-goal.sh show` on this branch:

```
v1.31.0 ██████████ 55h/48h left=-7h · 5 merged, 2 tagged · due 2026-09-15T20:52:01Z
        basis=docs/roadmaps/releases.yaml:L86 window=v1.30.0..HEAD(9884334a)
```

The cut is seven hours past its declared due instant. That is measured rather
than excused: the window's last PR (#568) merged at 22:59Z on 2026-09-15, two
hours after the due instant, and paiml/infra is blocked on the release —
PMAT-607 folds `machines/{yoga,gx10}/forjar-ephemeral.yaml` into the host
manifest and its done-when is a drift verdict 1.30.0 cannot give.

## The diff

- `Cargo.toml` / `Cargo.lock` — 1.30.0 → 1.31.0
- `CHANGELOG.md` — the `[1.31.0]` heading over the three behaviour paragraphs
- `docs/audits/crux-1.31.0.md` — one comparison row per bullet, >= 3 systems each
- `docs/audits/dogfood-1.31.0-receipt.md`, `docs/audits/logs/PMAT-574-cut.log`
- `docs/audits/impl-PMAT-574-receipt.md` — this file. A review lane read the
  list above, found it did not name the receipt doing the listing, and filed it;
  the omission was real and is corrected here. The lane's other half — that the
  file is an unrequested addition — is refuted by `scripts/dogfood/harness.sh`,
  which fails gate A for any merged PR whose ticket has no
  `docs/audits/impl-<ticket>-receipt.md` at HEAD ending in
  `IMPL-<ticket>-RECEIPT-END`
- `.quorum/PMAT-574-release-1.31.0.json` and `.quorum/evidence/release-1.31.0-*.md`
  — the quorum receipt gate E requires, and the round that produced it
- `docs/roadmaps/roadmap.yaml` — the bookkeeping below
- `README.md` — both version lines, 1.30 → 1.31. Gate D passed either way
  (Cargo reads `forjar = "1.30"` as `>=1.30.0, <2.0.0`, which admits 1.31.0),
  so this is the cut keeping the documented version equal to the shipped one
  rather than a gate forcing it

## The bookkeeping is part of the cut, not around it

Gate B's arm 7 refused this branch — CB-2112 37/35, CB-2114 36/34, CB-2115
49/43 — and named every part of the growth. The repairs, all by the convention
`scripts/ratchets/cb21xx-baseline.json` states (a row whose id tail is the
issue number, a milestone, a bare `release:`):

1. `PMAT-557`, `PMAT-560`, `PMAT-564` — merged, their issues closed, their rows
   still `planned`/`inprogress`. Marked `completed`. That single edit is both
   the ISSUE-CLOSED half of CB-2112 and three of CB-2115's ORPHAN-ROADMAPs.
2. `#565`, `#572`, `#573` — filed during the window with no row. Minted with
   `pmat work add --github-issue N`, so each id's tail IS its issue, each on
   the 1.32.0 milestone with `release: 1.32.0`.
3. `PMAT-560`, `PMAT-574` — `release: 1.31.0`.
4. `PMAT-559` — its row's title was TRUNCATED when it was minted, so it no
   longer equalled #559's title and read as DRIFT. Restored from the issue.

Measured after, by the gate, on the committed tree: CB-2112 34 (ceiling 35),
CB-2114 34 (34), CB-2115 42 (43). Nothing lowered.

## What this cut deliberately does not carry

**PMAT-565 (#570) is not in 1.31.0.** The per-machine lock naming its writer is
the third signature of paiml/infra#605 and its PR was still open when the cut
was made — it has a real red of its own (a test file reached the 500-line
file-health limit exactly, and PMAT-565's one added field pushed it to 501). A
behaviour that has not merged owes no crux row, so its ticket carries
`release: 1.32.0` and the crux document says so in its own words rather than
leaving the absence to be noticed.

**The cut ticket stays open.** `PMAT-574` is `inprogress` here. The commit-msg
hook refused the first attempt at this commit because the row said `completed`,
and it was right: a ticket is open in its own PR and the booking PR marks it
completed, exactly as PMAT-557's booking closed PMAT-555.

## What is still owed after this merges

Tag, GitHub release, crates.io publish, docs.rs, the forjar-cookbook bump whose
`Cargo.lock` must PIN 1.31.0, and the ledger booking PR that appends v1.31.0's
measured row, names that cookbook commit and declares v1.32.0. `make
release-check` (gate R) is the instrument for the first four and none of them
is claimed here.

IMPL-PMAT-574-RECEIPT-END
