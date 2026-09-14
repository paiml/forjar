# Quorum evidence — PMAT-237 — adjudicated claims

One round of three lanes on 479d8edd, base pinned at 0c1277c6. **One lane returned**; one wrote to the operator's repository and was stopped, and one returned nothing. The round is one lane deep and is recorded as such. Its two substantive findings were the two worst outcomes the design had, and both are closed at HEAD.

Citations resolve at the merge base: `tests/falsification_dogfood_scripts_declare_mutations.rs:1` is the suite that already holds every gate script to a declared mutation, which is the shape this ticket's classifier follows.

## CONFIRMED

1. [measurement] THE COST IS REAL — one PR spent about 131 job-minutes: ledger-replay 34, dogfood 26, benchmark 18, the workspace suite 14, coverage 12, and the rest across lint, examples and doctests. Eight of the ten most recently merged PRs touched no `src/` at all.
- evidence: the returning lane checked the run durations and the file lists with `gh`; the orchestrator measured the same ten PRs and recorded the per-job split in `docs/audits/logs/PMAT-237-class-census.log`. The suite that pins what follows from it opens at `tests/falsification_pr_lane_runs_what_the_change_can_break.rs:1`.

2. [allow-list] THE DECISION FAILS TOWARD RUNNING — an unclassified path, an unreadable diff and an empty file list are all code, and the script says why in its own header.
- evidence: the returning lane read the script and drove it; `tests/falsification_pr_lane_runs_what_the_change_can_break.rs:244` is the case that fails the build if an unclassified path ever reads as harmless, and `tests/falsification_pr_lane_runs_what_the_change_can_break.rs:257` the one for an unreadable list.

3. [wiring] THE HEAVY JOBS CARRY THE CONDITION AND THE CLASS IS COMPUTED ONCE — five jobs in ci.yml, `ledger-replay` in proofs.yml and `benchmark` in bench.yml gate on it, through one composite action that calls the one script.
- evidence: the returning lane read all four workflows; `tests/falsification_pr_lane_runs_what_the_change_can_break.rs:141`'s neighbours assert it by reading the YAML, and a mutation that ungates one job turns that rule red.

4. [release-untouched] `make dogfood-release` AND `scripts/dogfood/*` ARE UNTOUCHED — the release gate still runs everything, so this redistributes cost rather than lowering a floor.
- evidence: the returning lane diffed the tree; `git diff --name-only main...HEAD` names four workflows, one composite action, one script, `tests/falsification_pr_lane_runs_what_the_change_can_break.rs:1` and the roadmap — and nothing under `scripts/dogfood/`.

5. [self-review] THE CHANGE DOES NOT SKIP ITS OWN REVIEW — it touches `.github/**`, `scripts/**` and `tests/**`, so the classifier calls it code and every heavy job runs on this PR.
- evidence: the orchestrator ran the classifier over this branch's own diff (`docs/audits/logs/PMAT-237-class-census.log`); the returning lane confirmed it. `tests/falsification_pr_lane_runs_what_the_change_can_break.rs:85` is the case that makes a workflow or a script code.

6. [actionlint] THE FINDING SET IS UNCHANGED — 22 findings on both sides, and all four edited workflows parse as YAML.
- evidence: measured on both trees by the orchestrator and confirmed by the returning lane.

## REFUTED

7. [unmeasured-class] THE WORST OUTCOME THE DESIGN HAD — if `classify` fails or is cancelled its output is EMPTY, every heavy job's `if` is false so they all skip, and the gate read an empty class as "not code" and passed. A change nothing tested would have merged green.
- evidence: the returning lane traced `ok()` in ci.yml's gate: `[ "$CODE" != "true" ]` is true for an empty value, so a skipped job was accepted. It is the single most valuable thing the round produced.
- corrected: the gate checks `needs.classify.result` and refuses an empty class, and proofs.yml's aggregator defaults an absent class to code (`${CODE:-true}`). The case that pins both is in `tests/falsification_pr_lane_runs_what_the_change_can_break.rs:423`'s file, appended after the wiring rules.

8. [rename] A RENAME OUT OF THE SOURCE TREE READ AS HARMLESS — `git diff --name-only` reports a rename as its NEW path alone, so moving `src/thing.rs` to `docs/thing.rs` listed one harmless path while deleting a source file.
- evidence: the returning lane worked it out from git's own behaviour rather than from the diff.
- corrected: `--no-renames` splits a rename into a delete and an add, and the deletion is code; the case in `tests/falsification_pr_lane_runs_what_the_change_can_break.rs:423`'s file asserts both the flag and the classification.

9. [claim-wordings] TWO CLAIMS WERE WRONG AS WRITTEN — that `README.md` and `contracts/**` are "under the harmless prefixes" (they are not; they are separate arms), and that "nine cases drive the script" (six do; the other three test the wiring).
- evidence: the returning lane read the case list and `tests/falsification_pr_lane_runs_what_the_change_can_break.rs:72` onward, where the six classifier cases end and the wiring rules begin.
- corrected: the claim set and the receipt both say six cases drive the classifier and the rest pin the wiring, and the script's arms are listed in the order they are read.

## Found by the orchestrator, not by the round

The harmless set in the first draft called all of `docs/**` and `CHANGELOG.md` harmless. Grepping the tree for `CARGO_MANIFEST_DIR`-joined paths showed that `CHANGELOG.md`, `docs/audits/crux-*`, `docs/specifications/*`, `docs/book/*` and `docs/mcp-schema.json` are each read BY NAME by a test — so a change to any of them would have skipped the suite that reads it. All are excluded now, and `every_record_path_a_test_reads_is_classified_as_code` re-derives the list from the tree so it cannot rot. The same grep corrected the headline: **three** of ten PRs skip the heavy jobs, not eight.

Coverage was removed from the gated set on the operator's instruction and on its own merits, and the floor was measured on this branch rather than asserted: 95.69% lines, `--fail-under-lines 95` exit 0.
