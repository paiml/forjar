# Quorum evidence — PMAT-226 — adjudicated claims

One round of three sandboxed `--mode plan` lanes, base pinned at 191770fa, the diff at 568dc169: 1/3 PASS, 2/3 FAIL. Every code claim (C1..C4, C6..C10, C12) was confirmed by all three lanes. The two refutations are corrections to the claim TEXT — C5's count and C11's wording — and neither changes a byte of the diff, so the corrected statements stand below and the diff was not re-judged. Every lane claim was re-run by the orchestrator before it was recorded.

## CONFIRMED

1. [rail] THE RAIL NAMES THE LEDGER BY FILE — `on_rail` admits `docs/audits/**`, `docs/roadmaps/roadmap.yaml`, `docs/roadmaps/releases.yaml` and `.quorum/**`, and any other path is refused by name in the gate's own output.
- evidence: tests/falsification_quorum_gate_has_a_triage_shape.rs:44 is `RELEASES`, the ledger path the positive case `a_triage_receipt_over_the_release_ledger_is_on_the_rail` adds (the whole gate exits 0 over that diff); the negative case `another_file_under_roadmaps_is_still_off_the_rail` writes `docs/roadmaps/notes.md` and asserts the refusal names it; scripts/quorum-gate.sh:384-386 is the predicate. Confirmed by lanes 1, 2 and 3; the orchestrator ran the test binary (8 passed).

2. [red-on-main] THE POSITIVE CASE IS RED AGAINST MAIN'S GATE — with main's scripts/quorum-gate.sh in place the binary reports 7 passed, 1 failed, the failing test the new one; at HEAD 8 passed.
- evidence: tests/falsification_quorum_gate_has_a_triage_shape.rs:44 (the path the failing case adds) and :173 (the fixture that adds it); measured by the orchestrator with the script stashed (docs/audits/logs/PMAT-226-gate-tests.log) and reproduced by lanes 1, 2 and 3 in their clones with `git checkout 191770fa -- scripts/quorum-gate.sh`.

3. [fixture] THE FIXTURE REFACTOR CHANGES NO EXISTING CASE — `also: Option<(&str, &str)>` replaces `touch_code: bool`; the code-edit shape is the constant `CODE_EDIT` and still edits `src/lib.rs`; the six pre-existing cases pass unchanged.
- evidence: tests/falsification_quorum_gate_has_a_triage_shape.rs:46 (CODE_EDIT), :173 (the signature), :201 (the single extra write); the refused-by-name case at :437 still names `src/lib.rs`. Lanes 1, 2 and 3.

4. [bump] THE VERSION BUMP IS COMPLETE — Cargo.toml, Cargo.lock's forjar entry and README's two caret requirements; `cargo metadata --locked --offline` succeeds; nothing else in the tree names 1.27.0 as current.
- evidence: Cargo.toml:3, Cargo.lock:1171, README.md:96 and README.md:98; gate D's version-claim arm PASS on the bumped tree (docs/audits/dogfood-1.28.0-receipt.md). Lanes 1, 2 and 3.

5. [census] THE INTRO'S CENSUS IS MEASURED — nine PRs merged since v1.27.0 (#493 #494 #496 #498 #500 #502 #504 #505 #507), nine tickets, every roadmap row labelled `release:v1.28.0`; #494 is test-only.
- evidence: CHANGELOG.md:12 opens the census; `gh pr list --state merged --search 'merged:>=2026-09-08T22:10:01Z'` returns exactly those nine (lane 1 and the orchestrator ran it); `scripts/release-goal.sh sync --check` prints `already` for every ticket; gate T's `9 of 9 PR(s) … carry release:v1.28.0`. Lanes 1, 2 and 3.

6. [crux] EVERY [V] CITATION RESOLVES AND GATE H PASSES 9 OF 9 — each row of docs/audits/crux-1.28.0.md cites a line at 568dc169 that supports the claim beside it and names at least three roster systems.
- evidence: docs/audits/logs/PMAT-226-gate-H.log (`GATE H PASS 9 of 9 …`); the citation table in docs/audits/impl-PMAT-226-receipt.md, every line opened by the orchestrator; lanes 1, 2 and 3 ran the gate in their clones and opened the files.

7. [corrections] THE THREE CORRECTIONS THE CRUX METHOD CLAIMS ARE REAL — the word `signature` appears nowhere in scripts/quorum-gate.sh or scripts/quorum_evidence.py (the receipt is bound by `diff_sha256` and blob hashes); PR #498 left `machine_is_local` untouched; the rail reads `touched` from git, not from the receipt.
- evidence: lane 1 grepped both scripts; lanes 2 and 3 read PR #498's diff and the rail block at scripts/quorum-gate.sh:371-395.

8. [mutation-arm] NO src/ FILE CHANGES ON THIS BRANCH — the mutable set gate F mutates is empty, so its mutation arm passes by the rule scripts/dogfood/coverage.sh states for a tests-only diff, and the line floor is still measured.
- evidence: `git diff --name-only main...HEAD -- 'src/*.rs'` is empty; the gate F line in docs/audits/dogfood-1.28.0-receipt.md. Lanes 1, 2 and 3.

9. [comment-only] scripts/quorum_evidence.py CHANGES IN A COMMENT ONLY — two `#` lines in the CIT_RE_TRIAGE comment; no behaviour changes.
- evidence: the diff hunk; lanes 1, 2 and 3.

10. [gate-t] GATE T REPORTS THE CUT IN FLIGHT — `GATE T PASS … cut in flight: Cargo.toml is at 1.28.0`, the designed T6 state for a bumped tree before its tag, not a pass by accident.
- evidence: scripts/dogfood/tagged.sh:302-303 (the in-flight arm); measured by the orchestrator (docs/audits/logs/PMAT-226-gate-T.log) and by lanes 1, 2 and 3.

## REFUTED

11. [changelog-count] AS WORDED: "[1.28.0] carries the four paragraphs [Unreleased] held plus FIVE new ones" — lane 3 refuted the count: six paragraphs are new under [1.28.0], because the non-bold introduction is new too.
- evidence: CHANGELOG.md:10 (the heading), CHANGELOG.md:12 (the introduction, not bold, no gate H key), CHANGELOG.md:88, CHANGELOG.md:101, CHANGELOG.md:113, CHANGELOG.md:123 and CHANGELOG.md:132 (the five new bold paragraphs). Lanes 1 and 2 confirmed the five; lane 3 counted six and is right about what it counted.
- corrected: [1.28.0] carries the four bold paragraphs [Unreleased] held, FIVE new bold behaviour paragraphs (each a gate H key), and ONE new non-bold introduction, which gate H does not key. All three lanes compared each bold paragraph to its PR body (#500, #496, #493, #498) and to the code and found them accurate.

12. [row-wording] AS WORDED: "the PMAT-226 roadmap row carries … orch:fable with an orch-basis: token" — lanes 2 and 3 read `orch-basis: token` as a literal and found the row says `orch-basis:release`.
- evidence: docs/roadmaps/roadmap.yaml, the PMAT-226 row (`kind: code`; labels kind:code, orch:fable, release:v1.28.0; notes opening `orch-basis:release`); model-gate.sh printed `decision=admit basis=file` for it.
- corrected: the row's notes open with the token `orch-basis:release`, one of the five spellings model-gate.sh accepts (`orch-basis:(Q3|M>=3|state|release|stop)`). The claim meant "a token of that form" and said it badly; the row is as the corrected text states, and lane 1 confirmed exactly that.
