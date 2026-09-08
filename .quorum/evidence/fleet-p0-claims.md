# Quorum evidence — 1.27.0 fleet P0 — the claims put to the lanes

Four issues, three tickets, one branch. The operator declared them P0 with the fleet blocked, so the branch carries all three fixes and each keeps its own commit, receipt and falsification test.

1. `drift -f <config>` checks only the machines that config declares, and a stack it does not declare is never opened, so it can never be judged against the local filesystem.
2. `--all-stacks` asks the wide question explicitly, and every DRIFTED row names its machine, so an aggregated run can be attributed.
3. A `-m` naming a machine the config does not declare is refused by name rather than scanning zero machines and reporting clean — through EVERY door that walks the state dir.
4. `--refresh` re-checks an entry the lock records as failed and persists the promotion, so one run is a way back and a plain apply afterwards is green.
5. The cargo check and the cargo drift observable repair the same PATH the install repairs, honouring `CARGO_HOME`, so an installed crate is never reported missing and the toolchain bootstrap does not re-run every apply.
6. Nothing in the diff is outside what the three tickets ask for.

Claims 1, 2, 4 and 5 were confirmed. Claim 3 was refuted: the guard was in one door and not the other. Claim 5 was confirmed in substance and refuted in detail: a hard-coded `$HOME` shadowed the `CARGO_HOME` repair when the bootstrap ran. Claim 6 was refuted on the bundling, and answered rather than accepted.
