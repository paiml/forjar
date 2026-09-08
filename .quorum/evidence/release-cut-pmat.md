# pmat lane — PMAT-165 — analyze_vacuous_tests

Tool: `pmat 3.39.0`, `pmat analyze vacuous-tests -f json`, over the branch at HEAD, filtered to the test files this cut adds.

```text
tests_examined    = 19518
files_parsed      = 2139
vacuous (repo)    = 376
in touched paths  = 0
```

The cut adds two test files, both shim-backed and both observed RED against the gate as it stands on main:

- `tests/falsification_crux_gate_reads_the_release_section.rs` — four cases (the cut reads the version section; before the cut `[Unreleased]` still wins; an empty release section is red; the row is still required).
- `tests/falsification_coverage_gate_mutation_scope.rs` — five cases (a tests-only diff passes and says why; a src/ change still demands mutants; a survivor is red; caught mutants pass; a diff with no Rust change never invokes the tool).

## Limit

The release cut's other content is documents; their falsifier is the gate set itself, which is run in full above.
