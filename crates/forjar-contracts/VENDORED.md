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

## The aprender corpus tests (#452)

The same missing corpus that makes `build.rs` inert makes 38 of the crate's
1,376 unit tests fail: they read `contracts/aprender/binding.yaml`, copy
`contracts/softmax-kernel-v1.yaml`, query the index for `softmax`/`RMSNorm`, or
assert `contracts.len() > 100`. All of those describe **aprender's** corpus.
forjar vendors the crate, not the corpus, so every one of them fails on
`NotFound` or an empty result set — a statement about the vendoring, never about
the code under test, and 38 permanent reds that make a real regression in this
crate invisible.

They are gated by

```rust
#[cfg_attr(not(feature = "aprender-corpus"), ignore = "<reason>")]
```

which is deliberately **not** `#[cfg(...)]`: the bodies still compile on every
run, so they cannot rot against an API change, and libtest prints the reason per
test, so the exclusion is counted and legible in the output rather than silent.
Run them with the corpus present:

```bash
cargo test -p forjar-contracts --lib --features aprender-corpus
```

`scripts/dogfood/coverage.sh` (gate F) asserts the ignored count against a
recorded ceiling that may only shrink, so a newly broken test cannot be parked
behind this feature without the gate saying so.
