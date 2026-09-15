# PMAT-562 — the agy round

    lane=quorum width=3 writes=false sandbox=true mode=plan
    schema=quorum-lane-schema.json timeout=25m
    out_dir=.../paiml-implement/agy/PMAT-562/<session>/ph3
    not_before=1789479913 (the dispatch instant)
    repo_root=<the PR worktree>, reviewed_commit=d1347afe

Composed by the paiml-agy-delegate through `agy-lane.sh --repo-root <worktree>`,
which cloned the worktree into a self-contained sandbox clone per lane (a
`--shared` clone's alternates point outside the sandbox and every git command
fails, measured on an earlier ticket), asserted the git config and the tree
witness before the lane ran, and asserted the clone byte-identical after.
No `--concurrent-scope` was declared: nothing else was dispatched beside the
lanes, so the whole checkout was asserted.

Every lane was briefed NO WRITES: no edit, no push, no scratch cargo project
(a lane on the previous ticket left an 8 GB `target/` behind).

The delegate hit its 30-turn cap before returning a receipt — the same
harness defect recorded on PMAT-547 and PMAT-560 (paiml-implement#141). All
three lane JSONs and `reduce.json` were on disk and were read directly; the
reduction was the delegate's own (`lane-reduce.sh` had already run), so no
verdict here was reconstructed by the orchestrator.

Result: 1 FAIL (lane 1), 1 PASS (lane 3), 1 NO-VERDICT (lane 2, API 503).
`agreed=false, partial=true`. The FAIL was acted on and every item it named is
closed in `22a53170`; the sandbox test failure it also reported was re-run
here and found green.
