# Vendored Dependency

This directory contains a vendored copy of the `aprender-contracts` and `aprender-contracts-macros` crates from the `paiml/aprender` repository.

- **Upstream Crate**: `aprender-contracts` / `aprender-contracts-macros` (crates.io library names: `provable_contracts`, `provable_contracts_macros`)
- **Version**: 0.31.2
- **Source Commit**: d6c6c6f8fdaa09cfa88e66f85cfbe03108d7d6dd (paiml/aprender)

This crate was copied from the registry cache to guarantee exact byte-for-byte fidelity with what `forjar` previously consumed from crates.io.

## Lint debt carried from upstream

The workspace's `cargo clippy --all-targets --workspace -- -D warnings` finds
three things in this crate that upstream did not lint against: `#[cfg(kani)]`
without a `check-cfg` declaration, one `manual_strip`, one
`manual_is_multiple_of`. They are allowed in this crate's `[lints]` table
rather than fixed, so the source stays byte-identical to the registry copy;
fix them upstream and re-vendor.

## Trimmed

`examples/` (and any benches) are removed: they `include_str!` contract YAML
by path from the upstream repository (`../../../contracts/*.yaml`), which
does not exist here, so `cargo clippy --all-targets --workspace` could not
build them. forjar uses the library, the build helper and the macro only.

## Repo rules that do not apply to the payload

Several vendored files exceed the repo's 500-line budget (for example
`src/explain_tests.rs`); they are upstream payload, not forjar code, and are
exempt. `build.rs` looks for `../../contracts/*.yaml` from the upstream
repository and finds none here, so it reports "0 preconditions, 0
postconditions" — the crate's own binding self-check is inert in this tree;
forjar's `build.rs` verifies forjar's bindings.
