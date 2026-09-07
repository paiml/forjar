# pmat lane — PMAT-164 — analyze_vacuous_tests

Tool: `pmat 3.39.0`, `pmat analyze vacuous-tests -f json`, run over the branch at HEAD and filtered to the touched test paths.

```text
tests_examined    = 19356
files_parsed      = 2107
vacuous (repo)    = 360
in touched paths  = 0
```

Touched test paths:

- tests/falsification_crux_audit_shape.rs

## Limit

The diff is one audit document, a shape test and its contract; discrimination was measured by emptying the audit and running the shape test (the receipt's falsification block).
