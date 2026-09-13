# PMAT-555 — the claims put to the round

forjar 1.30.0 is a release cut: nine PRs since v1.29.0, no Rust changed, and the
whole of the diff is the version, the record of what shipped, and the roadmap
bookkeeping the release gates demanded before they would go green.

The claims put to the three lanes were:

1. The version is bumped everywhere it is declared, and nowhere is left behind.
2. The CHANGELOG's `[1.30.0]` section describes the nine PRs of the window and
   no others, and each behaviour it names is one the window actually shipped.
3. Every behaviour bullet has a crux row naming at least three of the 28
   surveyed systems, and the audit says truthfully how it was produced.
4. The roadmap changes are bookkeeping the release gates require, they are
   confined to rows and fields, and no ceiling in `scripts/ratchets/` is raised.
5. `PMAT-549` is marked completed and bound to `release: 1.30.0` because it
   SHIPPED in this window, not to make a gate green.
6. The dogfood receipt reports the gate lines the run actually printed, and the
   cut log's REDs are reds this branch really had.
7. Nothing in the diff changes forjar's behaviour, so no falsification test is
   owed for a behaviour change, and the receipt says so rather than pointing at
   an unrelated test.

The round reviewed head 81b2aae4 against `main`. Three lanes, review-only,
sandboxed, `writes=false`; models declared and measured
(`gemini-3.1-pro-high`, `gemini-3.6-flash-high`, `gemini-3.8-flash-medium`),
none in the author's family.

## What the round could NOT adjudicate, and who did

The refutations in this cut did not come from the lanes. They came from the
release gates, before the lanes ever ran, and each is recorded in
`docs/audits/logs/PMAT-555-cut.log` with the change that cleared it. That is
recorded here so a reader does not mistake three PASS verdicts for a claim that
nothing was ever wrong with this branch.
