# PMAT-226 — quorum lanes (agy, --mode plan, --sandbox, writes=false)

Each lane cloned the branch with `git clone --shared` at 568dc169 (origin/main pinned to 191770fa in the clone), re-ran the commands of the claims it could — the triage-shape test at HEAD and with main's gate script checked out, gate H, `gh pr list` for the window — and returned one structured verdict. Conversation ids are shortened to `conv-<8hex>`; no lane wrote to the repository.

## round 1 (568dc169) — 3 lane(s)

### round 1 (568dc169) lane 1 — verdict PASS (conv-8a855795, 1 turn(s), 0s, exit ?)

All twelve claims are CONFIRMED. The gate explicitly adds `docs/roadmaps/releases.yaml` to the rail and successfully fails correctly shaped un-allowed paths on older gates while passing on HEAD. The version bump is clean across Cargo.toml, Cargo.lock, and README.md, and `cargo metadata` passes. The `[1.28.0]` changelog carries exactly the combined paragraphs and matches PR bodies precisely. The PR and ticket census is accurate, with all nine roadmap entries tagged with `release:v1.28.0`. All `[V]` citations in the crux document correctly map to their target lines, and `crux-reconcile.sh` passed 9 of 9 behavior bullets. Finally, `tagged.sh` reports the cut in flight accurately.

| claim | grounding | file | finding |
|---|---|---|---|
| C1 | cited | `scripts/quorum-gate.sh:384` | CONFIRMED scripts/quorum-gate.sh adds docs/roadmaps/releases.yaml to the rail by file name, and prints off_rail files in its die statement. |
| C2 | measured | `tests/falsification_quorum_gate_has_a_triage_shape.rs:446` | CONFIRMED cargo test --test falsification_quorum_gate_has_a_triage_shape failed a_triage_receipt_over_the_release_ledger_is_on_the_rail on 191770fa but passed on HEAD. another_file_under_roadmaps_is_still_off_the_rail passed on both. |
| C3 | cited | `tests/falsification_quorum_gate_has_a_triage_shape.rs:45` | CONFIRMED The fixture touch_code was replaced by also: Option<(&str, &str)> with CODE_EDIT covering the previous src/lib.rs case. All 6 pre-existing tests still pass. |
| C4 | measured | `Cargo.toml:3` | CONFIRMED Version bumped to 1.28.0 in Cargo.toml and Cargo.lock, README has 1.28. cargo metadata --locked --offline succeeds. 1.27.0 is only seen in historical contexts. |
| C5 | cited | `CHANGELOG.md:10` | CONFIRMED [Unreleased] is empty, [1.28.0] carries 4 old and 5 new paragraphs that accurately describe the changes from PRs 500, 496, 493, 498 and PMAT-226. |
| C6 | measured | `docs/roadmaps/roadmap.yaml:1` | CONFIRMED gh pr list confirmed exactly the 9 merged PRs. Every ticket (PMAT-215 to PMAT-225) carries the release:v1.28.0 label. PR 494 is accurately named test-only. |
| C7 | measured | `docs/audits/crux-1.28.0.md:1` | CONFIRMED crux-reconcile.sh passed 9 of 9 rows. Every [V] citation was checked and verifies the text at the cited line. |
| C8 | cited | `scripts/quorum_evidence.py:51` | CONFIRMED No 'signature' string exists in scripts/quorum-gate.sh or quorum_evidence.py. machine_is_local was untouched. The rail is verified from the diff. |
| C9 | measured | `scripts/dogfood/coverage.sh:76` | CONFIRMED git diff shows no files changed under src/. coverage.sh executes line coverage before honestly skipping mutation testing if mutable_rs is 0. |
| C10 | cited | `scripts/quorum_evidence.py:54` | CONFIRMED The only change in scripts/quorum_evidence.py is a comment update mentioning the release ledger. |
| C11 | cited | `docs/roadmaps/roadmap.yaml:3292` | CONFIRMED PMAT-226 carries kind: code, orch:fable, release:v1.28.0, orch-basis:release, and has measurable acceptance criteria. |
| C12 | measured | `scripts/dogfood/tagged.sh:300` | CONFIRMED Gate T (tagged.sh) exits reporting 'cut in flight: Cargo.toml is at 1.28.0' exactly as designed. |

### round 1 (568dc169) lane 2 — verdict FAIL (conv-9f491252, 1 turn(s), 0s, exit ?)

11 of the 12 claims are confirmed, but Claim 11 is refuted because the PMAT-226 roadmap row notes specify `orch-basis:release`, not `orch-basis: token` as claimed.

| claim | grounding | file | finding |
|---|---|---|---|
| C1 | cited | `scripts/quorum-gate.sh:385` | CONFIRMED scripts/quorum-gate.sh admits docs/roadmaps/releases.yaml by file on the triage rail, and other paths outside the rail are refused by name. |
| C2 | measured | `tests/falsification_quorum_gate_has_a_triage_shape.rs:450` | CONFIRMED The positive case (a_triage_receipt_over_the_release_ledger_is_on_the_rail) is RED at main and GREEN at HEAD, while the negative case remains GREEN. |
| C3 | measured | `tests/falsification_quorum_gate_has_a_triage_shape.rs:173` | CONFIRMED The fixture refactor uses Option<(&str, &str)> and does not change existing test behaviors. |
| C4 | measured | `Cargo.toml:3` | CONFIRMED The version bump is complete across Cargo.toml, Cargo.lock, and README caret requirements. |
| C5 | cited | `CHANGELOG.md:88` | CONFIRMED CHANGELOG.md carries the 4 unreleased paragraphs plus 5 new ones, each accurately describing the PR behavior. |
| C6 | measured | `CHANGELOG.md:13` | CONFIRMED The intro census is accurately measured with 9 PRs across 9 tickets merged since v1.27.0. |
| C7 | measured | `docs/audits/crux-1.28.0.md:37` | CONFIRMED Every [V] citation resolves correctly at HEAD, and gate H passes 9 of 9 behavior bullets. |
| C8 | cited | `scripts/quorum-gate.sh:370` | CONFIRMED The quorum receipt is hash-bound, machine_is_local remains unchanged by PR 498, and the rail is verified from the diff. |
| C9 | cited | `scripts/dogfood/coverage.sh:127` | CONFIRMED Gate F's mutation arm measures 0 mutable src/ files and passes honestly, while line-coverage is still measured. |
| C10 | cited | `scripts/quorum_evidence.py:54` | CONFIRMED scripts/quorum_evidence.py changes only in comments. |
| C11 | cited | `docs/roadmaps/roadmap.yaml:3316` | REFUTED The PMAT-226 roadmap row carries `orch-basis:release`, not `orch-basis: token` as claimed. |
| C12 | measured | `scripts/dogfood/tagged.sh:296` | CONFIRMED Gate T correctly reports 'cut in flight: Cargo.toml is at 1.28.0' at HEAD. |

### round 1 (568dc169) lane 3 — verdict FAIL (conv-8a38ece1, 1 turn(s), 0s, exit ?)

Claims 1, 2, 3, 4, 6, 7, 8, 9, 10, and 12 were CONFIRMED by reading files and running the requested validations. Claims 5 and 11 were REFUTED: Claim 5 is refuted because CHANGELOG.md carries six new paragraphs, not five (it includes an introductory paragraph); Claim 11 is refuted because the PMAT-226 roadmap row carries `orch-basis:release`, not `orch-basis: token`.

| claim | grounding | file | finding |
|---|---|---|---|
| C1 | cited | `scripts/quorum-gate.sh:386` | CONFIRMED scripts/quorum-gate.sh admits exactly docs/roadmaps/releases.yaml on the kind: triage rail beside docs/roadmaps/roadmap.yaml — by file, not by directory — and any other path outside the rail is still refused BY NAME. |
| C2 | measured | `tests/falsification_quorum_gate_has_a_triage_shape.rs:446` | CONFIRMED tests/falsification_quorum_gate_has_a_triage_shape.rs: the new positive case (a_triage_receipt_over_the_release_ledger_is_on_the_rail) is RED against main's scripts/quorum-gate.sh and GREEN after; the negative case (another_file_under_roadmaps_is_still_off_the |
| C3 | measured | `tests/falsification_quorum_gate_has_a_triage_shape.rs:171` | CONFIRMED The fixture refactor (touch_code: bool -> also: Option<(&str, &str)>) changes no existing case's behaviour: 6 pre-existing tests still pass and still exercise the same shapes. |
| C4 | measured | `Cargo.toml:3` | CONFIRMED The version bump is complete and consistent: Cargo.toml 1.28.0, Cargo.lock's forjar entry 1.28.0, README caret requirements 1.28 at both sites; cargo metadata --locked --offline succeeds; no other file in the tree still names 1.27.0 as the current version (doc |
| C5 | cited | `CHANGELOG.md:12` | REFUTED CHANGELOG.md carries four paragraphs from [Unreleased] plus *six* new ones (an introductory paragraph plus the five issue paragraphs named). |
| C6 | cited | `docs/roadmaps/roadmap.yaml:3292` | CONFIRMED The [1.28.0] intro's census is measured: nine PRs merged since v1.27.0 (#493 #494 #496 #498 #500 #502 #504 #505 #507), nine tickets (PMAT-215 217 219 220 221 222 223 224 225), every one carrying release:v1.28.0 on the roadmap; #494 is correctly called test-onl |
| C7 | measured | `docs/audits/crux-1.28.0.md:32` | CONFIRMED docs/audits/crux-1.28.0.md: every [V] citation resolves at HEAD to a line that supports the claim beside it; every row names >= 3 systems from scripts/dogfood/crux-reconcile.sh's SYSTEMS roster; gate H passes 9 of 9. |
| C8 | cited | `src/transport/mod.rs:445` | CONFIRMED The three corrections the crux Method section claims were made are real: the quorum receipt is hash-bound (no signature anywhere in scripts/quorum-gate.sh or scripts/quorum_evidence.py), machine_is_local is unchanged by #495 (PR #498), and the rail is verified |
| C9 | cited | `scripts/dogfood/coverage.sh:177` | CONFIRMED No file under src/ changes on this branch, so gate F's mutation arm (scripts/dogfood/coverage.sh) measures 0 mutable files and passes that arm honestly; the line-coverage arm is still measured. |
| C10 | cited | `scripts/quorum_evidence.py:55` | CONFIRMED scripts/quorum_evidence.py changes in a comment only; behaviour unchanged. |
| C11 | cited | `docs/roadmaps/roadmap.yaml:3316` | REFUTED The PMAT-226 roadmap row carries orch-basis:release, not orch-basis: token. |
| C12 | measured | `scripts/dogfood/tagged.sh:296` | CONFIRMED Gate T on this branch reports "cut in flight: Cargo.toml is at 1.28.0", which is the designed pre-tag state (T6), not a pass by accident. |

## reduction

`lane-reduce.sh <out_dir> --width 3 --not-before <epoch of the dispatch>`: 1 PASS, 2 FAIL, dissent 2 (lane 2: C11; lane 3: C5 and C11). Both refutations are corrections to the claim text and neither touches the diff — see release-1.28.0-judges.md — so no second round was run on an unchanged diff.
