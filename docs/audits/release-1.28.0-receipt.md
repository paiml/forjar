# Release receipt — forjar 1.28.0

verdict: SHIPPED — crates.io serves 1.28.0, docs.rs built it, the GitHub release is published with 14 assets, and every pre-tag gate was green on the tagged sha. Two of the last three steps were done by hand and both are named below with the defect that made them necessary: the workflow declined to un-draft a release it had not created (PMAT-232), and promotion from prerelease to full release is the operator's step by design, taken after `make dogfood-published VERSION=1.28.0` passed.

## Identity

| field | value |
|---|---|
| version | 1.28.0 (from 1.27.0; minor, because `api::ProbeMap` joins the supported surface and `drift` starts measuring resources it used to skip) |
| tag | `v1.28.0` → `931ee2eb`, annotated, tagger date 2026-09-10T16:07:14Z, on `cdcc0e80a17a95158fbf9693556df81149020401` |
| main at the cut | `cdcc0e80` — "release: forjar 1.28.0 — the first cut under the two-day cadence, and the triage rail admits the release ledger (PMAT-226) (#508)" |
| crates.io | `forjar 1.28.0`, `created_at 2026-09-10T16:12:45Z`, 3,822,649 bytes, not yanked — published from a detached worktree of the tag by `make publish-from-tag TAG=v1.28.0` after a `DRY_RUN=1` pass, with the local credentials file |
| docs.rs | `doc_status: true` for 1.28.0 |
| GitHub release | published 2026-09-10T23:56:51Z, not a draft, not a prerelease, **14 assets** — 6 tarballs, 6 `.sha256`, `SHA256SUMS` and `install.sh`, all uploaded by the workflow |
| cadence | the first release under `docs/roadmaps/releases.yaml`'s `cadence_days: 2` (PMAT-225). Booked by PMAT-227; the next goal is `v1.29.0`, due 2026-09-12T16:07:14Z |
| window | 10 PRs, 10 tickets: PMAT-215, 217, 219, 220, 221, 222, 223, 224, 225, 226 — every one carrying `release:v1.28.0` on its roadmap row before the cut |

## Gates on the tagged sha

| gate | command | result |
|---|---|---|
| A–H, T | `make dogfood-release` on `568dc169` (the cut branch, tree clean) | exit 0, all nine green — quoted line by line in `docs/audits/dogfood-1.28.0-receipt.md` |
| F, stated | `cargo llvm-cov --workspace --locked --fail-under-lines 95` inside the gate | 96.45%; the mutation arm found no `src/` change to mutate, a measured zero (PMAT-216 unchanged) |
| R, post-tag | `scripts/dogfood/release-check.sh` | PASS — 13 PRs since v1.27.0 all carry a receipt; the tag is on origin, the release is published, crates.io and docs.rs serve it |
| C, D on the published artifact | `make dogfood-published VERSION=1.28.0` | PASS — 211 CLI names, 12 MCP tools, 12 HTTP verbs live in the crate crates.io serves; 18 documented invocations run; 98 cookbook configs validate |

`docs/audits/logs/PMAT-231-release-check.log` holds both runs verbatim.

## What the release itself broke, and what fixed it

The first tag-triggered run got as far as the binaries and then died in `dist-artifacts`:

```
/tmp/SHA256SUMS already exists (use `--clobber` to overwrite file or `--skip-existing` to skip file)
```

`/tmp` persists between jobs on the clean-room runners, and that file was left by an earlier release. `publish-release` was skipped and the release sat as a draft prerelease with thirteen assets and no installer. The same class of defect had been failing `homebrew` since at least v1.27.0, two steps earlier, unnoticed because that job blocks nothing.

**PMAT-230** (#510) fixed it: `--clobber` on every `gh release download` (never `--skip-existing`, which would exit 0 leaving the previous release's checksums for `forjar dist --checksums-file` to embed in `install.sh`), and `rm -rf` on the two other fixed `/tmp` paths the workflow writes into. Rules 9 and 10 in `tests/falsification_release_workflow_fixed_paths.rs` hold both, red against the workflow as it was, with twelve mutation cases measured one at a time.

A re-run could not pick that up — GitHub re-runs a run against the workflow file of its original commit — so the release was re-dispatched from `main` (`gh workflow run release.yml --ref main -f tag=v1.28.0`). Run `34530301814`: every binary green, `checksums` green, and **the step that had died reported success**, uploading `install.sh` as the fourteenth asset.

## The two steps taken by hand, and why

1. **Un-drafting.** `publish-release` ran, reported success, and did nothing: its `gh release edit --draft=false` is guarded by `needs.create-release.outputs.created == 'true'`, and this run did not create the release — the earlier, failed run did. So did the assertion that would have caught it. A release whose creating run fails after `create-release` can never be published by the workflow. Filed as **PMAT-232**; the release was un-drafted with `gh release edit v1.28.0 --draft=false --prerelease`.
2. **Promotion to a full release.** This is the operator's step by design — the workflow's own words are *"Promotion to a full release is the operator's step after `make dogfood-published VERSION=` passes"* — and it was taken after that command passed gates C and D against the crate crates.io serves.

## Filed while shipping

| ticket | what |
|---|---|
| PMAT-232 | `publish-release` cannot un-draft a release a previous run created |
| PMAT-233 | the `homebrew` job's tap push has an empty `GH_TOKEN` and fails authentication — reached for the first time by PMAT-230's fixes, and never once successful before |
| PMAT-234 | gate R's verdict line says `PASS pre-tag … PENDING until the tag is cut` whenever ANY arm is pending, so after every successful release the one line an operator is told to read states the opposite of the truth |

All three carry `release:v1.29.0`, applied when they were minted rather than at the next cut. That is the property PMAT-225 put in place and this release is the first to exercise end to end.

## What is not claimed

- `homebrew` has never published a formula for any release, including this one (PMAT-233). It is not on `publish-release`'s `needs`, so it has never blocked one.
- `cargo mutants` remains unmeasurable on this workstation (PMAT-216); the 1.28.0 branch changed nothing under `src/`, so its mutation arm had nothing to measure and said so.
- The clean-room fleet was saturated by other repositories throughout: the first tag run had four jobs cancelled at 16:53:40Z by a runner-side event, and the re-dispatch waited roughly ninety minutes for a `clean-room` runner. Neither is a property of this release.

RELEASE-1.28.0-RECEIPT-END
