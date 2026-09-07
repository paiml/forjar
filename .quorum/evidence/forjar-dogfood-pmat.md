# pmat lane — PMAT-163 — analyze_vacuous_tests

Tool: `pmat 3.39.0`, `pmat analyze vacuous-tests -f json`, run over the branch at HEAD and filtered to the touched test paths.

## Result

```text
tests_examined    = 19362
files_parsed      = 2111
vacuous (repo)    = 360
conditional_skips = 4
in touched paths  = 1
```

Touched test paths:

- crates/forjar-contracts/src/build_helper_corpus_tests.rs
- crates/forjar-contracts/src/lint/mod_tests.rs
- crates/forjar-contracts/src/query/index_corpus_tests.rs
- crates/forjar-contracts/src/query/query_tests.rs
- crates/forjar-contracts/src/query/query_tests_coverage.rs
- crates/forjar-contracts/src/scoring/codebase_tests.rs
- crates/forjar-contracts/src/scoring/scoring_tests.rs
- tests/falsification_dogfood_release_check_pr_window.rs
- tests/falsification_dogfood_scripts_declare_mutations.rs
- tests/falsification_dogfood_skill_is_named.rs
- tests/fixtures/dogfood/Makefile
- tests/fixtures/dogfood/local-files.yaml
- tests/fixtures/dogfood/single-stack.yaml
- tests/fixtures/dogfood/stack-a.yaml
- tests/fixtures/dogfood/stack-b.yaml

## Hits in touched paths

- crates/forjar-contracts/src/query/query_tests_coverage.rs:256 `coverage_map_enrichment` (None)

## Limit

The count is what the tool measured; the 43 aprender-corpus tests are `cfg_attr(ignore)` with a reason each and the exact ratchet fails if that count moves. Discrimination was measured separately: deleting the skill's `name:` line turns the skill-name guard red (the receipt's falsification block).
