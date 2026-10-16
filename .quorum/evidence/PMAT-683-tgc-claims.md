# Claims — forjar#683 follow-up: the falsifier within the pre-commit limits

The fix for #683 merged in #690. Its falsifier's `toolchain_gaps` was
Cognitive 39, so the pre-commit hook refused every commit that brought it in:

    toolchain_gaps (tests/falsification_683_cross_mounts_under_work_root.rs:277) - Cognitive 39 > 25

A merge of main into a branch that lacks the file judges every function in it
as new, so every open PR that merged main was refused. Briefed to every lane
with the full diff against main ec477d93.

- C1: `toolchain_gaps` is now Cognitive 11, Cyclomatic 5; the new `first_gap` is 12 / 6 and `sets_rustup_home` 1 / 2. No function in the file is above 25 / 30 (the highest is `cross_jobs`, Cognitive 22).
- C2: no behaviour change. `first_gap` is the old per-leg loop: an install step only updates `installed` (the old `continue` is now `else if`), and the first step needing a toolchain before a running install is returned (the old `break`). `sets_rustup_home` is the old workflow-or-job env test.
- C3: the falsifier still discriminates: green on main's workflows; red when `first_gap` never reports; red with release.yml at 1d3e2d45, naming all four Linux legs.
- C4: the diff touches only the test file and the ticket's roadmap entry; no workflow changes.
