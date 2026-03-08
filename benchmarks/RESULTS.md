# Benchmark Results

Last updated: 2026-03-08T22:44:48Z

<!-- BENCH-TABLE-START -->
| Operation | Target | Last Run | p50 | p95 | Status |
|-----------|--------|----------|-----|-----|--------|
| validate (3m, 20r) | < 10ms | 936.5µs | 955.0µs | 1.1ms | pass |
| plan (3m, 20r) | < 2s | 1.0ms | 976.0µs | 1.1ms | pass |
| drift (100 resources) | < 1s | 2.1ms | 2.0ms | 2.8ms | pass |
| blake3 hash (4KB) | < 2µs | 2.8µs | 2.0µs | 2.0µs | fail |
| topo sort (20 nodes) | < 100µs | 40.3µs | 39.0µs | 48.0µs | pass |
| blake3 hash (1MB) | < 500µs | 198.8µs | 203.0µs | 212.0µs | pass |
<!-- BENCH-TABLE-END -->
