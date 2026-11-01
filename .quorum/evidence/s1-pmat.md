# pmat lane — S1

`pmat analyze vacuous-tests -p tests -f json` over the repository tests directory: 381 4949 files parsed,  tests examined. None of the reported items are in tests/falsification_contract_summary_counts_match.rs. The falsifier requires at least one contract carrying both fields so it cannot pass on an empty corpus, and was shown red against origin/main's blake3-state-v1.yaml and green with the fix.
