# Judges — tool-requiring jobs on bare metal (PMAT-598)

Round count and heads: see `PMAT-598-lanes.md`.

## CONFIRMED

1. [diff] C1 — Exactly five jobs are re-routed across the workflows, and every other job keeps its previous runs-on unchanged, which three rounds of lanes each verified by diffing every runs-on line between the base and the head rather than by reading the claim.
   - evidence: the discovery test at `tests/falsification_tool_jobs_run_where_the_tools_are.rs:246` asserts the unfiltered-lib-test jobs by walking every workflow, and its anti-vacuity loop at `tests/falsification_tool_jobs_run_where_the_tools_are.rs:292` names the three it must find.
2. [diff] C2 — The change only narrows the eligible runner set: every hunk appends one label to an existing array, and no condition is widened, no step skipped, and no continue-on-error added, while both prover assertions still exit 1 when their tool is absent.
   - evidence: confirmed by all nine lane verdicts; mutation.yml's pre-existing continue-on-error on the mutants step is untouched by this diff.
3. [fleet] C3 — The label set matches exactly the fifteen intel clean-room runners, which the Claude lane verified independently against the live runner API in two rounds rather than trusting the table in the brief, and the set is non-empty so no pinned job can queue forever.
   - evidence: X64 excludes every gx10 runner, clean-room excludes yoga-gpu, yoga-eph and the perf-solo intel runner, and intel excludes every yoga-build container.
4. [test] C4 — Name-filtered and targeted runs cannot reach nas_archive's rsync-requiring safety test and are correctly left unpinned, because that test is a lib unit test in the source tree and is reached only by an unfiltered lib run.
   - evidence: `tests/falsification_tool_jobs_run_where_the_tools_are.rs:305` asserts every narrowed form stays unmatched, including the exact convergence, examples-validate and doctests commands in the tree.
5. [scope] C6 — The note on the reusable sovereign-ci test job is a stated limit rather than a claim of coverage, since that job's runs-on lives in another repository and no detector reading this repository's workflows can see it.
   - evidence: two lanes with the commit message in their brief confirmed it; the one refutation came from a brief that omitted the commit message.

## REFUTED

1. [test] R1 — The first draft's commit message claimed four declared mutations while the test file declared exactly one, and all three round-1 lanes found the discrepancy independently by reading the file rather than the message.
   - corrected: every mutation is now declared in the file header where a reader of the test finds it.
2. [scope] R2 — Pinning only kani, lean and coverage missed stress.yml and mutation.yml, which run unfiltered lib tests on the same label set and so reach the rsync-requiring safety test; neither had ever been observed on a yoga runner, so the lottery was live but undrawn.
   - corrected: both pinned, and the test now discovers such jobs from the workflows at `tests/falsification_tool_jobs_run_where_the_tools_are.rs:246` instead of listing them by hand.
3. [fleet] R3 — The comments named yoga-build, yoga-build2 and yoga-build3 as the containers, but those runners are ephemeral and only yoga-build3 was registered when round 1 queried the live API, so the phrase read as a claim about the present fleet.
   - corrected: the comments now say the runners are ephemeral and that all three were observed running these jobs on 2026-09-20.
4. [test] R4 — The substring detector missed bare cargo test, the workspace and all-targets forms, cargo nextest run, and a flag between the lib flag and the separator, each a false negative that would have left a future job silently unpinned.
   - corrected: tokenized, with every named form asserted on both edges at `tests/falsification_tool_jobs_run_where_the_tools_are.rs:305`.
5. [test] R5 — The first tokenizer stopped reading at the separator, so a name filter placed after it was classed as unfiltered and would have pinned a narrowed run, the safe direction but still a misclassification.
   - corrected: the test binary's arguments are read too at `tests/falsification_tool_jobs_run_where_the_tools_are.rs:219`.
6. [claim] R6 — C5 said every declared mutation reddens exactly its own arm, but the fourth mutation reddens both anti-vacuity floors by design, and the file already said so while the claim did not.
   - corrected: recorded as a wording refutation; the file's own description of the fourth mutation was already exact.
7. [test] R7 — Round 2's fix read the value of a libtest flag after the separator as a name filter, so a run skipping one pattern was classed as narrowed although it still reaches the safety test: a false negative introduced by the fix for the previous one.
   - corrected: libtest flags that take a value are skipped with their value, listed at `tests/falsification_tool_jobs_run_where_the_tools_are.rs:197`.
8. [test] R8 — A lane reported a command chained with a shell separator as classified correctly, and adding it as an asserted case failed at once because the tokenizer read the separator as a positional filter and called the unfiltered first command narrowed.
   - corrected: lines are split on shell separators at `tests/falsification_tool_jobs_run_where_the_tools_are.rs:142` before classifying.
9. [claim] R9 — The commit message said the detector was split across four functions, a count that was true when written and stale after later edits added three more, which a round-3 lane caught by counting.
   - corrected: the sentence no longer gives a count and says why.
