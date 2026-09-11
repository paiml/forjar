# Release receipt — forjar 1.29.0

verdict: SHIPPED — crates.io serves 1.29.0, docs.rs built it, the GitHub release is published with 14 assets and is now what `/releases/latest` resolves to, and every pre-tag gate was green on the tagged sha. Three steps were taken by hand and each is recorded with its reason: the tag-triggered `Release` run failed on shared-runner infrastructure and was re-run; `binary-release.yml` was dispatched with the tag because it did not fire on the push; and the promotion from prerelease to full release is the operator's step BY DESIGN here, taken after `make dogfood-published VERSION=1.29.0` passed.

## Identity

| field | value |
|---|---|
| version | 1.29.0 (from 1.28.0; minor, though no `src/` file changed — the release is gates, scripts, tests and the record) |
| tag | `v1.29.0` → `bb979f9e`, annotated, on `20e80f645cc34688415843bd36f2cc66fad6f117` |
| main at the cut | `20e80f64` — "release: forjar 1.29.0 — the cut, and the gate its own record needed (PMAT-520, closes #520) (#527)" |
| crates.io | `forjar 1.29.0`, `created_at 2026-09-11T19:56:31.597251Z`, 3876443 bytes, yanked=False — published with `cargo publish --locked` from the tagged tree after a `--dry-run` pass, with the local credentials file |
| docs.rs | `doc_status: true` for 1.29.0 |
| GitHub release | published 2026-09-11T21:26:27Z, not a draft, not a prerelease, **14 assets** |
| cadence | the second release under `cadence_days: 2`. Booked by PMAT-531; the next goal is `v1.30.0`, due 2026-09-13T19:55:10Z |
| window | 12 PRs, 15 tickets — every one carrying `release:v1.29.0` on its roadmap row at the moment it merged |
| cookbook | `7c100454e8f9fb2b5b13f076b773f808071d5e08` — **the first release whose ledger row names the paiml/forjar-cookbook commit it was qualified against** (PMAT-241) |

## Gates on the tagged sha

| gate | command | result |
|---|---|---|
| A–H, T | `make dogfood-release` on `629bb952` (the cut branch, tree clean) | exit 0, all nine green — quoted line by line in `docs/audits/dogfood-1.29.0-receipt.md` |
| F, stated | `cargo llvm-cov --workspace --locked --fail-under-lines 95` inside the gate | **96.43%**; the mutation arm found no `.rs` differing from main on that branch, a measured zero |
| C, D post-publish | `make dogfood-published VERSION=1.29.0` | exit 0 — the surface and the 18 documented invocations measured against **what crates.io actually serves**, including 98 cookbook configs |
| R, post-tag | `scripts/dogfood/release-check.sh` | exit 0 |

Gate R's line, quoted rather than paraphrased:

```
GATE R PASS v1.29.0 is on main and on origin; GitHub release published
(prerelease=false); crates.io serves forjar 1.29.0; docs.rs built the docs;
15 PR(s) since v1.28.0 (of 15 GitHub reports merged in that window) all carry
receipt=ok; also pending: no docs/audits/crux-1.29.0.md is owed (it is present)
```

**That sentence is itself a thing this release shipped.** Before PMAT-234, the
same state printed `PASS pre-tag … PENDING until the tag is cut` about a
release that was tagged, published and rendered — and named a document that
was present as missing.

## Three steps taken by hand, and why each

1. **The tag-triggered `Release` run failed and was re-run.** Its `verify`
   job died on `could not parse/generate dep info … No such file or directory`
   and `failed to run custom build command for zstd-safe`. That is the shared
   cargo registry sweep this fleet already has recorded: sixteen runners share
   one `~/.cargo` and an hourly reaper deletes `registry/src` by mtime under
   live builds. Infrastructure, not the release.
2. **`binary-release.yml` was dispatched with `-f tag=v1.29.0`** because it
   did not fire on the tag push, though it declares `push: tags ['v*']`. Why
   it did not is **NOT MEASURED** and is worth knowing before the next cut: the
   `Release` and `Security Audit` workflows both fired on the same push.
3. **The promotion from prerelease to full release.** `release.yml` un-drafts
   with `--prerelease` by design, leaving promotion to the operator. Taken
   after `make dogfood-published` passed.

## What the release exposed about itself

**GitHub's `/releases/latest` resolved to v1.25.2** while v1.26.0, v1.27.0 and
v1.28.0 were all `prerelease=false draft=false`. Anyone resolving that URL got
a four-version-old binary for days, and gate R was green throughout: it READS
`isPrerelease` and reports it, never asserts it, and never asks what
`/releases/latest` says. Corrected by promoting v1.29.0 with `--latest`;
filed as PMAT-534 (#534).

**The comply check roster is not stable across local builds of one pmat
version** — 172 checks, then 166, then 172, with `pmat --version` reading
3.40.0 throughout. Gate B's new Arm 7 is red while the checks it owns are
absent, correctly, and that means the same tree can be green here and red
elsewhere on an identical version string. The largest standing risk to the next
cut.

## The three PRs, and the fourth

#527 the cut, #532 the booking, and this one the record. A fourth was closed
unmerged: #525 carried the same work on a branch named `release-1.29.0`, and
the quorum gate reads `.quorum/<branch>.json`, so the branch was renamed to
match the ticket the way every other cut's is.

Gate T was red on main between the tag and the booking by design, and red again
after the booking for a reason it named precisely: PMAT-531 carried
`release:v1.29.0` and its own PR merged after the tag, so by the window rule it
is v1.30.0 work. Corrected here.

RELEASE-1.29.0-RECEIPT-END
