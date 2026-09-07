# Independent review — agy /teamwork-preview — PMAT-165

One /teamwork-preview lane (35-minute budget, sandboxed; it ran the shim-backed test target) reviewed the diff at ec67a34d without seeing the claim lanes.

## Verdict (FAIL)

The independent code review team completed its audit of PMAT-165-publish-from-tag. All constraints were respected (read-only, no real cargo publish, bash/cargo tests passed cleanly). However, the team discovered critical flaws in the script logic, primarily falsifying the author's core claim about `git worktree add --detach` (git automatically detaches on tags, making the claimed mutation guard vacuous). Additionally, they found deadlocks in DRY_RUN multi-crate path dependencies, dev-dependency cycle false positives in the topo sort, and regex substring matching bugs in the index poll. The standard test suite (pv validate, bash lint, cargo test) passed cleanly, proving that a standard claim lane would have falsely approved this PR. The target codebase requires changes.

## Findings, as returned

- T-F1 [measured] scripts/publish-from-tag.sh:31 — `git worktree add --detach` mutation guard on tags is necessary
- T-F2 [asserted] scripts/publish-from-tag.sh:100 — `grep -v '^!! target/'` correctly handles ignored target/
- T-F3 [asserted] scripts/publish-from-tag.sh:142 — `cargo search` correctly parses index version
- T-F4 [asserted] scripts/publish-from-tag.sh:167 — `DRY_RUN=1` succeeds with path dependencies
- T-F5 [asserted] scripts/publish-from-tag.sh:120 — Topological sort correctly handles cycles

## What became of it

Its five mechanisms were each answered in ea89439b: the vacuous --detach guard claim (the header now names the real guard and case c pins git-common-dir), the dead target/ grep, the grep -qF index poll (now cargo info NAME@VERSION with an anchored fallback and a bounded exit), the dev-dependency cycle (filtered), and the DRY_RUN limit with unpublished path dependencies (accepted as cargo's own, D7). It also stated that the green suite would have approved the branch — which is why the scratch-file defect the contract lane measured is the one that most changed the script.
