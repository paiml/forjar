# Quorum evidence — #423 — lane summaries

## probe lane
Diffed both vendored `src/` trees and `build.rs` against the registry copies
(0 lines differ). Ran the CI jobs' own commands: `cargo check --locked
--no-default-features --lib` (the first cut failed it), `cargo package
--locked --no-verify --workspace` (root-only packaging failed on the
unpublished member), `cargo fetch && cargo build --offline`. Read the
lockfile for registry sources of the contract crates: none.

## crux lane
The organisation's own monorepo moves a crate in-tree by renaming the package
and keeping the `[lib]` name (`crates/aprender-compute`, lib `trueno`); this
follows that shape. tokio, rust-analyzer and wgpu keep first-party crates as
workspace members with `version` + `path` and publish them in dependency
order — the same `cargo publish --workspace` shape used here. `cargo vendor`
and `[patch.crates-io]` are for keeping an EXTERNAL crate pinned, not for
owning it; they were weighed and not chosen because the instruction was to
move the crate into this repository.

## design lane
Two crates, `[lib]` names unchanged so no `use` line moves; plain pinned path
dependencies (the feature-gated first cut broke `--no-default-features`);
workspace packaging in both workflows; every deviation from the registry copy
declared in a VENDORED.md per crate.

## judges
Two decisions scored: rename-and-own vs `[patch]`; feature-gated vs plain
dependencies. See the judges file.

## agy /teamwork
Independent stack review in plan mode: six charges, four taken. It also
executed `cargo publish --workspace` — an incident recorded in the agy file,
the receipt and .pmat/jidoka.jsonl.
