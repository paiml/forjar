# PMAT-574 — the claims put to the round

forjar 1.31.0 is a release cut: three behaviour PRs since v1.30.0 (plus two that
change none), no Rust changed, and the whole of the diff is the version, the
record of what shipped, and the roadmap bookkeeping the release gates demanded
before they would go green.

The claims put to the three lanes were:

1. The version is bumped everywhere it is declared — `Cargo.toml`, `Cargo.lock`
   and the README's two dependency lines — and nowhere is left behind.
2. The CHANGELOG's `[1.31.0]` section describes the PRs of this window and no
   others, and each behaviour it names is one the window actually shipped.
3. Every behaviour bullet has a crux row naming at least three of the 28
   surveyed systems, and the audit says truthfully how it was produced.
4. The roadmap changes are bookkeeping the release gates require, they are
   confined to rows and fields, and no ceiling in `scripts/ratchets/` is raised.
5. `PMAT-557`, `PMAT-560` and `PMAT-564` are marked completed because their PRs
   MERGED in this window, not to make a gate green; `PMAT-565`, `PMAT-572` and
   `PMAT-573` are minted under their own issue numbers carrying
   `release: 1.32.0` because they are NOT in this release.
6. The dogfood receipt reports the gate lines the run actually printed, and the
   cut log's REDs are reds this branch really had.
7. Nothing in the diff changes forjar's behaviour, so no falsification test is
   owed for a behaviour change, and the receipt says so rather than pointing at
   an unrelated test.
8. PMAT-565 (#570) is excluded from this release, and the exclusion is STATED —
   in the crux document, in the impl receipt and in the ticket's own
   `release: 1.32.0` — rather than left for a reader to notice.

The round reviewed head 203d8a65 against `main` (9884334a). Three lanes,
review-only, sandboxed, `writes=false`; models declared and measured
(`gemini-3.1-pro-high`, `gemini-3.8-flash-high`, `gpt-oss-120b-medium`), three
distinct ids, none in the author's family.

## What the round could NOT adjudicate, and who did

A lane reads the diff. It cannot run `make dogfood-release`, cannot ask GitHub
what merged, and cannot measure the ratchet. Those refutations came from the
gates and the hooks, before the lanes saw the branch, and
`.quorum/evidence/release-1.31.0-judges.md` names them as the refuting
authority.
