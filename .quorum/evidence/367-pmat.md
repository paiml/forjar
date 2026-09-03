# pmat MCP lane — #367 / #371 / #375

`analyze_vacuous_tests`: 17823 tests examined; 0 vacuous tests in any file
this branch touches (`src/mcp/*`, `src/cli/workspace.rs`,
`src/core/unattended.rs`, the five falsifier binaries).

Falsification, tests kept: with `src/mcp/paths.rs`, `src/mcp/adapter.rs`,
`src/mcp/registry.rs` and `src/mcp/mod.rs` reverted to the merge-base the
workspace cases report an empty project, the stdio cases find no
`annotations` on any tool, and the docs case fails on the restored
`docs/mcp-schema.json`; the branch's own record: re-adding
`info.output_schema` turns `no_tool_promises_a_structured_result_the_server_does_not_send`
red (`left: true, right: false`).

Gates: the branch's own record `cargo test --lib 13411 passed; clippy
--all-targets 0 errors; fmt clean; 20 integration targets green`; re-run
counts in the receipt. Merged `origin/main` cleanly so the receipt is
anchored at 9e0815c0.
