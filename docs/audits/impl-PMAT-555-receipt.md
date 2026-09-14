# PMAT-555 — the forjar 1.30.0 cut

## What is being released

Nine PRs since v1.29.0. One changes what forjar SAYS about a machine it could
not reach (#551, PMAT-549: a query the target never answered is UNMEASURED, not
drift). The other eight are the gates and records that decide when a cut may
happen at all.

`make release-goal` at 586dba26 before the cut:

```
v1.30.0 █████████░ 46h/48h left=1h · 10 merged, 9 tagged · due 2026-09-13T19:55:10Z
        basis=docs/roadmaps/releases.yaml:L79 window=v1.29.0..HEAD(586dba26)
```

`make release-check`: GATE R PASS — v1.29.0 is on main and on origin, its GitHub
release is published, crates.io serves 1.29.0, docs.rs built the docs, and all
23 PRs of the previous window carry `receipt=ok`.

## The diff

- `Cargo.toml` / `Cargo.lock` — 1.29.0 → 1.30.0
- `README.md` — both version lines. Line 96 was at 1.29 and line 98 at **1.28**:
  the 1.29.0 cut moved one and left the other, so the library-only example has
  been a version behind the binary example since that release. Corrected here
  rather than carried.
- `CHANGELOG.md` — the `[1.30.0]` section, nine behaviour paragraphs
- `docs/audits/crux-1.30.0.md` — one comparison row per bullet
- `docs/audits/dogfood-1.30.0-receipt.md`, `docs/audits/logs/PMAT-555-cut.log`
- `docs/roadmaps/roadmap.yaml` — the bookkeeping below

## The bookkeeping is part of the cut, not around it

Gate B refused this branch: CB-2114 36 against a ceiling of 34, CB-2115 49
against 43. The growth was mine — six issues filed during this window with no
roadmap row, no milestone and no `release:` — and the ratchet named it by check
and by count.

`scripts/ratchets/cb21xx-baseline.json` states the convention that keeps the
counts flat when new work is minted, and all three parts were missing:

1. the GitHub issue is created FIRST and the roadmap id's tail is the issue
   number (`PMAT-552` for issue 552) — otherwise every new ticket is one
   TAIL-MISMATCH, and a ceiling that must rise per ticket is a treadmill;
2. the issue goes on a release MILESTONE, because `release:` is projected from
   it — otherwise every new ticket is one NO-RELEASE;
3. `release:` is the bare version string.

Applied: #546 #547 #550 #552 #553 #554 → rows + milestone 1.31.0 + `release:
1.31.0`; #555 → milestone 1.30.0; PMAT-549 → `status: completed` and `release:
1.30.0`, because its issue is closed and a `planned` row against a closed issue
is exactly what ORPHAN-ROADMAP means. The `release:` fields are written
TEXTUALLY: `pmat work sync --direction github-to-yaml` is the documented writer
and the baseline forbids it here, having been measured dropping 20 of 57 `kind:`
fields — and `kind:` is what the paiml-implement kind-gate reads.

Measured after, on the committed tree: CB-2112 34 (ceiling 35), CB-2114 34 (34),
CB-2115 42 (43). **No ceiling is raised and none is lowered here.** Lowering is
the only edit the baseline permits and it may only be done from a measurement of
the committed tree; this tree is that measurement, so the lowering belongs to the
next commit that can cite it, exactly as the baseline's own history records.

## Gates

Nine of nine green in one `make dogfood-release` run, exit 0. Three were red
first and every red is in `docs/audits/logs/PMAT-555-cut.log` with the change
that cleared it: gate B (the ratchet, above), gate T (`PMAT-549 … does not carry
release:v1.30.0` — a PR is labelled when it MERGES, and #551 merged into this
window hours before the cut began), and gate H three times over — the nine
entries were list items where the parser counts only paragraph-opening bold; then
the comparison document did not exist; then one row named two surveyed systems
where the floor is three.

## Disclosed

**The crux audit was not surveyed by a lane.** 1.29.0's was. This one was written
by the release orchestrator from documentation memory, with every third-party
claim marked `[X]`, and its Method section says so in the first paragraph. It is
a weaker source than an independent survey and is labelled as one.

**Gate F's mutation arm is vacuous for this cut** — no `.rs` differs from
origin/main, so there is nothing to mutate. The gate says this itself rather than
reporting a pass it did not earn.

**Three issues filed in this window are follow-ups this release does NOT fix**:
#552 (a drift run exits 1, not the drift class 10), #553 (`remote_path_digest`
outside the unmeasured reader), #554 (the quorum gate's hash covers a file the
quorum itself writes). All three are on the 1.31.0 milestone. #554 was hit while
cutting this release: committing the review verdict moved the hash the `.quorum`
receipt binds, so the receipt went stale for a reason nobody touched.

IMPL-PMAT-555-RECEIPT-END
