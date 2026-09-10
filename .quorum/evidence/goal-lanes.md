# PMAT-225 — quorum lanes (agy, --mode plan, --sandbox, writes=false)

Each lane cloned the branch with `git clone --shared` at the commit named, re-ran the commands of the claims it could, and returned one structured verdict. Conversation ids are shortened to `conv-<8hex>`.

## round 1 (db298604) — 3 lane(s)

### round 1 (db298604) lane 1 — verdict FAIL (conv-b4b4d310, 1 turn(s), 1355s, exit SUCCESS)

PMAT-225 introduces the required release cadence gates, correctly implements alias resolution for misnamed PRs like #496, and asserts release goal correctness against the ledger. However, it fails on two points: C7 is refuted because `release-goal.sh window` omits `dogfood:` and `crux:` keys (and emits unticketed PR comments) causing output mismatch with the written YAML, and R6 is confirmed as a defect because while the codebase correctly refuses multiple rows claiming the same alias, this refusal is completely untested in the falsification suite.

| claim | grounding | file | finding |
|---|---|---|---|
| C1 | measured | `scripts/dogfood/surface.sh` | C1 |
| C2 | cited | `scripts/dogfood/tagged.sh:139` | C2 |
| C3 | measured | `scripts/dogfood/docs.sh:327` | C3 |
| C4 | measured | `scripts/dogfood/comply.sh:85` | C4 |
| C5 | measured | `tests/falsification_dogfood_harness_and_quorum.rs:100` | C5 |
| C6 | cited | `scripts/dogfood/lib/window.sh:344` | C6 |
| C7 | measured | `scripts/release-goal.sh:258` | C7 => Add dogfood and crux keys to the window output, and remove unticketed PR comments to match YAML exactly. |
| C8 | measured | `scripts/release-goal.sh` | C8 |
| C9 | measured | `scripts/release-goal.sh` | C9 |
| C10 | cited | `scripts/dogfood/lib/window.sh:371` | C10 |
| C11 | measured | `contracts/forjar-dogfood-coverage-v1.yaml:259` | C11 |
| C12 | cited | `.github/workflows/release-goal.yml:18` | C12 |
| C13 | cited | `scripts/dogfood/tagged.sh:215` | C13 |
| C14 | cited | `scripts/release-goal.sh:197` | C14 |
| C15 | cited | `scripts/release-goal.sh:163` | C15 |
| R1 | cited | `scripts/dogfood/harness.sh:83` | R1 |
| R2 | measured | `scripts/dogfood/tagged.sh:300` | R2 |
| R3 | cited | `scripts/dogfood/lib/window.sh:390` | R3 |
| R4 | cited | `.github/workflows/release-goal.yml:67` | R4 |
| R5 | cited | `scripts/release-goal.sh:49` | R5 |
| R6 | measured | `tests/falsification_dogfood_harness_and_quorum.rs:100` | R6 => Add a falsification test asserting that when two roadmap rows declare the same alias, dogfood_resolve_id fails with 'resolves to no single ticket'. |

### round 1 (db298604) lane 2 — verdict FAIL (conv-80512b73, 1 turn(s), 1476s, exit SUCCESS)

All 15 implementation claims (C1-C15) are confirmed via tests, code inspection, and measured execution. The first 5 refutation hunts (R1-R5) failed to find gaps, proving the code correctly prevents stray aliases from being skipped, correctly handles jq/grep pipeline failures, properly upserts the GitHub issue, and strictly edits YAML files without corruption. However, refutation hunt R6 successfully found a gap: the logic refusing an alias claimed by multiple rows is reachable, but completely untested in the rust test suite (no test asserts the 'names two owners names none' failure). Because an R gap exists, the overall quorum verdict is FAIL.

| claim | grounding | file | finding |
|---|---|---|---|
| C1 | measured | `scripts/dogfood/tagged.sh` | C1 |
| C2 | measured | `scripts/dogfood/tagged.sh` | C2 |
| C3 | cited | `tests/falsification_release_goals_are_measured.rs` | C3 |
| C4 | cited | `tests/falsification_release_goal_cut_books_the_tag.rs` | C4 |
| C5 | cited | `tests/falsification_dogfood_harness_and_quorum.rs` | C5 |
| C6 | cited | `scripts/dogfood/lib/window.sh` | C6 |
| C7 | measured | `docs/roadmaps/releases.yaml` | C7 |
| C8 | measured | `docs/roadmaps/roadmap.yaml` | C8 |
| C9 | measured | `scripts/release-goal.sh` | C9 |
| C10 | cited | `tests/falsification_dogfood_scripts_declare_mutations.rs` | C10 |
| C11 | measured | `scripts/dogfood/comply.sh` | C11 |
| C12 | measured | `.github/workflows/release-goal.yml` | C12 |
| C13 | cited | `tests/falsification_release_goals_are_measured.rs` | C13 |
| C14 | cited | `scripts/release-goal.sh` | C14 |
| C15 | cited | `Makefile` | C15 |
| R1 | cited | `scripts/dogfood/lib/window.sh` | R1 |
| R2 | cited | `scripts/dogfood/tagged.sh` | R2 |
| R3 | cited | `scripts/dogfood/tagged.sh` | R3 |
| R4 | cited | `.github/workflows/release-goal.yml` | R4 |
| R5 | cited | `scripts/release-goal.sh` | R5 |
| R6 | measured | `scripts/dogfood/lib/window.sh` | R6 => Add a falsification test in tests/ to assert that the gate fails when multiple roadmap rows declare the same alias. |

### round 1 (db298604) lane 3 — verdict FAIL (conv-2e7f1c3f, 1 turn(s), 1259s, exit SUCCESS)

15/15 claims CONFIRMED. 5/6 refutations FAILED (R1-R5). 1/6 refutation SUCCEEDED (R6). The branch is rejected because the refusal for multiple rows sharing an alias in `dogfood_resolve_id` is reachable but untested.

| claim | grounding | file | finding |
|---|---|---|---|
| C1 | measured | `scripts/dogfood/tagged.sh:315` | C1: ran scripts/dogfood/tagged.sh; exited 0 and printed EXACT expected passing output |
| C2 | measured | `scripts/dogfood/tagged.sh:279` | C2: mutating cadence from 86400 to 86401 caused gate to go red with the expected discrepancy name |
| C3 | cited | `tests/falsification_release_goals_are_measured.rs:11` | C3: 12 tests verified for exactly asserting red failure behaviors on invalid dates/ledgers |
| C4 | cited | `tests/falsification_release_goal_cut_books_the_tag.rs:12` | C4: tests verified for cut flow formatting and sync --check ledger label extraction |
| C5 | asserted | `tests/falsification_dogfood_harness_and_quorum.rs:1` | C5: 18 test cases exist; the two new gate A cases fail against main's harness.sh |
| C6 | cited | `scripts/dogfood/lib/window.sh:346` | C6: dogfood_pr_tickets extracts first ID, dogfood_resolve_id properly falls back on stray |
| C7 | measured | `docs/roadmaps/releases.yaml:1` | C7: release-goal.sh window [tag] exactly matches the 4 ledger blocks. |
| C8 | measured | `docs/roadmaps/roadmap.yaml:1` | C8: git diff only contained updated:, labels:, and PMAT-225 row modifications |
| C9 | measured | `scripts/release-goal.sh:40` | C9: ran release-goal.sh window v1.25.1; it printed 'tickets: []' and exited 0 |
| C10 | cited | `tests/falsification_dogfood_scripts_declare_mutations.rs:1` | C10: no internal fail outputs are swallowed via `// true` or `>/dev/null`; mutations test covers all 10 scripts |
| C11 | measured | `contracts/forjar-dogfood-coverage-v1.yaml:1` | C11: comply.sh clean, contracts.sh valid for FALSIFY-DF-016..020. |
| C12 | measured | `.github/workflows/release-goal.yml:1` | C12: actionlint workflow clean, shape test has 5 cases |
| C13 | cited | `scripts/dogfood/tagged.sh:300` | C13: Cadence arm logic correctly checks if overdue and PRs merged. |
| C14 | cited | `scripts/dogfood/lib/releases.sh:34` | C14: Gate T reads Cargo.toml at HEAD, release-goal prints ledger=worktree. |
| C15 | cited | `CLAUDE.md:1` | C15: Makefile runs tagged.sh appropriately, gate T integrated in CLAUDE, CHANGELOG and contracts |
| R1 | cited | `scripts/dogfood/lib/window.sh:365` | R1 REFUTATION FAILED: Stray ID zeroes ticket causing gate A to fail. Branch/title ID combos are unioned and gate T strictly verifies both carry release:TAG, pre |
| R2 | cited | `scripts/dogfood/tagged.sh:300` | R2 REFUTATION FAILED: Expected behavior verified. Expected red before cut, passes when bumped, passes on empty window, correctly fails on feature branch if trul |
| R3 | cited | `scripts/dogfood/lib/window.sh:303` | R3 REFUTATION FAILED: grep/jq inside command substitutions correctly handle exit codes with `// rc=$?` under set -o pipefail. |
| R4 | cited | `.github/workflows/release-goal.yml:78` | R4 REFUTATION FAILED: Upsert queries use `.[0].number` to prevent multiple issues; success step closes all open issues. Workflows are correctly permissioned. |
| R5 | cited | `scripts/release-goal.sh:68` | R5 REFUTATION FAILED: Python string slicing rigorously bounds changes to specific block bytes. No corruption happens on missing `updated:` fields. |
| R6 | cited | `scripts/dogfood/lib/window.sh:329` | R6 REFUTATION SUCCEEDED: The failure path `*\" \"*) fail ...` for multiple rows sharing an alias is reachable but completely untested in falsification_dogfood_h => Add a test in `tests/falsification_dogfood_harness_and_quorum.rs` modifying the roadmap to give two tickets the same alias and asserting Gate A correctly turns red. |

## round 2 (ca3f8d70) — 3 lane(s)

### round 2 (ca3f8d70) lane 1 — verdict PASS (conv-72471c83, 1 turn(s), 327s, exit SUCCESS)

All claims are confirmed. Cargo tests pass (F1 and F4), bashrs reports 0 errors. F3's mutation (86400 -> 86401) correctly fails the cargo test falsification_release_goals_are_measured. F2 is confirmed via code review (cmd_window adds the keys conditionally based on dogfood_floor, and cmd_cut simply takes the row). H1 is confirmed (v1.25.x lack the keys). H2: cut assumes existence, Gate T enforces it. H3: no regressions found.

| claim | grounding | file | finding |
|---|---|---|---|
| F1 | measured | `tests/falsification_dogfood_harness_and_quorum.rs:135` | F1: the resolver's refusal of an alias declared by two rows is tested and passes 19 |
| F2 | cited | `scripts/release-goal.sh:150` | F2: window prints keys for tag at or above floor, and cut no longer appends them |
| F3 | measured | `scripts/dogfood/tagged.sh:277` | F3: the registered mutation (86400 -> 86401) turns it red |
| F4 | measured | `scripts/release-goal.sh:1` | F4: bashrs reports 0 errors, cargo tests pass 12 and 1 |
| H1 | cited | `docs/roadmaps/releases.yaml:22` | H1: tags below dogfood_floor omit keys, and v1.25.x rows in ledger have none |
| H2 | asserted | `scripts/release-goal.sh:258` | H2: cut blindly takes keys from window; if receipt does not exist, Gate T fails |
| H3 | measured | `tests/falsification_dogfood_harness_and_quorum.rs:1` | H3: no regressions in 18 pre-existing gate A/E cases |

### round 2 (ca3f8d70) lane 2 — verdict PASS (conv-c906c98c, 1 turn(s), 235s, exit SUCCESS)

All claims are confirmed. The branch tests the double-alias refusal properly (fx.declare_alias_on). 'window' prints output exactly byte-matching the ledger (including properly omitting keys for tags below dogfood_floor). 'cut' utilizes 'window' without manually appending keys, and gate T correctly guards against non-existent receipts via git cat-file. 'tagged.sh' passes with 0 exit code on HEAD and is appropriately sensitive to mutations. 'bashrs lint' shows 0 errors, and tests have no regressions (19 tests pass on the dogfood harness).

| claim | grounding | file | finding |
|---|---|---|---|
| F1 | cited | `tests/falsification_dogfood_harness_and_quorum.rs:137` | F1 - double alias refusal is tested and red naming both rows and names none |
| F2 | measured | `scripts/release-goal.sh:150` | F2 - window prints dogfood: and crux: for >= dogfood_floor byte-identically to releases.yaml, and cut no longer appends them |
| F3 | measured | `scripts/dogfood/tagged.sh:269` | F3 - tagged.sh exits 0 on ca3f8d70 and mutation 86400 -> 86401 turns it red |
| F4 | measured | `scripts/release-goal.sh:1` | F4 - bashrs reports 0 errors on release-goal.sh, test counts match |
| H1 | measured | `docs/roadmaps/releases.yaml:29` | H1 - window omits keys below dogfood_floor, ledger's v1.25.x rows have none |
| H2 | cited | `scripts/dogfood/tagged.sh:173` | H2 - cut produces row at floor that gate T accepts; if cut wrote a missing receipt, gate T would catch it |
| H3 | cited | `scripts/dogfood/lib/window.sh:408` | H3 - no regressions in 18 pre-existing cases or round-1 fixes |

### round 2 (ca3f8d70) lane 3 — verdict PASS (conv-3f64b007, 1 turn(s), 238s, exit SUCCESS)

F1: CONFIRMED. The resolver's refusal of an alias declared by two rows is tested in `gate_a_an_alias_declared_by_two_rows_names_no_owner_and_is_red` which names both rows and expects "names none". \nF2: CONFIRMED. `window TAG` outputs the byte-identical row for all 5 tags; the tool omits `dogfood:` keys if below `dogfood_floor`.\nF3: CONFIRMED. `tagged.sh` outputs `GATE T PASS 5 tagged release(s)... 8 of 8 PR(s) merged since v1.27.0 carry release:v1.28.0`. Mutation 86400->86401 turns it red with exit code 1.\nF4: CONFIRMED. `bashrs lint scripts/release-goal.sh` yields 0 errors.\n\nH1: `window` successfully omits keys for tags below `dogfood_floor` (v1.26.0) and the v1.25.x rows do not have them.\nH2: `cut` produces a row at the floor which gate T accepts (test passes). If the receipt does not exist, `cut` still writes the key, but gate T correctly fails with `not at HEAD`.\nH3: The new gate A test was cleanly added (now 19 tests) with no regressions to existing tests or round-1 findings.

| claim | grounding | file | finding |
|---|---|---|---|
| F1 | cited | `tests/falsification_dogfood_harness_and_quorum.rs:147` | F1 => r.assert_says("names none"); |
| F2 | measured | `scripts/release-goal.sh:144` | F2 => if dogfood_semver_ge "$tag" "$DOGFOOD_FIELD"; then |
| F3 | measured | `scripts/dogfood/tagged.sh:310` | F3 => echo "GATE T PASS ${checked} tagged release(s) ... |
| F4 | measured | `scripts/release-goal.sh:1` | F4 => Summary: 0 error(s), 12 warning(s), 32 info(s) |
| H1 | measured | `docs/roadmaps/releases.yaml:52` | H1 => dogfood_floor: v1.26.0 |
| H2 | cited | `tests/falsification_release_goal_cut_books_the_tag.rs:85` | H2 => r.assert_green("the cut is booked, the labels agree, the next goal is declared"); |
| H3 | cited | `tests/falsification_dogfood_harness_and_quorum.rs:135` | H3 => fn gate_a_an_alias_declared_by_two_rows_names_no_owner_and_is_red() |

