# Safety Certification & Supply Chain Integrity

Forjar provides a comprehensive safety certification pipeline for environments
where software correctness is not optional: avionics (DO-178C), automotive
(ISO 26262), and reproducible supply chains.

## Reproducible Builds (FJ-095)

Every forjar binary must be bit-for-bit reproducible from source. The
`repro_build` module checks three dimensions:

**Environment variables:**
- `SOURCE_DATE_EPOCH` — deterministic timestamps
- `CARGO_INCREMENTAL=0` — no incremental compilation artifacts
- `CARGO_PROFILE_RELEASE_STRIP` — consistent stripping

**Cargo profile settings:**
- `codegen-units = 1` — deterministic code generation
- `lto = true` — deterministic linking
- `panic = "abort"` — no unwinding tables

**Source and binary hashing:**
```rust
use forjar::core::repro_build::{hash_source_dir, hash_binary};

let source_hash = hash_source_dir(Path::new("."))?;
let binary_hash = hash_binary(Path::new("target/release/forjar"))?;
// Both are BLAKE3 64-char hex strings
```

## Ferrocene Certification (FJ-113)

[Ferrocene](https://ferrocene.dev) is the safety-certified Rust toolchain.
Forjar generates certification evidence for ISO 26262 and DO-178C builds.

**Safety standards supported:**
- `SafetyStandard::Iso26262` — Road vehicles (ASIL A–D)
- `SafetyStandard::Do178c` — Airborne systems (DAL A–E)
- `SafetyStandard::Iec61508` — Industrial functional safety
- `SafetyStandard::En50128` — Railway applications

**Source compliance checks** detect forbidden patterns:
- `unsafe` blocks and `unsafe` keyword usage
- `#![allow(unsafe_code)]` attribute
- `#![feature(...)]` nightly feature gates

```rust
use forjar::core::ferrocene::{check_source_compliance, generate_evidence, SafetyStandard};

let violations = check_source_compliance(source_code);
assert!(violations.is_empty(), "no unsafe code in certified builds");

let evidence = generate_evidence(SafetyStandard::Iso26262, binary_hash, source_hash);
assert!(evidence.compliance_checks["no_unsafe_code"]);
```

## Flight-Grade Execution (FJ-115)

The `flight_grade` module defines a `no_std`-compatible execution model:

- **Fixed-size arrays** — `MAX_RESOURCES = 256`, no heap allocation
- **Bounded loops** — all iterations bounded by `MAX_RESOURCES`
- **No panic paths** — all operations return `Result`
- **Deterministic memory** — stack-allocated `FgPlan` and `FgResource`

```rust
use forjar::core::flight_grade::{check_compliance, fg_topo_sort, FgPlan, MAX_RESOURCES};

let report = check_compliance(resource_count, max_depth);
assert!(report.compliant);    // Within limits
assert!(report.no_dynamic_alloc);
assert!(report.bounded_loops);

let mut plan = FgPlan::empty();
// ... populate resources and dependencies ...
fg_topo_sort(&mut plan)?;  // Bounded topological sort
```

## MC/DC Coverage (FJ-051)

Modified Condition/Decision Coverage is required by DO-178C DAL-A.
Forjar generates MC/DC test pairs that prove each condition independently
affects the decision outcome.

For an AND decision with n conditions:
- **n pairs** are generated (one per condition)
- **n + 1 total test cases** needed
- Each pair: `true_case` (all true) vs `false_case` (one flipped)

```rust
use forjar::core::mcdc::{build_decision, generate_mcdc_and, generate_mcdc_or};

let d = build_decision("a && b && c", &["a", "b", "c"]);
let report = generate_mcdc_and(&d);
assert_eq!(report.pairs.len(), 3);       // One per condition
assert_eq!(report.min_tests_needed, 4);  // n + 1
assert!(report.coverage_achievable);
```

## DO-330 Tool Qualification (FJ-114)

DO-330 defines tool qualification levels (TQL-1 through TQL-5).
Forjar generates a complete tool qualification data package:

- **Requirements traceability** — each requirement has test cases
- **Structural coverage evidence** — line, branch, and MC/DC
- **Qualification completeness** — all requirements verified, all coverage met

```rust
use forjar::core::do330::{generate_qualification_package, ToolQualLevel};

let pkg = generate_qualification_package("1.1.1", ToolQualLevel::Tql5);
assert!(pkg.qualification_complete);
assert_eq!(pkg.total_requirements, pkg.verified_requirements);
```

## Policy Boundary Testing (FJ-3209)

Boundary testing validates that compliance rules are non-vacuous:
every deny rule rejects at least one config, every assert rule passes
on golden configs.

```rust
use forjar::core::policy_boundary::{test_boundaries, format_boundary_results};

let result = test_boundaries(&pack);
assert!(result.all_passed());
println!("{}", format_boundary_results(&result));
```

## Secret Audit Trail (FJ-3308)

Every secret access is logged to a JSONL audit file:
- **resolve** — secret read from provider
- **inject** — secret pushed to child process
- **discard** — secret cleared from memory
- **rotate** — secret value changed

```rust
use forjar::core::secret_audit::{append_audit, audit_summary, read_audit};

let events = read_audit(state_dir)?;
let summary = audit_summary(&events);
// summary.resolves, summary.injects, summary.discards, summary.rotations
```

## Namespace Isolation (FJ-3306)

Secrets are injected into child processes via `env_clear()` + selective
inheritance. The parent process never sees the secret values.

```rust
use forjar::core::secret_namespace::{execute_isolated, verify_no_leak, NamespaceConfig};

let result = execute_isolated(&config, &secrets, "sh", &["-c", "echo $DB_PASS"])?;
assert!(result.success);
assert!(verify_no_leak("DB_PASS")); // Not in parent
```

## Falsification Example

Run the integrated falsification example:

```bash
cargo run --example policy_safety_falsification
```

This exercises all criteria above with assertions that would catch
any regression in the safety certification pipeline.
