# PMAT-557 — the agy round

    lane=quorum width=3 writes=false sandbox=true mode=plan
    schema=quorum-lane-schema.json timeout=25m
    out_dir=.../paiml-implement/agy/PMAT-557/<session>/ph2
    not_before=1789495652 (the dispatch instant)
    repo_root=<the booking worktree>, reviewed_commit=2edf43c7

Composed by the paiml-agy-delegate through `agy-lane.sh --repo-root
<worktree>` with lane models passed as `models.config.json` in the out dir (not
`lane-*.json`, which `fanout.sh` would count as a lane). No
`--concurrent-scope`. The delegate returned within budget (18 tool uses).

Harness facts the round surfaced:

- the sandbox has no `gh` auth, so every gate that reads GitHub is UNMEASURED
  inside a lane — a booking review can check the ledger against git, and the
  orchestrator must re-run the GitHub half;
- a writes=false lane still wrote a file into its own clone (lane 1), and
  `agy-lane.sh` caught it;
- the paiml-implement skill's `kind-gate.sh` refuses a `kind:triage` branch
  that touches `docs/roadmaps/releases.yaml`, while this repository's own
  `scripts/quorum-gate.sh` admits the release ledger on the triage rail since
  PMAT-226 — the two rails disagree, and the repository's governs the merge.

Result: 1 FAIL / 2 PASS, `agreed=false`. The FAIL was checked against the
commit itself and refuted.
