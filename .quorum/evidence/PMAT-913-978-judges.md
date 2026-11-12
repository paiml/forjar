# Judges — ci.yml cargo test step consolidation (PMAT-913 / forjar#978)

Round count and heads: see `PMAT-913-978-lanes.md`. Every claim `K1..K6` met 3 independent refuters across Correctness, Failure Modes, and Contract/Traceability.

## CONFIRMED

1. [diff] C1 — `.github/workflows/ci.yml:147-167` preserves the exact 20 non-sudo integration `--test` targets from `c5b7e4f3` under a single `cargo test --locked` invocation with identical default features.
   - evidence: `yaml.safe_load` AST parsing on `t2fw13` verified `nb == nh` (`len == 20`) with comment lines stripped before tokenizing `run:` blocks, and dropping `nh[-1]` raised `AssertionError` as required.
2. [diff] C2 — `.github/workflows/ci.yml:187-190` isolates `FORJAR_REQUIRE_SUDO_TESTS=1` exclusively to `falsification_sudo_transport_survives_closefrom` while keeping `falsification_sudo_transport_closefrom_emulated` unprivileged in the batch step.
   - evidence: `.github/workflows/ci.yml:188-189` scopes `FORJAR_REQUIRE_SUDO_TESTS: "1"` to the live-sudo step's `env:` mapping alone, leaving job-level `env:` free of privilege escalation flags while preserving `sudo -n true` preflight.
3. [diff] C3 — `.github/workflows/ci.yml:147-167` maintains fail-fast failure propagation, contiguous backslash line-continuations without embedded comments, and leaves `doctests`, `dogfood-guards`, `lint.yml`, and `gate.needs` (`ci.yml:430`) untouched.
   - evidence: all 19 backslash-terminated lines in `.github/workflows/ci.yml:148-166` contain zero trailing whitespace or intervening `#` lines, Cargo without `--no-fail-fast` exits non-zero on the first failing binary, and workflow `cargo test` count drops `21 -> 4`.
4. [doc] C4 — `docs/roadmaps/roadmap.yaml:6649` and `docs/audits/impl-PMAT-913-receipt.md:1-29` satisfy Gate A traceability with zero `.github/workflows/ci.yml` diff between `acc83726` and `20c48834`.
   - evidence: `docs/roadmaps/roadmap.yaml:6649` maps `id: PMAT-913` uniquely to `github_issue: 913`, `docs/audits/impl-PMAT-913-receipt.md:5,29` carries one `verdict:` line and `IMPL-PMAT-913-RECEIPT-END`, and `git diff acc83726..20c48834 -- .github/workflows/ci.yml` is empty.

## REFUTED

1. [doc] R1 — `.github/workflows/ci.yml:139` said `"instead of 20 invocations"` and `docs/audits/impl-PMAT-913-receipt.md:3,14` said `"instead of 19 steps"`, conflating 20 targets and 19 total steps with the 18 non-sudo steps actually merged.
   - corrected: `examples-validate` had 18 non-sudo `cargo test` steps (20 `--test` flags) plus 1 live-sudo step = 19 total before, becoming 1 non-sudo step plus 1 live-sudo step = 2 total after (`-17` invocations); narrowed by all 3 lanes and queued for prose update by `forjar-cop`.
2. [doc] R2 — `docs/audits/impl-PMAT-913-receipt.md:16` claimed `"the set and order of --test targets executed by examples-validate is unchanged"` across the whole job, whereas pulling step 21 into the non-sudo batch moved `falsification_version_matches_manifest` ahead of `falsification_sudo_transport_survives_closefrom`.
   - corrected: the 20 non-sudo targets preserve exact relative order (`nb == nh`), while at the 21-target job level `falsification_version_matches_manifest` now executes immediately before `falsification_sudo_transport_survives_closefrom`; narrowed by all 3 lanes with zero runtime coupling.
