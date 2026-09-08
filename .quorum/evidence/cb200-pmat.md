# pmat lane — PMAT-206 — analyze_vacuous_tests

Tool: `pmat 3.39.0`, `pmat analyze vacuous-tests -f json`, over the branch at HEAD, filtered to the touched test paths.

```text
tests_examined    = 19505
files_parsed      = 2136
vacuous (repo)    = 376
in touched paths  = 0
```

No test was added by this branch; the existing suites (observe: 75, purifier: 51, the two chmod falsification files: 24, the example run) were re-run green after every reduction.
