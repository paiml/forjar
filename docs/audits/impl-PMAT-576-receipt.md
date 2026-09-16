# Implementation receipt — PMAT-576 — the v1.31.0 booking

verdict: PASS — v1.31.0, tagged and published on 2026-09-16, has its ledger row: cut 2026-09-16T08:50:38Z, six PRs, six tickets, the dogfood and crux receipts, and paiml/forjar-cookbook 710b0877 — the cookbook commit whose Cargo.toml admits 1.31.0 and whose Cargo.lock PINS it, written in place of the 1.30.0 commit the tooling copies by default. v1.32.0 is declared, due 2026-09-18T08:50:38Z. PMAT-574 is marked completed here, because a ticket is open in its own PR and the next PR closes it. It adds no code.

## Identity

- ticket PMAT-576 (forjar#576, milestone 1.32.0); kind: triage
- branch `PMAT-576-book-v1.31.0`, from `main` @ 8285616f
- companion PR: paiml/forjar-cookbook#22 (squash 710b0877)

## What was measured

    $ bash scripts/dogfood/tagged.sh                       # origin/main
    GATE T FAIL v1.31.0 is reachable from HEAD, at or above the floor v1.25.0,
    and has no row in docs/roadmaps/releases.yaml: a release that was cut and
    never declared

    $ bash scripts/dogfood/tagged.sh                       # this branch
    GATE T v1.31.0 cut 2026-09-16T08:50:38Z: 6 PR(s), 6 ticket(s) labelled release:v1.31.0 ok

## The row

```
  - tag: v1.31.0
    cut: 2026-09-16T08:50:38Z
    prs: [548, 563, 568, 569, 571, 575]
    tickets: [PMAT-547, PMAT-557, PMAT-560, PMAT-562, PMAT-564, PMAT-574]
    dogfood: docs/audits/dogfood-1.31.0-receipt.md
    crux: docs/audits/crux-1.31.0.md
    cookbook: 710b08774517428fe5d9b01e6004443903ce5858
next:
  tag: v1.32.0
  due: 2026-09-18T08:50:38Z
```

Every field is measured rather than declared: `scripts/release-goal.sh window
v1.31.0` reads the PR list from GitHub over the commit range and the ticket list
from those PRs, and `cut` is the tag's own instant.

## The cookbook commit, and why the default was wrong

`release-goal.sh cut` copies the previous row's `cookbook:`. Left alone that
would have named 60acf9c9, whose `Cargo.lock` pins **1.30.0** — and gate T's
cookbook arm exists precisely to refuse that: a requirement is a range, the lock
is what cargo builds, and the first commit any release named locked 1.2.1 while
the release was 1.29.0. paiml/forjar-cookbook#22 was opened, checked and merged
BEFORE this row was written:

    Cargo.toml   forjar = { version = "1.30", … }  ->  { version = "1.31", … }
    Cargo.lock   forjar 1.30.0                     ->  forjar 1.31.0
    cargo check --workspace                            clean

## What this PR closes out

- `PMAT-574` → `status: completed`. The cut could not mark its own ticket: the
  commit-msg hook refused it with `Pmat-Ticket PMAT-574 is completed — work
  belongs to an open item, and CB-2113 refuses this commit in CI`. That rule is
  measured here rather than remembered.
- `release-goal.sh cut` also moved three `release:v1.31.0` labels (PMAT-526,
  PMAT-528, PMAT-529) to `release:v1.32.0`, because those tickets were not in
  v1.31.0's window. A ticket claiming a release it was not in is a fabricated
  link, which is the arm of gate T that catches it.
- `PMAT-576` carries `release: 1.32.0` and the 1.32.0 milestone: a booking PR
  merges into the NEXT window, exactly as PMAT-557 did for 1.30.0.

## The release itself

- tag `v1.31.0` = 8285616f, pushed 2026-09-16
- crates.io: `Published forjar v1.31.0 at registry crates-io`; the API's
  `max_version` reads 1.31.0
- the GitHub Release was created by the tag's workflow as a DRAFT prerelease and
  its binaries were still building when this receipt was written, so
  `repos/paiml/forjar/releases/latest` still resolved to v1.30.0 at that moment.
  `make release-check` (gate R) is the instrument for that arm and it is named
  here rather than claimed: PMAT-534 requires the pointer to resolve to this
  release before the release is finished, and promoting the draft is the
  remaining step.

## What is NOT in this booking

PMAT-565 (#570, the per-machine lock names its writer) missed the cut: its PR
was open when the tag was made, it carries `release: 1.32.0`, and it appears in
no row here. The crux document for 1.31.0 says the same in its own closing
section.

IMPL-PMAT-576-RECEIPT-END
