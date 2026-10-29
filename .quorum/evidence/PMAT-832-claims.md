C1. cmd_check resolves data sources before rendering (src/cli/check.rs:368).
C2. load_drift_config resolves them and downgrades failure to a warning (src/cli/drift.rs:192).
C3. Three integration tests drive the real binary (tests/falsification_check_drift_resolve_data_sources.rs:46).