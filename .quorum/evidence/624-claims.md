# Quorum evidence — #624 (fix/624-nightly-musl) — adjudicated claims

## CONFIRMED — 3 claims survived refutation

1. [probe] (explains-symptom) The nightly built only the `-gnu` Linux legs, linked on the fleet host against glibc 2.39, so on lambda-labs (glibc 2.35) every nightly forjar exited with `version 'GLIBC_2.39' not found` and fleet-bins recorded a WONT-RUN failure on every poll.
   - evidence: the fleet-bins andon on lambda read rows=9 failures=2 stale=1 with the row `WONT-RUN paiml/forjar forjar nightly: GLIBC_2.39 not found`; release.yml's build-binaries matrix already carries both musl targets, which is what the release channel installs on lambda. The new test reads that matrix at tests/falsification_nightly_ships_every_release_linux_target.rs:42 and requires a musl target among it at tests/falsification_nightly_ships_every_release_linux_target.rs:47 before comparing anything.

2. [probe] (fix-is-sufficient) Adding the two musl legs, plus musl-tools for the x86_64 leg exactly as release.yml installs it, makes the nightly publish a static binary for every Linux target a release publishes; aarch64-musl builds through `cross` the same way the release does (PMAT-547 comment kept in place above its step).
   - evidence: the test compares the nightly `build` matrix to the release `build-binaries` matrix at tests/falsification_nightly_ships_every_release_linux_target.rs:51 and tests/falsification_nightly_ships_every_release_linux_target.rs:52 and is green on this branch; deleting either musl leg from nightly.yml turns it RED with `nightly.yml does not build ["x86_64-unknown-linux-musl"]` (resp. aarch64), rc=101, measured locally for both mutants.

3. [design] (no-third-list) The guard derives the expected targets from release.yml rather than naming them, so a Linux target added to releases and not to the nightly is RED without anyone remembering to edit a third list; the parse fails loudly by name if either job or its matrix moves.
   - evidence: tests/falsification_nightly_ships_every_release_linux_target.rs:24 through tests/falsification_nightly_ships_every_release_linux_target.rs:37 parse jobs.<job>.strategy.matrix.include[].target with serde_yaml_ng and panic naming the file and job when the path is absent, so a renamed job cannot turn the comparison into an empty-set pass; the vacuity guard at tests/falsification_nightly_ships_every_release_linux_target.rs:47 closes the gnu-only case.

## REFUTED — 1 claim killed

1. [design] (alternative) Keep the nightly gnu-only and build it inside an older-glibc container (glibc 2.35) so the dynamic binary runs on lambda, instead of adding musl legs.
   - corrected: refuted because the release channel already ships and fleet-bins already installs the static musl asset on lambda; a second portability mechanism for the nightly alone would make the two channels differ in linkage and would still break on the next host with an older glibc. The parity test at tests/falsification_nightly_ships_every_release_linux_target.rs:52 encodes the chosen rule: the nightly covers what the release covers.
