# PMAT-564 — the agy round

    lane=quorum width=3 writes=false sandbox=true mode=plan
    schema=quorum-lane-schema.json timeout=25m
    out_dir=.../paiml-implement/agy/PMAT-564/<session>/ph3
    not_before=1789484590 (the dispatch instant)
    repo_root=<the PR worktree>, reviewed_commit=bd1664a8
    base for the diff: PMAT-562-drift-exits-on-drift, not main

Composed by the paiml-agy-delegate through `agy-lane.sh --repo-root
<worktree>`; each lane in a self-contained sandbox clone with the tree
witness asserted before and the clone asserted byte-identical after. The
delegate's dispatch-local model config first matched `fanout.sh`'s
`lane-*.json` glob (it read children=4); the delegate renamed it before
`lane-reduce.sh` ran and re-measured 3 — recorded as a harness edge, not a
lane. No `--concurrent-scope` was declared: nothing else was dispatched, so
the whole checkout was asserted.

Every lane was briefed NO WRITES and no scratch cargo projects. Lane 1 wrote
a three-line file into its own clone anyway; `agy-lane.sh` kept the clone
and said so on stderr (a finding, not a detail); the shared checkout was
untouched.

The delegate returned its receipt within budget this time (21 tool uses).

Result: 1 FAIL (lane 1) / 2 PASS. `agreed=false`. The FAIL was re-executed
here and acted on in 22dcb702; the one point on which two lanes agreed and
were wrong (N pre-expansion) was settled by reading `load_drift_config`.
