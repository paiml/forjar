# Conversion Strategy, Pin Tripwire & Validation

Falsification coverage for FJ-1328 (conversion strategy), FJ-1314 (pin tripwire), FJ-1306/FJ-1329 (purity and reproducibility validation).

## Conversion Strategy (FJ-1328)

Automates the 5-step conversion ladder for making recipes reproducible:

```rust
use forjar::core::store::convert::{analyze_conversion, ConversionSignals};

let signals = vec![ConversionSignals {
    name: "curl".into(),
    has_version: false,
    has_store: false,
    has_sandbox: false,
    has_curl_pipe: false,
    provider: "apt".into(),
    current_version: None,
}];

let report = analyze_conversion(&signals);
// Auto changes: AddVersionPin, EnableStore, GenerateLockPin
// Manual changes: "Add sandbox: block..."
```

### Conversion Ladder

| Step | Type | Description |
|------|------|-------------|
| 1 | Auto | Add version pins to all packages |
| 2 | Auto | Add `store: true` to cacheable resources |
| 3 | Auto | Generate `forjar.inputs.lock.yaml` |
| 4 | Manual | Add `sandbox:` blocks for full purity |
| 5 | Manual | Replace `curl\|bash` with declarative resources |

## Pin Tripwire (FJ-1314)

Lock file staleness detection during `forjar apply`:

```rust
use forjar::core::store::pin_tripwire::{check_before_apply, pin_severity, PinSeverity};

let result = check_before_apply(&lock_file, &current_hashes, &all_inputs);
if !result.all_fresh {
    match pin_severity(&result, strict_mode) {
        PinSeverity::Warning => eprintln!("{}", result.summary),
        PinSeverity::Error => return Err("stale pins in CI mode"),
        PinSeverity::Info => {} // all fresh
    }
}
```

## Purity & Repro Validation (FJ-1306/FJ-1329)

```rust
use forjar::core::store::validate::{validate_purity, validate_repro_score};

// Purity validation with minimum requirement
let v = validate_purity(&[("nginx", &signals)], Some(PurityLevel::Pinned));
assert!(v.pass);

// Reproducibility score validation
let rv = validate_repro_score(&inputs, Some(80.0));
println!("Score: {:.1}/100 (Grade {})", rv.score.composite, rv.grade);
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_convert_pin_validate.rs` | 26 | ~330 |
