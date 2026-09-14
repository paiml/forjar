# PMAT-234 — adjudicated claims

One round of three sandboxed agy quorum lanes: 1 PASS, 2 FAIL. The refutation
was re-run by the orchestrator before it was acted on, and it reproduced.

## CONFIRMED

1. [buckets] The split into `note_pretag` (the tag, the GitHub release,
   crates.io, docs.rs) and `note_pending` (everything else) covers every call
   site, and none moved to the wrong side.
   - evidence: all three lanes enumerated the five `note_` call sites and
     assigned each independently, and all three assignments agreed with the
     script: four in Arm 1, whose obligation does not exist until the tag does,
     and one in Arm 6, whose condition is the `Cargo.toml` version and is true
     or false regardless of any tag.
   - evidence: driven from both sides, so a call site moved to the wrong bucket
     turns a case red rather than merely changing a sentence — the published
     case at
     `tests/falsification_release_check_says_what_is_pending.rs:27` and the
     pre-tag case at
     `tests/falsification_release_check_says_what_is_pending.rs:92`.

2. [no-fail-became-a-note] The rewording touches the sentence and not the
   verdict: no `fail` call was deleted, weakened, or turned into a note.
   - evidence: all three lanes diffed every `fail` invocation in the script
     against `origin/main`'s and found none changed, and none of the conditions
     guarding them touched. One lane looked specifically for a state where the
     old script exited non-zero and the new one exits 0 and reported finding
     none.
   - evidence: the cheapest way for a rewording to become a hole is for a FAIL
     to start reading as a note, so
     `tests/falsification_release_check_says_what_is_pending.rs:116` drives the
     published state with `doc_status: false` and asserts red.

3. [nothing-hidden] `also pending:` reports the other set rather than dropping
   it, and never sits beside a claim that the crux document is present when it
   is not.
   - evidence: the lanes traced both branches of the verdict. When the tag is
     not cut, the pre-tag list is printed and the other set follows it. When
     the tag is cut, the second list REPLACES the `; <CRUX> present` clause
     rather than being appended to it, which is the state where a stale clause
     would have been a second false claim in the same line.

4. [the-cases-falsify] The three original cases go red against `origin/main`'s
   script and green at HEAD, for the reasons they name.
   - evidence: each lane rebuilt the harness in its OWN temp directory rather
     than reading the assertions, and each watched the published case fail on
     the old script because the output carries `pre-tag`, and pass on the new
     one. `tests/falsification_release_check_says_what_is_pending.rs:27`.
   - evidence: the orchestrator's own red/green over both the fixture and the
     REAL repository is in `docs/audits/logs/PMAT-234-pretag.log`, which is the
     stronger measurement of the two because the real repository is the tree
     the defect was found on.

5. [extraction] Moving the fixture to `tests/release_check_fixture/mod.rs` left
   all nine existing cases behaving identically.
   - evidence: all three lanes diffed the moved code against what it was and
     reported only formatting and added `pub(crate)` modifiers; one compared
     the parsed shape rather than the text. The nine cases still run and pass
     from `tests/falsification_dogfood_release_check_pr_window.rs:1`, which was
     440 lines against a 500-line ratchet before the move.

6. [stubs] The `cargo` and `curl` stubs placed first on `PATH` answer what the
   real tools answer, so the published fixture cannot pass where the real
   script would fail.
   - evidence: one lane ran `cargo search` itself and compared its output to
     the stub's, confirming the `name = "x.y.z"    # description` shape the
     script's awk takes field 3 of; another checked that the docs.rs stub's
     body carries the `doc_status` key the script reads with jq. PATH is the
     honest injection point because the script resolves both tools by name and
     offers no environment variable for either.

## REFUTED

1. [false-note] That the one remaining note says something true. It asserted
   the crux document was MISSING.
   - evidence: refuted by two of three lanes, unasked — the brief did not put
     this claim to them. Re-run by the orchestrator before it was accepted:
     `docs/audits/crux-1.28.0.md` is present and 13,860 bytes, and the gate
     printed `no docs/audits/crux-1.28.0.md: Cargo.toml is still at v1.28.0's
     version`. A verdict line being fixed for saying false things had kept one
     of its own, in the same sentence.
   - corrected: the note now says no crux document is OWED and reports whether
     one exists as a measurement — `is owed (it is present)` or `(it is
     absent)`. `tests/falsification_release_check_says_what_is_pending.rs:57`
     drives both directions, with the document written and then removed, so
     neither reading is baked in.
