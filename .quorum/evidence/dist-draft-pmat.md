# pmat lane — PMAT-208 — analyze_vacuous_tests

Tool: `pmat 3.39.0`, `pmat analyze vacuous-tests -f json`, over the branch at HEAD, filtered to the touched test file.

```text
tests_examined    = 19519
files_parsed      = 2139
vacuous (repo)    = 376
in touched paths  = 0
```

The branch adds one rule to `tests/falsification_release_workflow_shape.rs` and changes no other test. Its non-vacuity is measured rather than argued: the whole file is 8 passed on this branch and 7 passed / 1 failed against the workflow as it stands on main, the failure being rule 8 itself.

## Limit

The rest of the diff is two receipts and a roadmap row; their falsifier is the release that already happened and the gate run recorded in them.
