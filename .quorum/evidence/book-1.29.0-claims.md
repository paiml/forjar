# PMAT-531 — the claims put to the lanes

Six claims plus the standing instruction, one round of three sandboxed lanes,
all three FAIL.

1. **Every field of the booked row is what git and GitHub say** — the `cut`
   against the tag's creatordate, the `prs` against the measured window, the
   `tickets` against what those PRs name, the `dogfood` and `crux` paths at
   HEAD. Split 1 CONFIRMED / 2 REFUTED, and the refutation does not reproduce.

2. **The `cookbook:` field is right and is the first one.** CONFIRMED 3/3:
   `7c100454` was the tip of paiml/forjar-cookbook's master at the cut, its
   `Cargo.toml` admits 1.29.0 under the caret rule, and it is the first row
   carrying the field, matching `cookbook_floor`.
   `tests/falsification_cb21xx_ratchet_holds_the_ceiling.rs:1` is not about
   this claim; the cookbook arm's own cases are in
   `tests/falsification_release_cookbook_is_part_of_the_release.rs:1`.

3. **Thirteen PRs in the row where the CHANGELOG says twelve, both right.**
   CONFIRMED 2/3 — the cut's own PR #527 has merged now, so the window includes
   it.

4. **`next` is correct** — v1.30.0 at the cut instant plus `cadence_days`.
   CONFIRMED 3/3.

5. **The label moves are right.** REFUTED 3/3, unanimously, and it reproduces.

6. **This is honestly a `kind: triage` change.** CONFIRMED 3/3 at the time —
   and then made false by this branch's own later work, which is recorded
   rather than left.

And above the six: **quote any sentence or number in the booked row, the commit
message or `docs/audits/crux-1.29.0.md` that a reader could check and find
false.** REFUTED 2/3 with findings that reproduce.

The adjudicated tally in `book-1.29.0-judges.md` is **4 CONFIRMED, 4 REFUTED**.
