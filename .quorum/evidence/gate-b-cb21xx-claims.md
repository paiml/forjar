# PMAT-521 — the claims put to the lanes

Six claims, one round of three sandboxed lanes, all three FAIL. Every claim is
anchored in a file at the branch tip.

1. **This is a ratchet and not a waiver.** Compared against the CB-200
   precedent one arm above and against the rules at the top of
   `scripts/dogfood/comply.sh`. Is anything asserted LESS than before, other
   than the five named checks at or below their recorded counts?
   `tests/falsification_cb21xx_ratchet_holds_the_ceiling.rs:1`.
   REFUTED 2/3 — on the duplicate-id overwrite, not on the shape.

2. **The exemption cannot widen by accident.** The `CB21XX` array is written
   out rather than matched by prefix.
   `tests/falsification_cb21xx_ratchet_holds_the_ceiling.rs:1`.
   REFUTED 2/3 — a check in the array and NOT in the baseline is waived by both
   arms, which is the worst outcome this design had.

3. **Arm 7 fails closed in every unmeasurable direction, and says which.**
   REFUTED 3/3 — it failed closed and said NOTHING, because a
   `sys.exit("message")` writes to stderr where `$( )` takes stdout.

4. **The recorded numbers are the true ones.** Each lane was asked to
   re-measure. REFUTED 3/3 as unmeasurable: this host was inside a GitHub
   secondary rate limit and three of the five counts take a `gh` snapshot.
   CB-2110 and CB-2111 were confirmed at 49 by two lanes.

5. **The nine cases actually falsify.** REFUTED 3/3 — the harness defined
   `CB21XX_BASE` itself and merged stderr into stdout, which would have hidden
   claim 3 exactly.
   `tests/falsification_cb21xx_ratchet_holds_the_ceiling.rs:1`.

6. **CB-2113 is correctly excluded from both lists.** REFUTED 3/3 — the
   reading was right and the branch did not satisfy it, which is a different
   claim and the one that mattered.
   `tests/falsification_cb21xx_ratchet_holds_the_ceiling.rs:1`.

The adjudicated tally in `gate-b-cb21xx-judges.md` is **0 CONFIRMED, 6
REFUTED**, counting a claim once rather than once per lane. A round where every
claim falls is not a failed round; it is the round doing its job on a change
its author had reviewed only against himself.
