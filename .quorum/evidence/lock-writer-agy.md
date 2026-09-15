# PMAT-565 — the agy round

    lane=quorum width=3 writes=false sandbox=true mode=plan
    schema=quorum-lane-schema.json timeout=25m
    out_dir=.../paiml-implement/agy/PMAT-565/<session>/ph3
    not_before=1789488090 (the dispatch instant)
    repo_root=<the PR worktree>, reviewed_commit=a0070731
    base for the diff: PMAT-564-drift-declines-on-empty-scope, not main

Composed by the paiml-agy-delegate through `agy-lane.sh --repo-root
<worktree>`; each lane in a self-contained sandbox clone, tree witness
asserted before, byte-identical after, removed. The dispatch-local model
config was named `models.config.json` so `fanout.sh` counted three children,
not four (the PMAT-564 round's edge). No `--concurrent-scope` was declared.
Every lane was briefed NO WRITES and none wrote; no KEPT, no exit 3, no
exit 4. The delegate returned within budget (20 tool uses).

A first dispatch of this round was interrupted by the operator before any
lane launched; the round above is the re-dispatch, with a fresh
`--not-before`.

Result: 2 FAIL / 1 PASS, `agreed=false`. The FAILs were re-executed here and
acted on in adad4e6e.
