# Quorum evidence — 1.27.0 fleet P0 — mechanical lane

`pmat analyze vacuous-tests` over the touched test paths: 0 hits. The three falsification suites added here each execute the real binary or the emitted script and assert on measured output, and each is pinned by a mutation recorded in its ticket receipt.

`pmat work validate`: passes. PMAT-212, PMAT-213, PMAT-214 and PMAT-215 all resolve in `docs/roadmaps/roadmap.yaml`.

`pmat tdg`: no regression. `src/cli/drift_state.rs` was refused by the pre-commit ratchet at A (94.9) after the first shape of the scope filter and was restructured into `in_this_run` and `refuse_out_of_scope` until it measured A+ (95.5). `src/core/executor/mod.rs` was refused at 515 lines against the 500-line file-health limit and the reasoning moved into the `persist_unlatched` doc comment where it belongs; it is 500 now. Neither ceiling was raised and `--no-verify` was not used.

Dogfood gates on this branch: A PASS, B PASS, C PASS, D PASS, E PASS, G PASS, H PASS. Gate F is recorded in the release receipt.

Gate A caught a real gap while running: PR #484 had merged with no `docs/audits/impl-PMAT-208-receipt.md`, so nothing recorded that the 1.26.0 dist-artifacts repair went through the harness. It is written and committed here.
