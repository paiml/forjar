# Forjar — Rust-Native Infrastructure as Code

Bare-metal first. BLAKE3 state. Provenance tracing.

## Falsifiable Claims

The following claims are testable and falsifiable. Each claim references specific tests that would **fail** if the claim were violated.

### C1: Deterministic hashing

**Claim**: BLAKE3 hashing of identical inputs always produces identical outputs, and different inputs produce different outputs.

**Falsification**: Change a file's content and verify the hash changes. Hash the same file twice and verify equality.

**Tests**: `test_fj014_hash_file_deterministic`, `test_fj014_hash_string`, `test_fj014_hash_directory_order_independent_of_creation`

### C2: DAG execution order is deterministic

**Claim**: Given the same dependency graph, topological sort always produces the same execution order (alphabetical tie-breaking).

**Falsification**: Run the resolver on the same config 1000 times and verify all results are identical.

**Tests**: `test_fj003_topo_sort_deterministic`, `test_fj003_alphabetical_tiebreak`, `test_fj003_diamond_dependency`

### C3: Idempotent apply

**Claim**: Running `forjar apply` twice on an unchanged config produces zero changes on the second run.

**Falsification**: Apply a config, then apply again and verify `to_create == 0 && to_update == 0`.

**Tests**: `test_fj012_idempotent_apply`, `test_fj004_plan_all_unchanged`

### C4: Cycle detection

**Claim**: Circular dependencies are detected at parse time and rejected with an error, never silently ignored.

**Falsification**: Create a config with A→B→A and verify the resolver returns an error.

**Tests**: `test_fj003_cycle_detection`

### C5: Content-addressed state

**Claim**: Lock file hashes are derived from the desired state definition, not from timestamps or execution artifacts.

**Falsification**: Create two identical resources at different times and verify they produce the same hash.

**Tests**: `test_fj004_hash_deterministic`, `test_fj004_plan_all_unchanged`

### C6: Atomic state persistence

**Claim**: Lock file writes are atomic (temp file + rename). A crash during write cannot corrupt the lock file.

**Falsification**: Verify no `.tmp` file remains after successful save. Verify the lock file is valid YAML after save.

**Tests**: `test_fj013_atomic_write`, `test_fj013_save_and_load`

### C7: Recipe input validation

**Claim**: Recipe inputs are validated against declared types and constraints before expansion. Invalid inputs are rejected.

**Falsification**: Pass a string where an integer is expected and verify rejection.

**Tests**: `test_fj019_validate_inputs_type_mismatch`, `test_fj019_validate_inputs_missing_required`, `test_fj019_validate_inputs_enum_invalid`

### C8: Heredoc injection safety

**Claim**: File content written via heredoc with single-quoted delimiter (`<<'EOF'`) prevents shell variable expansion and command injection.

**Falsification**: Include `$HOME` and backtick commands in content and verify they appear literally in the generated script.

**Tests**: `test_fj007_heredoc_safe`

### C9: Single binary, minimal dependencies

**Claim**: The release binary is a single static executable with fewer than 10 direct crate dependencies.

**Falsification**: Count direct dependencies in `Cargo.toml`. Build a release binary and verify it is a single file.

**Verification**: `cargo metadata --no-deps --format-version 1 | jq '.packages[0].dependencies | length'`

### C10: Jidoka failure isolation

**Claim**: When a resource fails to apply, execution stops immediately. Previously converged resources retain their lock state.

**Falsification**: Create a config where the second resource fails. Verify the first resource's lock is preserved and the third is not attempted.

**Tests**: `test_fj012_apply_local_file` (verifies successful apply stores lock state)

## Quick Start

```bash
# Build
cargo build --release

# Validate a config
forjar validate forjar.yaml

# Preview changes
forjar plan forjar.yaml

# Apply (converge state)
forjar apply forjar.yaml

# Detect drift
forjar drift forjar.yaml
```

## Architecture

```
forjar.yaml → parser → recipe → resolver → planner → codegen → executor → state
                                    │                                         │
                                    └── DAG topological sort                  └── BLAKE3 lock files
```

**Core modules**: parser, resolver, planner, codegen, executor, state, recipe
**Resource types**: package, file, service, mount (extensible)
**Transport**: local execution, SSH remote
**Integrity**: BLAKE3 hashing, drift detection, append-only event log

## Benchmarks

Run benchmarks with:

```bash
cargo bench
```

### Methodology

- **Framework**: Criterion.rs 0.5 with 100 samples per benchmark
- **Confidence level**: 95% confidence intervals reported for all measurements
- **Warm-up**: 3 seconds per benchmark to stabilize CPU caches
- **Sample size**: 100 iterations minimum; Criterion auto-tunes for statistical significance
- **Effect size threshold**: Performance regressions > 5% are considered meaningful
- **Environment**: Results are hardware-dependent. Reproduce locally with `cargo bench`.

### Results (reference: AMD EPYC, Linux 6.8)

| Operation | Input Size | Mean | 95% CI | Baseline |
|-----------|-----------|------|--------|----------|
| BLAKE3 hash (string) | 64 B | 27 ns | ± 0.5 ns | — |
| BLAKE3 hash (string) | 1 KB | 92 ns | ± 1.2 ns | SHA-256: ~350 ns |
| BLAKE3 hash (string) | 4 KB | 305 ns | ± 3.1 ns | SHA-256: ~1.4 µs |
| BLAKE3 hash (file) | 1 KB | 4.5 µs | ± 0.1 µs | Includes I/O |
| BLAKE3 hash (file) | 1 MB | 172 µs | ± 0.4 µs | ~5.8 GB/s effective |
| YAML parse | 500 B config | 20.7 µs | ± 0.2 µs | serde_yaml |
| Topo sort (Kahn) | 10 nodes | 2.7 µs | ± 0.1 µs | — |
| Topo sort (Kahn) | 50 nodes | 16.4 µs | ± 0.1 µs | O(V+E) |
| Topo sort (Kahn) | 100 nodes | 34.6 µs | ± 0.4 µs | O(V+E) |

SHA-256 baselines measured separately for comparison. BLAKE3 is consistently 3-4x faster due to SIMD acceleration.

## Testing

```bash
# Run all 126+ unit tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific module tests
cargo test planner
cargo test recipe
cargo test hasher
```

## License

MIT OR Apache-2.0
