# pmat MCP lane — #400 / #401 / #386

`analyze_vacuous_tests`: 17815 tests examined; 0 vacuous tests in any file
this branch touches (the three falsifier binaries, `scripts/`, the
workflows, `.gitignore`).

Falsification, tests kept: with `scripts/quorum-gate.sh`,
`scripts/hooks/pre-push` and `.github/workflows/coverage.yml` at the
merge-base and `.pmat/baseline.json` re-staged from it, the gate cases
refuse the cross-branch push and pass the untracked receipt, the
gitignore case finds a tracked-and-ignored path, and the coverage case
misses `rust-cache`. Counts in the receipt.

Gates: clippy `-D warnings`, fmt, `cargo test --lib` counts in the
receipt. Merged `origin/main` (conflicts: `scripts/quorum-gate.sh` — the
cross-branch skip kept, main's GIT_* scrub folded into its `else` branch,
`import os` added; `.pmat/baseline.json` — deletion kept) so the receipt
is anchored at 9e0815c0.
