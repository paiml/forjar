# Quorum evidence — PMAT-226 — the claims as put to the lanes

# PMAT-226 — claims for the quorum lanes (release: forjar 1.28.0)

Branch PMAT-226-release-1.28.0, three commits on main (191770fa):
a183f21a (rail + tests + roadmap row), 97e72ce6 (bump + CHANGELOG + README),
and the crux commit (docs/audits/crux-1.28.0.md). Judge the diff `main...HEAD`.

1. scripts/quorum-gate.sh admits exactly `docs/roadmaps/releases.yaml` on the
   `kind: triage` rail beside `docs/roadmaps/roadmap.yaml` — by file, not by
   directory — and any other path outside the rail is still refused BY NAME.
2. tests/falsification_quorum_gate_has_a_triage_shape.rs: the new positive
   case (`a_triage_receipt_over_the_release_ledger_is_on_the_rail`) is RED
   against main's scripts/quorum-gate.sh and GREEN after; the negative case
   (`another_file_under_roadmaps_is_still_off_the_rail`) is green on both and
   asserts the refusal names the file.
3. The fixture refactor (`touch_code: bool` → `also: Option<(&str, &str)>`)
   changes no existing case's behaviour: 6 pre-existing tests still pass and
   still exercise the same shapes (the code-edit case still edits src/lib.rs).
4. The version bump is complete and consistent: Cargo.toml 1.28.0, Cargo.lock's
   forjar entry 1.28.0, README caret requirements `1.28` at both sites;
   `cargo metadata --locked --offline` succeeds; no other file in the tree
   still names 1.27.0 as the current version (docs/audits and the ledger name
   it as history, which is correct).
5. CHANGELOG.md: `[Unreleased]` is left empty at the top; `[1.28.0] -
   2026-09-10` carries the four paragraphs [Unreleased] held plus five new
   ones (#497, #485, #487, #495, PMAT-226). Each new paragraph describes the
   merged PR's behaviour change accurately against the PR body and the code
   (PRs #500, #496, #493, #498; the rail change in this diff).
6. The [1.28.0] intro's census is measured: nine PRs merged since v1.27.0
   (#493 #494 #496 #498 #500 #502 #504 #505 #507), nine tickets
   (PMAT-215 217 219 220 221 222 223 224 225), every one carrying
   `release:v1.28.0` on the roadmap; #494 is correctly called test-only.
7. docs/audits/crux-1.28.0.md: every `[V]` citation resolves at HEAD to a
   line that supports the claim beside it; every row names >= 3 systems from
   scripts/dogfood/crux-reconcile.sh's SYSTEMS roster; gate H passes 9 of 9.
8. The three corrections the crux Method section claims were made are real:
   the quorum receipt is hash-bound (no signature anywhere in
   scripts/quorum-gate.sh or scripts/quorum_evidence.py), `machine_is_local`
   is unchanged by #495 (PR #498), and the rail is verified from the diff.
9. No file under src/ changes on this branch, so gate F's mutation arm
   (scripts/dogfood/coverage.sh) measures 0 mutable files and passes that arm
   honestly; the line-coverage arm is still measured.
10. scripts/quorum_evidence.py changes in a comment only; behaviour unchanged.
11. The PMAT-226 roadmap row carries kind: code, orch:fable with an
    orch-basis: token, the label release:v1.28.0, and acceptance criteria
    that are commands or measurable statements.
12. Gate T on this branch reports "cut in flight: Cargo.toml is at 1.28.0",
    which is the designed pre-tag state (T6), not a pass by accident.
