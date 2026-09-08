# pmat lane — PMAT-206 — analyze_vacuous_tests

Tool: `pmat 3.39.0`, `pmat analyze vacuous-tests -f json`, over the branch at HEAD, filtered to the touched test paths.

```text
tests_examined    = 19505
files_parsed      = 2136
vacuous (repo)    = 376
in touched paths  = 0
```

No test was added by this branch; the existing suites (observe: 75, purifier: 51, the two chmod falsification files: 24, the example run) were re-run green after every reduction.

## Touched paths and what re-ran

- `src/core/observe/mod.rs` — `cargo test --lib -- observe`: 75 passed after the table replaced the match.
- `src/core/purifier_sec017.rs` — `cargo test --lib -- purifier`: 51 passed; `tests/falsification_chmod_path_is_not_a_mode.rs` 9 passed and `tests/falsification_chmod_gate_survives_review.rs` 15 passed after each decomposition.
- `examples/cron_secret_encryption_falsification.rs` — `cargo run --example cron_secret_encryption_falsification` ends with its survival line; every one of the fourteen criteria still asserts.
- `scripts/cb200-ratchet.sh` — `bash -n`, `bashrs lint` at 0 errors, and two live runs (cache fresh; cache older than a touched source).
- `cargo clippy --all-targets --locked -- -D warnings` — 0 errors at HEAD.

## Limit

The reductions are behaviour-preserving by construction, so no test can tell the before from the after; the evidence that they landed is the ratchet's own number, 654 with the stale cache and 651 with a fresh one, in the claims dossier.
