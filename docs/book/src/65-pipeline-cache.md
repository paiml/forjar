# Pipeline State & I/O Cache Tracking

Falsification coverage for FJ-2700 and FJ-2701.

## Pipeline State (FJ-2700)

### State Building

```rust
use forjar::core::task::pipeline::{build_pipeline_state, StageExecResult};

let results = vec![
    StageExecResult { name: "lint".into(), cached: false, exit_code: 0, duration_ms: 200, input_hash: None },
    StageExecResult { name: "test".into(), cached: false, exit_code: 1, duration_ms: 3000, input_hash: None },
];
let state = build_pipeline_state(&results);
// state.status == StageStatus::Failed (any failure → overall fail)
```

### Status Mapping

| Condition | Stage Status |
|-----------|-------------|
| `cached == true` | Skipped |
| `exit_code == 0` | Passed |
| `exit_code != 0` | Failed |

Overall status is `Passed` unless any stage is `Failed`.

### Summary Formatting

```
  [ 1] [   PASS] lint (200ms)
  [ 2] [   FAIL] test (3000ms)

  Pipeline: FAILED
```

### Stage Command

`stage_command()` wraps stage commands with `set -euo pipefail` for safe shell execution. Stages without commands produce `true\n` (no-op).

## I/O Cache Tracking (FJ-2701)

### Cache Skip Decision

```rust
use forjar::core::task::should_skip_cached;

// Skip when cache enabled AND both hashes present AND match
assert!(should_skip_cached(true, Some("blake3:abc"), Some("blake3:abc")));

// Don't skip when hashes differ
assert!(!should_skip_cached(true, Some("blake3:abc"), Some("blake3:def")));

// Don't skip when cache disabled
assert!(!should_skip_cached(false, Some("blake3:abc"), Some("blake3:abc")));
```

### Input/Output Hashing

- `hash_inputs(patterns, base_dir)` — BLAKE3 composite hash of glob-matched files
- `hash_outputs(artifacts)` — BLAKE3 composite hash of output files/directories
- Files are sorted and deduped for deterministic hashing

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_pipeline_io_tracking.rs` | 15 | 282 |
