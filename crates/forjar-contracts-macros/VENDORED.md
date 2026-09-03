# Vendored Dependency

A copy of `aprender-contracts-macros` 0.31.2 from the crates.io registry cache
(paiml/aprender), the proc-macro crate behind `#[contract]`.

- **Upstream crate**: `aprender-contracts-macros` (crates.io library name: `provable_contracts_macros`)
- **Version**: 0.31.2
- **Package name here**: `forjar-contracts-macros`; `[lib] name` is unchanged so no `use` line in forjar moves.

## Deviations from the registry copy

- `src/` is byte-identical (checked with `diff -rq` against the registry copy).
- `tests/` and its `[[test]]` targets are not carried: they exercise the macro
  against upstream fixtures that are not in this repository. forjar's own
  `tests/contract_macros_are_not_inert.rs` covers the macro as forjar uses it.
- Package name and version are pinned by the workspace; nothing else in the
  manifest was edited.
