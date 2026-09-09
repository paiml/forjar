# Quorum evidence — PMAT-217 — mechanical lane

`cargo test --workspace`: **exit 0**, 311 test binaries green, run from `~/src/forjar` where a sibling aprender checkout is visible. This is the command that reported one failure before the fix and that reproduced on `origin/main` at 54e36f13.

`cargo test -p forjar-contracts --lib`: 1332 passed, 0 failed, 44 ignored, matching `IGNORED_EXPECTED=44`.

`cargo test -p forjar-contracts --lib --features aprender-corpus`: 39 failed, matching `APRENDER_ANNOTATIONS=39` and the 39 annotations counted in the tree. Three independent routes to the same number.

`cargo fmt --all -- --check`: clean. `cargo clippy --all-targets --all-features -- -D warnings`: exit 0.

`pmat work validate`: passes. `kind-gate.sh`: `kind=code ticket=PMAT-217`. `model-gate.sh`: `model=opus class=opus decision=admit basis=file`.

Mutations, each run by hand and each killing exactly the case it should:

| mutation | result |
|---|---|
| the sibling check restored inside the test body | `no_test_decides_to_assert...` FAILS |
| the same defect hidden in a helper, in a non-exempt file | FAILS — the capability this round added |
| the shim's count reverted to 43 | the ratchet invariant FAILS |
| `Cargo.toml`'s figure reverted to 38 | the ratchet invariant FAILS |

Gate F's mutation arm is not claimed: it cannot run on this host at all (PMAT-216).
