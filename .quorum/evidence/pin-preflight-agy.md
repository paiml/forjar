# PMAT-579 / PMAT-581 / PMAT-582 — the agy rounds

Every round is three sandboxed `agy` review lanes, review-only, in their own
clones with push removed, `writes=false`, against `main` at 583a58ea. The
per-round table in `pin-preflight-lanes.md` is the ONLY place this receipt
counts rounds; no other file restates the number.

Round 3's models: `gemini-3.1-pro-high`, `gemini-3.8-flash-high`,
`gemini-3.7-flash-high` — three distinct ids, none in the author's family (the
author is `claude-opus-5`, declared to the lane runner).

## What a lane can and cannot rule on here

A lane reads this diff inside a forjar clone. It can verify the rail, the
quotation from `src/core/planner/unprobed.rs`, the `kind:` label counts in the
roadmap, and that every criterion describes registration rather than work. It
CANNOT reach paiml/infra, where the two measurements behind PMAT-579 and
PMAT-582 were taken: the `forjar history --json` events naming which binary
performed each apply, and the three drift runs over one host that returned 9,
2 and 3. Those were measured by the orchestrator and are marked as such in
`pin-preflight-judges.md` rather than attributed to a lane that could not see
them. The lanes are therefore the binding judges of the diff's shape and its
in-repo citations, and the weaker half on the cross-repo numbers.
