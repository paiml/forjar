# Quorum evidence — #364 (fix/yanked-spin-pin) — adjudicated claims

## CONFIRMED — 4 claims survived refutation

1. [probe] (explains-symptom) `Cargo.lock` pinned `spin 0.9.8`, yanked on crates.io, reached only through `wasmi 0.40.0` (the WASM plugin runtime), and every release preflight since 1.21.0 printed `warning: package spin v0.9.8 in Cargo.lock is yanked` and went on.
   - evidence: `Cargo.lock` at the merge-base carries `name = "spin" / version = "0.9.8"`; `cargo package --locked` warns and exits 0. Bumped to 0.9.9 (the un-yanked patch release). Pinned by `cargo_lock_pins_no_known_yanked_release` in `tests/falsification_lockfile_carries_no_known_yanked_pin.rs`, which parses the lockfile directly.

2. [probe] (explains-symptom) Both halves of the standing gate were blind to it in different ways: `cargo deny check` resolves a FEATURE graph and `deny.toml`'s `all-features = false` keeps the optional `wasmi` out of it (measured: `yanked = "deny"` exits 0 with zero mentions); `cargo audit` in the same job reads the lockfile, HAS reported it on every daily run, and a yank is a warning class it forgives (`warning: 2 allowed warnings found`, exit 0).
   - evidence: `.github/workflows/audit.yml` at the merge-base ran cargo-audit without `--deny yanked`; the step now passes it, so a yanked pin fails the lane. Pinned by `audit_workflow_denies_yanked_crates`, which reads the workflow text.

3. [design] The lockfile check is a test in the tree, not only a CI lane, so the pin cannot return silently between releases.
   - evidence: the falsifier carries the known-yanked set it checks and reads `Cargo.lock` at test time; a `cargo update` that re-pins a yanked release fails `cargo test` locally before it reaches the audit lane.

4. [design] The falsifiers cannot pass vacuously.
   - evidence: both parse real files under the repo root (`repo_file`), fail if the file is missing, and assert on parsed content — the lockfile case on the `[[package]]` table entries, the workflow case on the literal flag in the cargo-audit step.

## REFUTED — 1 claim killed

1. [design] refuted 1/1 — "Set `yanked = "deny"` in deny.toml and be done."
   - corrected: measured to exit 0 without seeing the crate at all, because `all-features = false` excludes the optional `wasmi` subtree from the feature graph; the lane that reads the lockfile (cargo-audit) is the one that has to deny.
