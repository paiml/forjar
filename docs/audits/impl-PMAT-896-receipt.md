# Implementation receipt — PMAT-896 — python-ban train: ratchet scripts read JSON with jq, dead splitters deleted

verdict: PARTIAL — the two ratchet scripts run no python and the targeted tests are green on gx10, but two measured reds are host-shaped (a test that needs a populated `.git`, a cap test that needs more than 513 account threads) and one lib test flipped between runs; the scripts' accepted inputs changed and need a separate gate sign-off.

## Identity

- ticket PMAT-896; kind: code; PR #959, branch `cf/896-python-ban-train`, merged with `origin/main` @ dbd1ec3e (this lane's merge commit 02644022)
- files: `scripts/cb200-ratchet.sh`, `scripts/ratchets/comply-count.sh`, `pmat_split.py` (deleted), `split_files.py` (deleted), `docs/roadmaps/roadmap.yaml` (row PMAT-896 added), this receipt
- children: forjar#749 (cb200-ratchet), forjar#752 (comply-count), forjar#958 (dead splitters)

## Root cause of the red CI (five whys)

1. `dogfood-guards` on head 1ee18a0 exited 101 at `cargo test --locked --test ... --test falsification_coverage_gate_mutation_scope ...`.
2. cargo: `no test target named falsification_coverage_gate_mutation_scope`.
3. The workflow step on that run's base still named that target.
4. #973 deleted the test file; the PR branch was 12 commits behind main and carried the old workflow text.
5. #982 on main stopped naming it (`git log origin/main -- .github/workflows`); after `git merge origin/main`, `grep -rl falsification_coverage_gate_mutation_scope .github scripts tests` finds nothing. No code change on the PR was needed; the red was a stale base. Not re-measured on CI (no push).

## What the PR changes (checked by reading the diff against origin/main)

- `cb200-ratchet.sh`: ceiling read with `jq -er '.ceiling'` (an unreadable ceiling now exits 1 with "UNMEASURED"); the count of functions below grade is a jq filter over the comply JSON (a parse failure still yields -1); `*.py` dropped from the newest-source scan.
- `comply-count.sh`: the Python parse is a jq filter with the same contract (pass is 0; a leading integer before `finding` is the count; missing id, no count, no JSON, bad JSON exit non-zero). The wording of the no-count and rotted-id messages is shortened; the exit code for no-JSON and bad JSON is 1, as before.
- Filters spot-checked on lambda with `jq` only (no compile): grade-count filter gives 3 on a message "3 function(s) below minimum grade" and -1 on `{"a":1}`; comply filter gives `COUNT 44` for "44 finding(s) ..." and `NOCOUNT fail ok` for "ok".
- `pmat_split.py`, `split_files.py`: `grep -rlE 'split_files\.py|pmat_split\.py'` outside `.pmat-work/` finds only the roadmap row GH-958's title.

## Evidence (gx10, wrapper `fc-pre.sh pr-959`, tree = worktree at 224dc885 without `.git`)

- `cargo fmt --all -- --check`: rc=0
- `cargo clippy --all-targets -- -D warnings`: rc=0
- `cargo test --lib`, run 1: 13784 passed, 1 failed, 4 ignored. Failed: `cli::inode_gate::tests_scope::a_running_container_that_cannot_answer_still_fails_closed` ("container 'forjar-box' is not running yet" then `unwrap_err()` on Ok). Run 2 (`cargo test --lib -- inode_gate::tests_scope`): 4 passed, 0 failed. The verdict flips on identical inputs; no code touched by this PR is involved; not edited.
- dogfood-guards targets (the ci.yml list that exists on main), `--no-fail-fast`: all green except `falsification_gate_a_the_pr_and_its_commits_name_one_ticket::the_shipped_floor_is_the_commit_it_says_it_is` (8 passed, 1 failed, both runs): `TRAILER_FLOOR "b4719737" does not resolve` because the synced tree has no `.git`. On lambda `git cat-file -t b4719737` is `commit` and `git rev-parse --is-shallow-repository` is `false`. Host-shaped; the other 8 gate-A tests pass.
- `falsification_cb200_ratchet_measures_this_tree`: 4 passed. `falsification_607_release_path_runs_no_python`: 5 passed. `falsification_cb21xx_ratchet_holds_the_ceiling`: 9 passed. `falsification_dogfood_comply_clean_file_is_measured`: 3 passed.
- `falsification_comply_count_cannot_run_inside_itself`: 4 passed, 1 failed (both runs): `the_process_cap_is_applied_and_fails_closed`. The test stubs `ps` to report 1 thread, so the script caps `ulimit -u` at 513 and expects the real `pmat` to be unable to fork; the account on gx10 had 300 threads at the time (`ps -u noah -L | wc -l`), below the cap, so nothing is refused and the script measured (stdout 49). The same shell reproduction with a stubbed `ps` gave rc=0 on the PR script. The cap code is untouched by this PR (the diff covers only the tail of the file). A run of the main version of the script for comparison hung past the 2 min budget and was killed; not measured.

Known LAB reds (forjar#858) not run: `falsification_untyped_error_sites_ratchet`, `falsification_budget_dry_run`, `falsification_release_workflow_shape`.

Not measured: CI on the merged tree (nothing pushed); `cargo test --workspace`; the clean-room.

IMPL-PMAT-896-RECEIPT-END
