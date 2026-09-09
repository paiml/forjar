# Quorum evidence — PMAT-220 — mechanical lane

`cargo test --workspace`: **exit 0, 313 test binaries**. `cargo test --lib output_verify`: 9 passed. The rule file: 2 passed. `cargo fmt --all -- --check`: clean. `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.

`analyze_vacuous_tests` over the touched paths: 0. `pmat work validate`: passes. `kind-gate.sh`: `kind=code ticket=PMAT-220`. `model-gate.sh`: `model=opus class=opus decision=admit basis=file`.

`pmat tdg`: `src/core/executor/output_verify.rs` was refused by the pre-commit ratchet at A (93.5) when the two new cases repeated the fixture shape; folding them into one table brought it to A+ (95.5). `src/core/executor/mod.rs` is left at exactly 500 lines with no new comment, because the previous review refused shrinking unrelated prose to make room and was right.

Mutations, each killing exactly its own case:

| mutation | result |
|---|---|
| the loose predicate restored at `output_verify` | the behavioural case and the tree rule both fail |
| a fourth call added in `src/core/state/reconstruct.rs` | the tree rule names that file |
| a trailing-comment fake fix | the tree rule fails |

Gate F's mutation arm cannot run on this host (PMAT-216) and no `cargo mutants` figure is claimed.
