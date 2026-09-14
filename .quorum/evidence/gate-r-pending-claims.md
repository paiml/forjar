# PMAT-234 — the claims put to the lanes

Six claims, one round of three sandboxed lanes. Every claim is anchored in a
file at the branch tip.

1. **The split into `note_pretag` and `note_pending` is complete.** Every
   `note_` call site is in the right bucket and no arm that should note
   something now notes nothing. Five call sites: four in Arm 1 (the tag, the
   GitHub release, crates.io, docs.rs) and one in Arm 6 (the crux document).
   Driven by `tests/falsification_release_check_says_what_is_pending.rs:27` and
   `tests/falsification_release_check_says_what_is_pending.rs:92`.
   CONFIRMED 3/3.

2. **No FAIL became a note.** The rewording touches the sentence, not the
   verdict. Driven by
   `tests/falsification_release_check_says_what_is_pending.rs:116`, which
   drives the published state with `doc_status: false` and asserts the gate is
   red and names it. CONFIRMED 3/3.

3. **`also pending:` hides nothing.** Both lists are printed when both exist,
   and the second replaces the `; <CRUX> present` claim rather than sitting
   beside one that is false. CONFIRMED 3/3.

4. **The cases falsify.** Each lane rebuilt the harness in its OWN temp
   directory and watched `after_a_published_release_the_verdict_never_says_pre_tag`
   go red against `origin/main`'s script and green at HEAD.
   `tests/falsification_release_check_says_what_is_pending.rs:27`.
   CONFIRMED 3/3.

5. **The fixture extraction preserved all nine existing cases.**
   `tests/release_check_fixture/mod.rs:1`, with the cases left in
   `tests/falsification_dogfood_release_check_pr_window.rs:1`. All three lanes
   diffed the moved code and reported formatting and `pub(crate)` only.
   CONFIRMED 3/3.

6. **The `cargo` and `curl` stubs are an honest injection.** One lane ran
   `cargo search` itself and compared the output shape the script's awk parses.
   `tests/release_check_fixture/mod.rs:1`. CONFIRMED 3/3.

7. **Unasked, and refuted 2/3**: Arm 6's note asserted the crux document was
   MISSING. On this repository it is present and 13,860 bytes. Re-run here
   before it was accepted. Corrected, and driven in both directions by
   `tests/falsification_release_check_says_what_is_pending.rs:57`.

The adjudicated tally in `gate-r-pending-judges.md` is **6 CONFIRMED, 1
REFUTED**, counting a claim once rather than once per lane.
