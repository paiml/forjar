# PMAT-576 — the agy rounds

Every round is three sandboxed `agy` quorum lanes, review-only, in their own
clones with push removed, `writes=false`, against `main` at 8285616f. The
per-round table in `book-1.31.0-lanes.md` is the ONLY place this receipt counts
rounds: no other file restates the number, because a count repeated in several
files goes stale in all but one — which four separate lanes caught during the
1.31.0 cut, and which this receipt was written to avoid repeating.

Models: `gemini-3.1-pro-high`, `gemini-3.1-pro-low`, `gemini-3.6-flash-high` —
three distinct ids, none in the author's family (the author is `opus`/claude).

## What a lane can and cannot rule on here

A lane reads a diff and has no `gh` auth, so it cannot ask GitHub which PRs
merged in the window, cannot run `scripts/dogfood/tagged.sh` against the live
API, and cannot resolve the cookbook commit. On a ledger booking that is most of
the evidence, so the lanes are the weaker half of the adjudication and this
receipt says so rather than implying three independent confirmations of facts
the lanes could not reach.
