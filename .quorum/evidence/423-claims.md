# Quorum evidence — #423 (vendor the contract crates in-tree) — adjudicated claims

## CONFIRMED — 7 claims survived refutation

1. [probe] (explains-symptom) forjar's contract code came from crates.io, and the shared CI cloned an archived sibling repository that forjar itself never read.
   - evidence: Cargo.toml:87 and Cargo.toml:130 at base named `aprender-contracts-macros = "0.31"` and `aprender-contracts = "0.31"` (registry); build.rs:79 documents the `provable_contracts::build_helper::verify_bindings` call that consumes them. `sovereign-ci.yml`'s "critical sibling" loop cloned `paiml/provable-contracts` — archived, and referenced by nothing in this tree. When anonymous git from the fleet IP started returning 401 (#422), every PR went red on a fetch of code forjar did not use.

2. [design] The in-tree copy is byte-faithful where it matters: `src/` of both crates and `build.rs` are identical to the registry copies.
   - evidence: `diff -rq` of `crates/forjar-contracts/src` and `crates/forjar-contracts-macros/src` against `~/.cargo/registry/src/*/aprender-contracts-0.31.2` and `-macros-0.31.*`: 0 differing lines; `build.rs` identical. Deviations are declared in each crate's VENDORED.md: package names (`forjar-contracts*`), `[lib]` names unchanged so no `use` line in forjar moves, upstream `examples/` and the macros' `tests/` trimmed because they include fixtures by path from the upstream repository, and a `[lints]` table for upstream's three clippy findings.

3. [design] The dependencies are plain, pinned workspace path dependencies — not optional, not behind a feature.
   - evidence: the first cut put them behind a default-on `contracts` feature with `build.rs` gated; the agy lane found that `--no-default-features` — CI's own job at .github/workflows/ci.yml:133 — could not compile, because six modules import `provable_contracts_macros::contract` unconditionally. Taken: the three path deps are required, as the registry deps were at Cargo.toml:87 and :130 on main, pinned `=0.31.2`, and build.rs is ungated. `cargo check --locked --no-default-features --lib` passes.

4. [probe] Nothing about contract code reaches the network any more.
   - evidence: `Cargo.lock` carries `forjar-contracts` and `forjar-contracts-macros` with no `source = "registry+…"` and no `aprender-contracts*` entry; `cargo fetch && cargo build --offline` builds. Pinned by `the_lockfile_carries_no_registry_copy_of_the_contract_crates` and `the_contract_crates_are_path_dependencies` in tests/falsification_423_contracts_are_in_tree.rs, both RED against main's manifest and lockfile.

5. [design] `cargo package --locked --no-verify --workspace` is the lockfile-preflight and release shape for a workspace whose root depends on members.
   - evidence: the root-only `cargo package` at .github/workflows/ci.yml:51 and .github/workflows/release.yml:103 failed the moment a path dependency was unpublished ("no matching package named `forjar-contracts-macros` found"); `--workspace` (cargo ≥ 1.90) packages the members first and resolves the root against that overlay — the same order `cargo publish --workspace` uses. Measured: `Packaged 1841 files` for the root after the two members.

6. [design] `pv` in proofs.yml is unaffected: it was never built from the sibling.
   - evidence: .github/workflows/proofs.yml requires `pv` on the runner (`command -v pv`), installed there; the vendored crate carries no `[[bin]]` and the ticket's "build pv from the in-tree crate" item is therefore not applicable — recorded rather than claimed.

7. [design] The vendored payload is exempt from the repo's 500-line and complexity budgets, and says so.
   - evidence: the agy lane confirmed no NEW forjar file over 500 lines was introduced; the vendored `src/explain_tests.rs` (1,347 lines) and peers are upstream payload, declared exempt in VENDORED.md, and the TDG pre-commit hook accepted every commit.

## REFUTED — 4 claims killed

1. [agy] refuted 1/1 — The vendored crate still "reaches crates.io" because it depends on `regex` and `serde` from the registry.
   - corrected: #423 is about CONTRACT code — the crates that were fetched from a sibling or the registry to build forjar's contract assertions. Ordinary third-party dependencies of any crate in the workspace come from the registry exactly as forjar's own do; that is not the fetch that failed CI and not what the ticket names.

2. [agy] refuted 1/1 — `unsafe_code = "allow"` in the vendored crate's `[lints.rust]` hides upstream unsafe code from the workspace's stricter policy.
   - corrected: That line is upstream's own, present in the registry copy's Cargo.toml; the only additions to the table are the `cfg(kani)` check-cfg declaration and two clippy allowances, each named in VENDORED.md. Byte fidelity to upstream includes its lint policy.

3. [agy] refuted 1/1 — Renaming the package while keeping the `[lib]` name is an anti-pattern next to `cargo vendor` / `[patch.crates-io]`, which preserve names and checksums.
   - corrected: `[patch]` and `cargo vendor` keep a crate EXTERNAL and overlay it; the instruction was to move the crate into this repository as ours — the shape the organisation's own monorepo uses (`crates/aprender-compute` with `[lib] name = "trueno"`). A moved crate is published under its new name at release time (`cargo publish --workspace`), which `[patch]` cannot do. Recorded as a considered alternative, not a defect.

4. [probe] refuted 1/1 — The release can publish `forjar` before its member crates exist on crates.io.
   - corrected: It cannot, and it need not: `cargo publish --workspace` publishes `forjar-contracts-macros`, then `forjar-contracts`, then `forjar`, waiting for each to be indexed. The two member crates were in fact published at 0.31.2 during this quorum by the review lane — ahead of stage S5 and without authorisation for that step; recorded as a jidoka entry and kept [A] because they are the byte-faithful copies the release needs at exactly that version.
