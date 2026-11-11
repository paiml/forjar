# Implementation receipt — PMAT-896 — python-ban train: ratchet scripts read JSON with jq, dead splitters deleted

verdict: PARTIAL — the ratchet scripts run no python and now fail closed on unreadable or ambiguous comply output, and every target is green on gx10 except one test whose precondition (an account running more than 513 threads) gx10 does not meet; the scripts' accepted inputs changed and the gate-content sign-off is still open.

## Identity

- ticket PMAT-896; kind: code; PR #959, branch `cf/896-python-ban-train`, merged with `origin/main` @ dbd1ec3e
- files: `scripts/cb200-ratchet.sh`, `scripts/ratchets/comply-count.sh`, `pmat_split.py` (deleted), `split_files.py` (deleted), `tests/falsification_cb200_ratchet_measures_this_tree.rs`, `tests/falsification_comply_count_cannot_run_inside_itself.rs`, `docs/roadmaps/roadmap.yaml` (row PMAT-896 added), this receipt
- children: forjar#749 (cb200-ratchet), forjar#752 (comply-count), forjar#958 (dead splitters; roadmap row GH-958 left as is)

## Root cause of the red CI (five whys)

1. `dogfood-guards` on head 1ee18a0 exited 101 at `cargo test --locked --test ... --test falsification_coverage_gate_mutation_scope ...`.
2. cargo: `no test target named falsification_coverage_gate_mutation_scope`.
3. The workflow step on that run's base still named that target.
4. #973 deleted the test file; the PR branch was 12 commits behind main and carried the old workflow text.
5. #982 on main stopped naming it; after `git merge origin/main` nothing in `.github scripts tests` names it. No code change was needed. Not re-measured on CI (nothing pushed).

## Review round 1 (three non-author lanes FAIL on 31cae5b) and the fix

Defect (lanes 1 and 2, verified by RED tests below, not by the lanes' descriptions): the jq rewrite read each JSON document of the output separately.

- `comply-count.sh`: the output was cut at the first `{` anywhere (so `[log] cfg={}` starts the cut inside a banner), then `jq -r` printed one row per document. Two documents gave two rows, which matched no arm of `case "$row"`, and the script left the `case` with no `*)` arm: exit 0, empty stdout.
- `cb200-ratchet.sh`: with two documents jq printed `-1` and `700` on two lines. Measured on gx10 before the fix: exit 0 and "CB-200 at the recorded ceiling (-1 700)", a pass, because the integer test on a two-line value was not an error that stopped the script. Whitespace-only output reached the integer test as an empty string.

Fix: both scripts use `jq -s` and accept exactly one document (any other count is `AMBIGUOUS`/`-1`); `comply-count.sh` gets an `AMBIGUOUS` arm and a `*)` catch-all that exit 1 and say why; `cb200-ratchet.sh` forces any non-numeric `now` to -1 (UNMEASURED, exit 1).

RED (gx10, commit 5ea071d2, tests only): `an_output_that_is_not_exactly_one_document_is_unmeasured` FAILED ("two documents, noise first: expected a refusal with exit 1, got 0 ... at the recorded ceiling (-1 700)"); `unreadable_or_ambiguous_comply_output_fails_closed` FAILED ("brace on a banner line: exited 0 (stdout=\"\", stderr=\"\")"). Controls `exactly_one_document_is_measured` and `one_document_is_measured_with_or_without_a_plain_banner` passed.
GREEN (gx10, commit ad9b7049): the same tests pass (cb200 target 6 passed; comply-count target 6 passed, 1 failed, see below).

Inputs covered (each non-zero, nothing on stdout for comply-count; UNMEASURED exit 1 for cb200): noise document then report; report then `{}`; whitespace only (cb200); banner line `[log] cfg={}` before the report (comply-count); `{}` line before the report; two documents; no JSON at all; truncated document; a plain-text banner before the report is still read by comply-count (control).

### Where the jq version still differs from the deleted Python (read from `git show origin/main:<path>`)

- cb200: banner text before the document: Python and jq both give UNMEASURED (neither strips). Lane 2 asked for stripping; it was not added, because it would accept an input the Python refused.
- cb200: a `message` that is not a string (a list or object containing the phrase) was matched by Python through `str()`; jq's `strings` ignores it. No such output has been seen; not measured.
- comply-count: error wording for "no count" and "rotted id" is shorter; exit codes are the same (1). A multi-document output now says "not exactly one JSON document"; Python said "does not parse: Extra data". Both exit 1.
- comply-count: a non-string `status` or `name` is converted with `tostring`, as Python's `str()` did; jq prints `null` for a missing value where Python printed `None`. Neither can match.

## Evidence (gx10, wrapper `fc-pre.sh pr-959` via `bash green.sh`, tree = clone of the worktree at ad9b7049 WITH `.git`)

- `cargo fmt --all -- --check`: rc=0; `cargo clippy --all-targets -- -D warnings`: rc=0
- `cargo test --lib`: 13785 passed, 0 failed, 4 ignored (the inode_gate test that flipped in the earlier run was green)
- dogfood-guards targets present in ci.yml, `--no-fail-fast`: all passed, including `falsification_gate_a_the_pr_and_its_commits_name_one_ticket` (9 passed; it needs `.git`, which this run had)
- `falsification_607_release_path_runs_no_python` 5 passed; `falsification_cb21xx_ratchet_holds_the_ceiling` 9 passed; `falsification_dogfood_comply_clean_file_is_measured` 3 passed
- `falsification_comply_count_cannot_run_inside_itself`: 6 passed, 1 failed, in three runs of this file: `the_process_cap_is_applied_and_fails_closed`. It stubs `ps` to 1 thread so the cap is 513 and expects the real `pmat` to fail to fork; the gx10 account had 300 threads (`ps -u noah -L | wc -l`) when measured, below the cap, so nothing is refused. The cap code is not touched by this PR. Not measured on a host whose account runs more than 513 threads.

Known LAB reds (forjar#858) not run: `falsification_untyped_error_sites_ratchet`, `falsification_budget_dry_run`, `falsification_release_workflow_shape`.

Not measured: CI on the merged tree (nothing pushed); `cargo test --workspace`; the clean-room. Open: gate-content sign-off for the two ratchet scripts (changes what they accept), owned by the coordinator.

IMPL-PMAT-896-RECEIPT-END
