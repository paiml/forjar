# Release receipt — forjar 1.26.0

verdict: SHIPPED — crates.io serves 1.26.0, docs.rs built it, the GitHub release is published with 14 assets, and every pre-tag gate was green on the tagged sha.

## Identity

| field | value |
|---|---|
| version | 1.26.0 (from 1.25.2; minor, because `undo` now refuses input it used to accept) |
| tag | `v1.26.0` → `adbaaa9a`, on `818a060d972b2eb9deceb795334afea29dd28f0d` |
| main at the cut | `818a060d` — "release: forjar 1.26.0 — the cut (PMAT-165) (#483)" |
| crates.io | `forjar 1.26.0`, published from a detached worktree of the tag with the local credentials file |
| docs.rs | `doc_status: true` for 1.26.0 |
| GitHub release | published, not a prerelease, 14 assets — the workflow uploaded 13 (6 tarballs, 6 `.sha256`, `SHA256SUMS`) and `install.sh` is the fourteenth, added by hand for the reason below |
| release day rule | `overridden(operator 2026-09-06)` — the Friday-only rule appears in this repo only as historical notes on completed REL-* roadmap rows, never as a live gate, so nothing in the tree needed rewriting |
| clean room | `fleet` — `make -C ~/src/infra/machines/clean-room clean-room-forjar`, source announced as the release worktree at `818a060d`, ten gates A0–B4 all PASSED |

## Gates on the tagged sha

| gate | command | result |
|---|---|---|
| A harness receipts | `scripts/dogfood/harness.sh` | PASS 10 of 10 merged PRs since v1.25.2 |
| B pmat comply | `scripts/dogfood/comply.sh` | PASS; CB-200 ratchet held at 651 |
| C surface | `scripts/dogfood/surface.sh` | PASS 211 CLI, 12 MCP, 12 HTTP; matches the committed ledger |
| D documented claims | `scripts/dogfood/docs.sh` | PASS 18 invocations, 1 pinned known-broken |
| E quorum receipts | `scripts/dogfood/quorum.sh` | PASS 10 of 10 |
| F coverage + mutants | `scripts/dogfood/coverage.sh` | PASS lines 96.39% ≥ 95% |
| G contracts | `scripts/dogfood/contracts.sh` | PASS 40 validate, citations resolve, 12 verbs and 12 kinds reconcile |
| H crux reconciliation | `scripts/dogfood/crux-reconcile.sh` | PASS 3 of 3 behaviour paragraphs under `[1.26.0]` |
| R post-tag | `scripts/dogfood/release-check.sh` | PASS |
| published arm | `make dogfood-published VERSION=1.26.0` | PASS gates C and D against the crates.io artifact; `forjar 1.26.0` |

Binary provenance: the gates resolve their binary through `scripts/dogfood/lib/binary.sh`, which built and asserted `forjar 1.26.0` against the manifest. The brief's inline guard reads `cargo metadata`'s `target_directory`, which on this host resolves to a shared path the shell wrapper does not build into; the gate's own resolution is the authoritative one and is what every measurement above used.

## What shipped

Three user-visible behaviour changes, split into ten behaviours in the CRUX audit and counted as three by gate H:

- every resource-set selector resolves once, through one selection, closed downward over `depends_on` (#472)
- one state dir shared by a fleet of stacks: apply, status and the wrong-stack guard support it; undo refuses while it holds more than one stack (#473)
- a file resource whose path contains `666` or `777` can be applied again, and a world-writable mode is refused at the gate (#477)

## What was done by hand, and why

The release workflow created the draft prerelease exactly as PMAT-166 designed, then its `dist-artifacts` job failed: `forjar dist` resolves checksums from `https://github.com/paiml/forjar/releases/download/<tag>/<asset>`, which GitHub does not serve for a draft. All thirteen build assets were already uploaded. The installer was generated from a detached worktree of the tag with `--checksums-file` — the interface forjar's own error message names — uploaded, and the release promoted. The defect is PMAT-208, fixed in the same PR as this receipt and pinned by RULE 8 of the release-workflow shape test.

## Follow-ups

| id | what | release |
|---|---|---|
| PMAT-162 | generation ownership and stack-scoped restore — the spec is merged, the implementation is not | 1.27 |
| PMAT-167 | the infra pin for 1.26.0 | this release, applied on this host only |
| PMAT-189 to PMAT-207 | the deferrals and findings recorded in `docs/audits/triage-1.26.0.md` | 1.27 |
| PMAT-208 | the dist job's checksum source | fixed here |

RELEASE-1.26.0-RECEIPT-END
