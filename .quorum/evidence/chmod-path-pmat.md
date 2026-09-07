# pmat lane — PMAT-204 — analyze_vacuous_tests

Tool: `pmat 3.39.0`, `pmat analyze vacuous-tests -f json`, over the branch at HEAD, filtered to the touched test path.

```text
tests_examined    = 19432
files_parsed      = 2122
vacuous (repo)    = 360
in touched paths  = 0
```

Touched test path: `tests/falsification_chmod_path_is_not_a_mode.rs` (18 tests, every behavioural one through `validate_script`).

## Limit

Vacuity here was measured the hard way as well: each of the 45 shapes was run against the pre-fix baseline, and a test asserting a shape both commits treat identically is labelled a control in the file rather than presented as evidence of the fix.
