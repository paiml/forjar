# PMAT-542 — tools, and what they measured

| tool | what it said |
|---|---|
| `cargo test --test falsification_pr_lane_selects_the_gate_the_change_can_move` | 14 passed |
| `cargo test --test falsification_pr_lane_runs_what_the_change_can_break` | 13 passed |
| the same two, with three files reverted to `origin/main` | 14 of 14 red; 12 of PMAT-237's 13 green |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `bashrs lint scripts/ci/changed-class.sh` | 0 errors, 3 warnings, 7 infos |
| `bashrs lint` on `origin/main`'s copy | 0 errors, 2 warnings, 4 infos |
| `bash -n` on the classifier | exit 0 |
| `python3 -c "yaml.safe_load(…)"` on both workflow files | both parse |
| `scripts/ci/changed-class.sh` over 25 merged PRs | 3 / 15 / 7 |
| `pmat analyze vacuous-tests` | 414 of 19687 cannot fail (2.1%) across 2173 files |
| `analyze_vacuous_tests` | same run; none of the 27 cases appears |
| the pre-commit complexity gate | refused one case at Cognitive 27 > 25; split into `gate_path`, `paths_the_gates_name` and the case |
| `gh issue create` / `gh issue edit` | forjar#542 |

**The vacuity scan is necessary and not sufficient**, and this ticket is a good
example of why: none of the 27 cases is vacuous by the analyzer's measure, and
the suite still could not have found the defect the round found. What separates
a real case from a decorative one here is the RED run in §6 of the log, and what
separates a sound selection from an unsound one is reading the two gate scripts
— which is now a case of its own,
`every_path_the_two_gates_name_is_selected_by_the_classifier`.

**bashrs adds one warning and three infos** over `origin/main`'s copy of the
same file, all on the new `case` arms and the `if` chain; 0 errors on both
sides, which is the floor this repository holds.

**Tool defect worth naming:** none new. The 30-turn delegate limit is recorded
in the agy note and belongs to the paiml-implement repository.
