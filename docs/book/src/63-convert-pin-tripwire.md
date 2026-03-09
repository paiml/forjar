# Recipe Conversion, Pin Tripwire & Resolution

Falsification coverage for FJ-1328, FJ-1314, and FJ-1364.

## Recipe Conversion Ladder (FJ-1328)

Analyzes resources and recommends a 5-step purity upgrade path.

### Purity Levels

| Level | Criteria |
|-------|----------|
| Impure | `curl\|bash` install pattern |
| Constrained | No version pin |
| Pinned | Version + store enabled |
| Pure | Version + store + sandbox |

### Auto Changes

| Change | Trigger |
|--------|---------|
| AddVersionPin | `has_version == false` |
| EnableStore | No store AND cacheable provider |
| GenerateLockPin | Version exists but no lock entry |

Cacheable providers: `apt`, `cargo`, `uv`, `nix`, `docker`, `pip`.

```rust
use forjar::core::store::convert::{analyze_conversion, ConversionSignals};

let signals = vec![ConversionSignals {
    name: "curl".into(),
    has_version: false, has_store: false,
    has_sandbox: false, has_curl_pipe: false,
    provider: "apt".into(), current_version: None,
}];
let report = analyze_conversion(&signals);
// report.auto_change_count > 0, manual_changes suggest sandbox
```

## Pin Tripwire (FJ-1314)

Detects stale or missing pins before `forjar apply`.

### Severity Levels

| Condition | Non-Strict | Strict |
|-----------|-----------|--------|
| All fresh | Info | Info |
| Stale pins | Warning | Error |
| Missing inputs | Warning | Error |

```rust
use forjar::core::store::pin_tripwire::{check_before_apply, pin_severity};

let result = check_before_apply(&lockfile, &current_hashes, &inputs);
let severity = pin_severity(&result, /*strict=*/ true);
// PinSeverity::Error blocks apply in strict mode
```

### Report Format

```
WARNING: 1 stale pin(s) and 1 unpinned input(s) detected.
  STALE: curl — locked=blake3:aaa current=blake3:new
  MISSING: jq — not in lock file
```

## Pin Resolution (FJ-1364)

### Provider Commands

| Provider | Command |
|----------|---------|
| apt | `apt-cache policy {name}` |
| cargo | `cargo search {name} --limit 1` |
| nix | `nix eval nixpkgs#{name}.version --raw` |
| pip/uv | `pip index versions {name}` |
| docker | `docker image inspect {name} --format ...` |

### Version Parsing

Each provider has a dedicated parser for its CLI output format (Candidate line for apt, quoted version for cargo, digest for docker, Available versions for pip).

### Pin Hashing

BLAKE3 composite hash over `provider|name|version`. Deterministic and sensitive to all three components.

```rust
use forjar::core::store::pin_resolve::pin_hash;

let h = pin_hash("apt", "curl", "7.88.1");
assert!(h.starts_with("blake3:"));
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_convert_pin_tripwire.rs` | 32 | 420 |
