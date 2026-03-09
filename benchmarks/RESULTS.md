# Benchmark Results

Last updated: 2026-03-09T00:00:00Z

<!-- BENCH-TABLE-START -->
| Operation | Target | Last Run | p50 | p95 | Status |
|-----------|--------|----------|-----|-----|--------|
| validate (3m, 20r) | < 10ms | 148.7µs | 139.0µs | 214.0µs | pass |
| plan (3m, 20r) | < 2s | 162.5µs | 150.0µs | 230.0µs | pass |
| drift (100 resources) | < 1s | 338.3µs | 327.0µs | 402.0µs | pass |
| blake3 hash (4KB) | < 2µs | 1.2µs | 1.0µs | 1.0µs | pass |
| topo sort (20 nodes) | < 100µs | 3.5µs | 3.0µs | 4.0µs | pass |
| blake3 hash (1MB) | < 500µs | 123.9µs | 123.0µs | 129.0µs | pass |
<!-- BENCH-TABLE-END -->
