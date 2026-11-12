# Claims — ci.yml cargo test step consolidation (PMAT-913 / forjar#978)

1. K1: `.github/workflows/ci.yml:147-167` preserves the exact 20 non-sudo `--test` targets from `c5b7e4f3` with `--locked` and default features.
2. K2: `.github/workflows/ci.yml:187-190` keeps `FORJAR_REQUIRE_SUDO_TESTS=1` isolated to `falsification_sudo_transport_survives_closefrom` while keeping `falsification_sudo_transport_closefrom_emulated` unprivileged.
3. K3: `.github/workflows/ci.yml:147-167` preserves fail-fast exit semantics, clean backslash continuations, and leaves `doctests`, `dogfood-guards`, `lint.yml`, and `gate.needs` untouched.
4. K4: `docs/roadmaps/roadmap.yaml:6649` (`PMAT-913`) and `docs/audits/impl-PMAT-913-receipt.md:1-29` satisfy Gate A traceability with zero `.github/workflows/ci.yml` drift after `acc83726`.
5. K5: `.github/workflows/ci.yml:139` and `docs/audits/impl-PMAT-913-receipt.md:3,14` accurately state the number of merged `cargo test` steps.
6. K6: `docs/audits/impl-PMAT-913-receipt.md:16` accurately states that target execution order across the entire `examples-validate` job is unchanged.
