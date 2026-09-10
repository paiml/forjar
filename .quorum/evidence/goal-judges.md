# Quorum evidence — PMAT-225 — adjudicated claims

Two rounds of three sandboxed `--mode plan` lanes, base pinned at 8e8e79e4, the diff at db298604 (round 1: 3/3 FAIL, C1..C15 confirmed by two lanes and 14/15 by the third, two material findings) and at ca3f8d70 (round 2: 3/3 PASS, lane-reduce agreed, zero dissent). Every lane verdict was re-run by the orchestrator before it counted. The two findings are the REFUTED items below, each with what was corrected.

## CONFIRMED

1. [gate-green] GATE T IS GREEN ON THE COMMITTED LEDGER — `bash scripts/dogfood/tagged.sh` exits 0 on HEAD and prints `GATE T PASS 5 tagged release(s) since v1.25.0 reconcile with git and GitHub and 16 ticket(s) carry their tag; 8 of 8 PR(s) merged since v1.27.0 carry release:v1.28.0`, reading the ledger, the registry, Cargo.toml and the receipts at HEAD.
- evidence: tests/falsification_release_goals_are_measured.rs:18 is the green fixture case asserting the PASS line and its counts; docs/audits/logs/PMAT-225-gate-T.log is the live run. Three lanes in each round ran the gate and quoted the line.

2. [mutation] THE REGISTERED MUTATION TURNS IT RED — changing `86400` to `86401` in the derived due instant prints `GATE T FAIL next.due is declared 2026-09-10T22:10:01Z … puts the due instant at 2026-09-10T22:10:03Z — the declared goal and the derived one disagree` and exits 1 on the real ledger; reverting restores PASS.
- evidence: docs/audits/logs/PMAT-225-gate-T-mutant.log; tests/falsification_release_goals_are_measured.rs:91 pins the same arm with a due instant one second off. Measured by lanes 2 and 3 of both rounds and by the orchestrator.

3. [fixtures] TWELVE CASES, EACH RED FOR ITS OWN REASON — over temp repositories with a bare origin, annotated tags and a stub gh, a declared window that disagrees with GitHub, an unlabelled merged ticket, an unlabelled shipped ticket, a fabricated link, a tag origin lacks, a skewed due instant, an overdue cut, a version no goal names, a missing receipt, a stray id and a gh that cannot answer are each red naming their subject; a gate that exits 1 without its FAIL line fails the case.
- evidence: tests/falsification_release_goals_are_measured.rs:28 (window), :40 (next label), :52 (shipped label), :64 (fabricated), :80 (origin), :102 (overdue), :118 (version), :129 (receipt), :144 (stray), :166 (gh); docs/audits/logs/PMAT-225-gate-tests.log carries `12 passed`.

4. [cut-flow] THE CUT IS BOOKED END TO END — a tag with no row is red naming `release-goal.sh cut`; `cut v0.0.1 --next v0.0.2` writes the measured row, declares next.due exactly cadence_days after the cut, confirms the shipped ticket's label and moves a free-rider's `release:v0.0.1` to `release:v0.0.2`; `sync --check` exits 1 naming the unlabelled open ticket and `sync` labels it; committed, the gate is green and `show` reads `1h/48h left=47h · 1 merged, 1 tagged … ledger=worktree`.
- evidence: tests/falsification_release_goal_cut_books_the_tag.rs:14; tests/release_goal_fixture/mod.rs:145 builds the pre-cut world. Lane 1 of round 1 ran the case; lanes 2 and 3 read it; round 2 confirmed it still passes with `cut` no longer appending the receipt keys.

5. [gate-a-red-on-main] THE TWO GATE A CASES ARE RED AGAINST MAIN'S SCRIPTS — with main's scripts/dogfood/harness.sh and lib/window.sh stashed in, `gate_a_a_first_id_that_is_not_a_roadmap_row_is_named_and_red` and `gate_a_a_stray_id_resolves_through_the_row_that_declares_it_as_alias` FAIL (0 passed, 2 failed); popped back, 19 pass.
- evidence: tests/falsification_dogfood_harness_and_quorum.rs:103 and :119; measured by the orchestrator (the stash) and confirmed by lane 1 of round 1; lane 3 marked it asserted because it could not run cargo, which is why the orchestrator's measurement is the one that counts.

6. [ticket-rule] THE FIRST ID IS THE ADDRESS AND A STRAY ID IS NEVER SKIPPED — `dogfood_pr_tickets` resolves the first id in branch, then title, then body against the registry at HEAD; an id that is no row resolves only through a row labelled `alias:<id>`; a stray first id leaves the address empty and gate A fails naming it, so a forgotten roadmap row cannot fall through to an older ticket in the body whose receipt already exists.
- evidence: tests/falsification_dogfood_harness_and_quorum.rs:103 asserts the refusal names PMAT-997 and `alias:`; the plan grill refuted the skipping rule before any code and all three round-1 lanes confirmed R1 against the shipped rule.

7. [ledger-measured] EVERY LEDGER ROW IS THE TOOL'S OUTPUT — for v1.25.0, v1.25.1, v1.25.2, v1.26.0 and v1.27.0 the row in docs/roadmaps/releases.yaml is byte-identical to `scripts/release-goal.sh window TAG` minus its comment lines: PRs by ancestry, tickets by the one rule, the cut as the tag's creation instant in UTC, the receipt keys from dogfood_floor on. None was typed.
- evidence: five diffs run by the orchestrator (docs/audits/logs/PMAT-225-release-goal.log carries the five rows); lanes 2 and 3 of round 2 re-derived all five by diff; tests/release_goal_fixture/mod.rs:10 pins the same instant rendering in UTC that the fixture once got wrong.

8. [roadmap-diff] THE ROADMAP DIFF IS LABELS AND STAMPS ONLY — `git diff 8e8e79e4..HEAD -- docs/roadmaps/roadmap.yaml` changes only `- release:<tag>` and `- alias:PMAT-218` label lines, `updated:` stamps, `labels: []` to `labels:` on rows that gained a label, and the PMAT-225 row; no YAML round-trip, no re-quoting, no timestamp rewrite elsewhere.
- evidence: the orchestrator's filtered diff printed nothing outside those shapes; lane 3 of round 1 measured the same (C8); tests/release_goal_fixture/mod.rs:145 writes the registry in the shape `pmat work add` writes it, `labels: []` included.

9. [empty-census] AN EMPTY TICKET CENSUS DOES NOT KILL THE TOOL — `bash scripts/release-goal.sh window v1.25.1` prints a row with `tickets: []` and exits 0; the census selects non-empty lines with `awk NF`, because `grep -v '^$'` exits 1 on zero lines and pipefail then kills the caller inside an assignment with no verdict.
- evidence: docs/audits/logs/PMAT-225-release-goal.log shows the v1.25.1 row; docs/audits/jidoka.jsonl carries the row for the death; lanes 1 and 3 of round 1 ran the command (C9).

10. [no-printer] NOTHING SWALLOWS A VERDICT — gate T and the two libraries carry no `|| true` and no `>/dev/null` on a call that can `fail` (fail prints on stdout; a redirected call is a death, measured when every fixture case reported one); the mutations guard passes over ten scripts.
- evidence: tests/falsification_dogfood_scripts_declare_mutations.rs:32 registers tagged.sh in the REQUIRED list the guard checks for strictness and swallowed measurements; docs/audits/jidoka.jsonl rows 1 and 2 record the two deaths the suite caught. All three round-1 lanes hunted R3 and found no printer.

11. [gates-b-g] GATES B AND G PASS — `scripts/dogfood/comply.sh` prints `GATE B PASS … 14 gate script(s) … at 0 bashrs errors`; `scripts/dogfood/contracts.sh` prints `GATE G PASS` with FALSIFY-DF-016..020 resolving to tests that exist by name.
- evidence: docs/audits/logs/PMAT-225-gate-B.log and PMAT-225-gate-G.log; contracts/forjar-dogfood-coverage-v1.yaml names tests/falsification_release_goals_are_measured.rs:28's case and its siblings; lanes 1 and 3 of round 1 ran both gates.

12. [workflow] THE DAILY INSTRUMENT HAS THE SHAPE — `.github/workflows/release-goal.yml` is actionlint-clean, fires daily and on demand, runs gate T with the token before the release build after a full clone with tags, keeps one labelled issue open on failure and closes it on success, has `issues: write`, and carries no `continue-on-error` or `|| true` outside comments.
- evidence: tests/falsification_release_goal_workflow_shape.rs:97 pins the upsert and the close; docs/audits/logs/PMAT-225-bashrs.log ends with `actionlint: 0 findings`; all three round-1 lanes confirmed C12 and hunted R4 (two issues, an unclosed issue) without success.

13. [cadence-arm] THE CADENCE ARM IS RIGHT AT EVERY MOMENT — not overdue passes whether Cargo.toml is at the newest tag or at next.tag; overdue with PRs merged and Cargo.toml at the newest tag is red `OVERDUE by Nh`; Cargo.toml at next.tag is `cut in flight`; any other version is red `no goal names`; the clock is the tag's creation instant and DOGFOOD_NOW pins it for fixtures only.
- evidence: tests/falsification_release_goals_are_measured.rs:102 (red then green on the bump) and :118 (the third version); the plan grill's hazard about a non-overdue main was answered here; all three round-1 lanes confirmed C13 and R2.

14. [reads-head] THE GATE READS HEAD, THE TOOL READS THE TREE — gate T reads the ledger, the registry, Cargo.toml and the receipts with `git show HEAD:…`; `scripts/release-goal.sh` reads the working tree and prints `ledger=worktree` or `ledger=worktree(dirty)`, so the label just added is in the status line.
- evidence: tests/falsification_release_goal_cut_books_the_tag.rs:14 asserts `ledger=worktree` and that the gate is green only after the commit; the plan grill refuted a HEAD-reading status tool before any code. All three round-1 lanes confirmed C14.

15. [wiring] THE REGISTRATION IS CONSISTENT — `make dogfood-release` runs tagged.sh after quorum.sh and before coverage.sh, `make release-goal` exists, the mutations guard's REQUIRED list has ten entries, the forjar-dogfood skill table and CLAUDE.md carry a T row, the contract's qa_gate names T, and CHANGELOG [Unreleased] has one paragraph for #506.
- evidence: Makefile:170; CHANGELOG.md:10; tests/falsification_dogfood_scripts_declare_mutations.rs:32. Lane 2 of round 1 confirmed C15 against the Makefile, lane 3 against CLAUDE.md.

16. [double-alias] TWO ROWS DECLARING ONE ALIAS NAME NONE — the resolver refuses rather than picking one, and gate A is red naming both rows and `names none`; the case was added because all three round-1 lanes found the refusal reachable and untested.
- evidence: tests/falsification_dogfood_harness_and_quorum.rs:135; tests/dogfood_gates_harness/mod.rs:166 declares the alias on every owner given. All three round-2 lanes confirmed F1 and the 19-count.

17. [window-keys] WINDOW PRINTS THE ROW AS THE LEDGER SPELLS IT — from dogfood_floor on `release-goal.sh window TAG` prints the `dogfood:` and `crux:` lines, so the five rows are byte-identical to its output; a tag below the floor still omits them and the v1.25.x rows have none; `cut` takes the row as printed and no longer appends the keys itself.
- evidence: tests/falsification_release_goal_cut_books_the_tag.rs:14 asserts the `dogfood:` line in the booked row; lanes 2 and 3 of round 2 re-derived the five diffs (F2, H1); lane 1 confirmed by reading cmd_window and cmd_cut.

18. [workspace] THE WORKSPACE SUITE AND CLIPPY ARE GREEN — `cargo test --workspace --locked --no-fail-fast` exits 0 on ab9bddc8 with 322 binaries, 19,643 passed and 0 failed; `cargo clippy --all-targets --locked -- -D warnings` exits 0; rustfmt check is clean.
- evidence: docs/audits/logs/PMAT-225-workspace.log and PMAT-225-clippy.log; the six gate test binaries re-run on ca3f8d70 in docs/audits/logs/PMAT-225-gate-tests.log (12 + 1 + 19 + 5 + 4 + 9 passed). Orchestrator measurement; no lane ran the whole workspace.

19. [no-regression] THE SHARED WINDOW REFACTOR CHANGED NEITHER GATE — the sixteen pre-existing gate A and E cases pass unchanged with `dogfood_merged_prs` now a call to `dogfood_prs_between` and the registry read at HEAD; the stub gh's search string is what it was.
- evidence: tests/falsification_dogfood_harness_and_quorum.rs:103 sits beside the sixteen, all green in docs/audits/logs/PMAT-225-gate-tests.log; all three round-2 lanes confirmed H3.

## REFUTED

20. [window-keys-first-wording] EVERY LEDGER ROW EQUALS THE TOOL'S OUTPUT, AS FIRST CLAIMED — lane 1 of round 1 ran `release-goal.sh window` for every tag and found the rows for v1.26.0 and v1.27.0 carried `dogfood:` and `crux:` keys the tool did not print, so the text was not the same and the claim was false as worded.
- evidence: round-1 lane 1's finding at scripts/release-goal.sh's cmd_window; the orchestrator's own diffs agreed once run without the awk bound that had hidden it.
- corrected: ca3f8d70 makes `window` print the keys from dogfood_floor on and `cut` take the row as printed; the five rows are byte-identical to the tool's output, re-derived by two round-2 lanes and the orchestrator (item 17).

21. [untested-refusal] THE RESOLVER'S DOUBLE-ALIAS REFUSAL WAS COVERED — the hunt R6 found that the `more than one roadmap row declares alias:` arm in lib/window.sh was reachable and that no test exercised it; all three round-1 lanes returned FAIL on that gap alone or with item 20.
- evidence: round-1 lanes 1, 2 and 3 all named the arm and the absence of a case; the orchestrator confirmed by grep that no test contained `names none`.
- corrected: ca3f8d70 adds tests/falsification_dogfood_harness_and_quorum.rs:135 through the fixture's `declare_alias_on`; the case is red naming both rows, and round 2 confirmed it 3/3 (item 16).
