# pmat MCP lane — #423

`analyze_vacuous_tests` over the whole worktree: 19109 tests examined. Five
`no-failure-mode` findings sit inside the VENDORED payload
(`crates/forjar-contracts/src/*_tests.rs`), upstream tests carried byte-for-
byte and accepted as such in the receipt; ZERO in the falsification file or
in any forjar file the branch touches.

Falsification (tests/falsification_423_contracts_are_in_tree.rs, three
cases): the two manifest/lockfile readers are RED against main's `Cargo.toml`
(registry `aprender-contracts*` deps at lines 87 and 130) and `Cargo.lock`
(registry entries for both); the third is a link-time fact — it compiles only
because `provable_contracts` resolves through the workspace.

Gates: `cargo check --locked --no-default-features --lib` (CI's job) passes;
`cargo package --locked --no-verify --workspace` packages all three crates;
`cargo build --offline` after `cargo fetch` builds; `cargo clippy
--all-targets --workspace -- -D warnings` 0; fmt clean; full lib suite green
— counts in `gates`.
