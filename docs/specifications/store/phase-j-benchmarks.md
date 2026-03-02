# Phase J: Performance Guarantees (FJ-1355)

**Status**: ✅ Complete
**Implementation**: `benches/store_bench.rs`

---

## 1. Store Operation Benchmarks

Criterion.rs benchmarks for all store operations with 95% confidence intervals.

| Benchmark | Function | Target |
|-----------|----------|--------|
| `bench_store_path` | `store_path()` hash computation | < 1 us |
| `bench_purity_classify` | `classify()` for all 4 purity levels | < 1 us |
| `bench_closure_hash` | `closure_hash()` for 3/10/50 node graphs | < 10 us |
| `bench_repro_score` | `compute_score()` for 1/5/20 resources | < 100 us |
| `bench_far_encode` | `encode_far()` for 1KB/1MB/10MB payloads | < 100 ms |
| `bench_far_decode` | `decode_far_manifest()` roundtrip | < 10 ms |
| `bench_lockfile_staleness` | `check_staleness()` for 10/100/1000 pins | < 1 ms |
| `bench_sandbox_validate` | `validate_config()` for all 4 presets | < 1 us |
| `bench_derivation_closure` | `derivation_closure_hash()` for 5-node DAG | < 10 us |
| `bench_purify_script` | `purify_script()` for small/medium/large scripts | < 10 ms |

## 2. Auto-Update

`make bench-update` runs benchmarks and updates the README table between `<!-- BENCH-TABLE-START -->` and `<!-- BENCH-TABLE-END -->` markers.

## 3. Existing Core Benchmarks

See `benches/core_bench.rs` for BLAKE3, YAML parse, topo sort, Spec §9 targets, and Copia delta sync benchmarks.
