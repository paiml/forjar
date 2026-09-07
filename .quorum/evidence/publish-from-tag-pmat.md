# pmat lane — PMAT-165 — analyze_vacuous_tests

Tool: `pmat 3.39.0`, `pmat analyze vacuous-tests -f json`, run over the branch at HEAD and filtered to the touched test paths.

```text
tests_examined    = 19357
files_parsed      = 2108
vacuous (repo)    = 360
in touched paths  = 0
```

Touched test paths:

- tests/falsification_publish_from_tag.rs
- tests/publish_from_tag_harness/mod.rs

## Limit

The count is what the tool measured; discrimination was measured by the orchestrator by replacing `git worktree add` with a clone (the receipt's falsification block) and by the worker's RED commit 02d65ef1 before ea89439b.
