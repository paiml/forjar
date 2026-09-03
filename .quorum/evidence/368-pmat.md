# pmat MCP lane — #363 / #368 / #378

`analyze_vacuous_tests`: counts recorded in the receipt after the last
commit; the three falsifiers assert refusal text by name AND the filesystem
consequence, so neither a clap error nor a silent converge passes.

Falsification, tests kept: with the fix hunks reverted, the phony-resource
case refuses with the provenance error, the forced case aborts (debug) or
over-reports (release), and the every-gate case converges past a tampered
sidecar. All three RED against the merge-base, GREEN on the branch.

Gates: `cargo test --lib` 13370/0; clippy `-D warnings` 0; fmt clean.
Merged `origin/main` at 47035f5a to clear the `.pmat/baseline.json`
conflict (#401); the receipt is anchored to the post-merge merge-base.
