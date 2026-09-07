# pmat lane — PMAT-166 — analyze_vacuous_tests

Tool: `pmat 3.39.0`, `pmat analyze vacuous-tests -f json`, run over the branch at HEAD and filtered to the touched test paths.

```text
tests_examined    = 19354
files_parsed      = 2107
vacuous (repo)    = 360
in touched paths  = 0
```

Touched test paths:

- tests/falsification_release_workflow_shape.rs

## Limit

The diff is two workflow files plus a shape test and its contract; the tool measures the test file. Discrimination was measured by restoring both workflow files from origin/main and running the shape test (the receipt's falsification block).
