# Claims — data source fault isolation (PMAT-828)

C1. `resolve_data_sources_isolated` tries every `data:` source, injects the healthy ones and returns the failures instead of aborting (`resolve_data_sources_isolated`).
C2. `dependents_of_failures` drops exactly the resources whose templates reference a failed key, plus anything that `depends_on` them, transitively (`dependents_of_failures`).
C3. `plan` and `apply` still exit non-zero after processing the unrelated resources, on every return path (src/cli/data_isolation.rs:32).
C4. The strict `resolve_data_sources` is unchanged for every other caller (`test_strict_resolver_still_errors`).
C5. The integration test drives the real binary and has a negative control (tests/falsification_data_source_fault_isolation.rs:95).
C6. An outage after a good apply leaves the converged file and its lock entry alone (tests/falsification_data_source_fault_isolation.rs:114).
