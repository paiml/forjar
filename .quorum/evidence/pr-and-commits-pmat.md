# PMAT-540 — tools, and what they measured

| tool | what it said |
|---|---|
| `cargo test --test falsification_gate_a_the_pr_and_its_commits_name_one_ticket` | 9 passed |
| `cargo test --test falsification_dogfood_harness_and_quorum` | 19 passed, unchanged |
| the same two against `origin/main`'s `harness.sh` | 5 of 9 red; all 19 green |
| four targeted mutations | each kills its own cases; the matrix is §6 of the log |
| `bash scripts/dogfood/harness.sh` | exit 0 with the floor, exit 1 without it |
| the rule against the 30 most recently merged PRs | 29 agree, 1 mismatch, 0 without a trailer |
| `git log -1 --format='%(trailers)' b4719737` | the `Co-authored-by:` line alone |
| `bashrs lint scripts/dogfood/harness.sh` | 0 errors (0 on `origin/main` too) |
| `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` | exit 0 / exit 0 |
| `pmat analyze vacuous-tests` | 414 of 19687 cannot fail (2.1%) across 2173 files |
| `analyze_vacuous_tests` | same run; none of the nine cases appears |
| `scripts/ci/changed-class.sh` on this branch's diff | `gates=C,D`; `gates=none` without the one `.github/` line |

**The vacuity scan is necessary and not sufficient**, and this ticket is the
clearest example yet. None of the nine cases is vacuous by the analyzer's
measure, and until a review lane read the fixture, SIX of them could not have
caught an arm rewritten to use git's own trailer parser — the one regression the
suite exists to prevent. What settles it is the kill matrix in §6 of the log:
nine cases, nine killers, each named.

**Tool defect worth naming:** the paiml-implement delegate hit its 30-turn cap
before writing a receipt for the eleventh time this session. Filed as
paiml-implement#141.
