# Refinement Types, Contract Tiers & CI Pipeline

Falsification coverage for FJ-043, FJ-2203, FJ-2400, and FJ-2403.

## Refinement Types (FJ-043)

Compile-time and runtime validated config values:

```rust
use forjar::core::types::refinement::*;

let port = Port::new(8080).unwrap();       // 1-65535
let mode = FileMode::new(0o644).unwrap();  // 0-0o777
let ver = SemVer::parse("1.2.3").unwrap(); // X.Y.Z
let host = Hostname::new("web-01.example.com").unwrap(); // RFC 1123
let path = AbsPath::new("/etc/app.conf").unwrap();       // absolute
let name = ResourceName::new("pkg-nginx").unwrap();       // alphanum + -_
```

All types reject invalid values at construction with descriptive errors.

## Verification Tiers (FJ-2203)

Six-level contract verification maturity model (L0-L5):

```rust
use forjar::core::types::*;

assert!(VerificationTier::Runtime < VerificationTier::Bounded);

let report = ContractCoverageReport { total_functions: 24, entries, handler_invariants };
let hist = report.histogram(); // [usize; 6] counts per tier
assert_eq!(report.at_or_above(VerificationTier::Bounded), 5);
println!("{}", report.format_summary());
```

## CI Pipeline Types (FJ-2403)

Reproducible builds, MSRV enforcement, and feature matrix:

```rust
use forjar::core::types::*;

// Reproducible build config
let repro = ReproBuildConfig::default();
assert!(repro.is_reproducible()); // locked + no_incremental + lto + codegen_units=1

// MSRV
let msrv = MsrvCheck::new("1.88.0");
assert!(msrv.satisfies("1.89.0"));

// Feature matrix — generates 2^n test combinations
let matrix = FeatureMatrix::new(vec!["encryption", "container-test"]);
assert_eq!(matrix.combinations().len(), 4);
for cmd in matrix.cargo_commands() { println!("{cmd}"); }
```

## Purification & Model Integrity (FJ-2400/2401)

```rust
use forjar::core::types::*;

let bench = PurificationBenchmark { resource_type: "file".into(), validate_us: 50.0, purify_us: 150.0, sample_count: 100 };
assert!((bench.overhead_ratio() - 3.0).abs() < 0.01);

let check = ModelIntegrityCheck::check("llama3", "abc", "abc", 1_000_000);
assert!(check.valid);
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_refinement_contract_ci.rs` | 41 | ~425 |
