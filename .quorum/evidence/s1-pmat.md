# pmat lane — S1

The only new test is tests/falsification_contract_summary_counts_match.rs. It asserts on the parsed contracts, requires at least one contract carrying both fields so it cannot pass on an empty corpus, and was shown red against origin/main's blake3-state-v1.yaml and green with the fix.
