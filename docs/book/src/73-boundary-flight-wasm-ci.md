# Policy Boundary, Flight-Grade, Ferrocene, WASM & CI

Falsification coverage for FJ-3209, FJ-115, FJ-113, FJ-2402, and FJ-2403.

## Policy Boundary Testing (FJ-3209)

Auto-generates boundary configs that exercise policy rules at decision boundaries:

```rust
use forjar::core::policy_boundary::*;

let configs = generate_boundary_configs(&pack);
// 2 configs per rule: golden (should pass) + boundary (should fail)

let result = test_boundaries(&pack);
assert!(result.all_passed());
println!("{}", format_boundary_results(&result));
```

Supports Assert, Deny, Require, and RequireTag checks. Script checks cannot have auto-generated boundaries.

## Flight-Grade Execution (FJ-115)

No-std compatible core with fixed-size buffers and bounded iteration:

```rust
use forjar::core::flight_grade::*;

let report = check_compliance(100, 10);
assert!(report.compliant); // within MAX_RESOURCES=256, MAX_DEPTH=32

let mut plan = FgPlan::empty();
plan.resources[0].id = 0;
plan.count = 1;
fg_topo_sort(&mut plan).unwrap();
```

## Ferrocene Certification (FJ-113)

Source compliance checking for safety-certified builds:

```rust
use forjar::core::ferrocene::*;

let violations = check_source_compliance(source);
// Detects: unsafe blocks, #![allow(unsafe_code)], #![feature(...)]

let evidence = generate_evidence(SafetyStandard::Iso26262, "binhash", "srchash");
```

Supports ISO 26262, DO-178C, IEC 61508, EN 50128. ASIL levels QM-D, DAL levels E-A.

## WASM Types (FJ-2402)

Size budgets and bundle drift detection:

```rust
use forjar::core::types::*;

let budget = WasmSizeBudget::default(); // core=100KB, app=500KB
assert!(budget.check_core(90 * 1024));

let drift = BundleSizeDrift::check(&budget, 90 * 1024, Some(85 * 1024));
assert!(drift.is_ok()); // within budget, <20% growth
```

CDN targets (S3, Cloudflare, Local) with cache policies and optimization levels.

## CI Pipeline Types (FJ-2403)

```rust
use forjar::core::types::*;

let config = ReproBuildConfig::default();
assert!(config.is_reproducible()); // locked, no_incremental, lto, codegen_units=1

let msrv = MsrvCheck::new("1.88.0");
assert!(msrv.satisfies("1.89.0"));

let matrix = FeatureMatrix::new(vec!["encryption", "container-test"]);
assert_eq!(matrix.combinations().len(), 4); // 2^2
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_boundary_flight.rs` | 37 | ~380 |
| `falsification_wasm_ci_types.rs` | 45 | ~370 |
