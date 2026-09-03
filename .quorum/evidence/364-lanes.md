# Quorum evidence — #364 — lane summaries

## probe lane
`cargo metadata --locked` and `cargo package --locked --no-verify
--workspace` on the branch: no yanked warning. On the merge-base: the
warning on every invocation. The falsifier binary runs green on the
branch; with the merge-base's Cargo.lock restored it is red.

## crux lane
cargo-deny's own documentation: `yanked` is a `[advisories]` check over the
resolved graph, subject to the feature selection; cargo-audit's `--deny
yanked` is the lockfile-level refusal. Rust projects that gate releases
(tokio, rust-analyzer via their `cargo deny`/`cargo audit` CI) run cargo-audit
with `--deny yanked` or `--deny warnings`, and Cargo itself refuses `cargo
publish` of a crate whose lockfile pins a yanked dependency only with
`--locked` on a fresh resolve. Denying at the lockfile reader is the
default this branch adopts.

## design lane
A test that reads the lockfile; a lane flag that denies; no change to
deny.toml's feature selection (which is right for what it checks).

## judges
One decision scored: where the denial lives. See the judges file.

## agy /teamwork
Ran (the gate requires it): an independent plan-mode review in a scrubbed
HOME refuted five of its own attacks and confirmed the cargo-deny blind
spot — see the agy file.
