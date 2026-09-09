# Quorum evidence — PMAT-219 — mechanical lane

`analyze_vacuous_tests` over the touched paths: 0. All six cases exercise real code; two of them pass before the fix as well and are guards rather than demonstrations, which is stated in the file.

`cargo test --workspace`: **exit 0, 311 test binaries**. `cargo test --lib baseline_transport`: 6 passed. `cargo fmt --all -- --check`: clean. `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.

`pmat work validate`: passes. `kind-gate.sh`: `kind=code ticket=PMAT-219`. `model-gate.sh`: `model=opus class=opus decision=admit basis=file`.

The digest comparison the lanes could not run, run here:

```
ordinary        file=f92ca07e3206 str=f92ca07e3206 same=true
no_trailing_nl  file=6437b3ac3846 str=6437b3ac3846 same=true
empty           file=af1349b9f5f9 str=d70cbc1aa622 same=false
non_utf8        file=2a7c022c5f18 str=6329f2bdda5d same=false
```

Mutations, each killing exactly its own case:

| mutation | result |
|---|---|
| the predicate reverted to `machine_is_local` | the pepita case fails, the other five stay green |
| the fix reverted entirely | the two original cases fail, the local guard stays green |

`src/core/executor/mod.rs` is untouched by this branch and remains at 500 lines. Gate F's mutation arm is not claimed: it cannot run on this host (PMAT-216).
