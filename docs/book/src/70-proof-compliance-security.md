# Proof Obligations, Compliance & Security

Falsification coverage for FJ-1385, FJ-1382, FJ-1387, FJ-1390, and FJ-2702.

## Proof Obligation Taxonomy (FJ-1385)

Every resource operation is classified into one of four formal categories:

| Category | Property | Safe |
|----------|----------|------|
| Idempotent | `f(f(x)) = f(x)` | Yes |
| Monotonic | Only adds state | Yes |
| Convergent | Reaches fixed point | Yes |
| Destructive | Removes irreconstructable state | No |

```rust
use forjar::core::planner::proof_obligation::{classify, label, is_safe};
use forjar::core::types::{ResourceType, PlanAction};

let po = classify(&ResourceType::File, &PlanAction::Destroy);
assert_eq!(label(&po), "destructive");
assert!(!is_safe(&po));
```

Key classifications:
- File/Package create → Idempotent
- Service create → Convergent
- Model create → Monotonic
- File/User destroy → Destructive
- Service destroy → Convergent

## Reversibility (FJ-1382)

Classifies destroy operations as reversible or irreversible:

```rust
use forjar::core::planner::reversibility::{classify, Reversibility};
use forjar::core::types::{Resource, ResourceType, PlanAction};

let file = Resource { resource_type: ResourceType::File, ..Default::default() };
// No content/source → irreversible (data lost)
assert_eq!(classify(&file, &PlanAction::Destroy), Reversibility::Irreversible);
```

`count_irreversible()` and `warn_irreversible()` scan an execution plan for destructive operations.

## Compliance Benchmarks (FJ-1387)

Four benchmark suites evaluate IaC configs against security standards:

| Benchmark | Rules |
|-----------|-------|
| CIS | World-writable mode, root /tmp, service restart policy, version pin |
| NIST 800-53 | AC-3 (owner/mode), AC-6 (root service), CM-6 (port bindings), SC-28 (sensitive paths), SI-7 (integrity) |
| SOC2 | CC6.1 (file ownership), CC7.2 (service monitoring) |
| HIPAA | 164.312(a) (other access), 164.312(e) (unencrypted ports) |

```rust
use forjar::core::compliance::{evaluate_benchmark, count_by_severity};

let findings = evaluate_benchmark("nist-800-53", &config);
let (critical, high, medium, low) = count_by_severity(&findings);
```

## Security Scanner (FJ-1390)

Ten IaC security smell detection rules:

| Rule | Category | Severity |
|------|----------|----------|
| SS-1 | Hardcoded secrets | Critical |
| SS-2 | HTTP without TLS | High |
| SS-3 | World-accessible | High |
| SS-4 | Missing integrity check | Medium |
| SS-5 | Privileged container | Critical/Medium |
| SS-6 | No resource limits | Low |
| SS-7 | Weak crypto | High |
| SS-8 | Insecure protocol | High |
| SS-9 | Unrestricted network | Medium |
| SS-10 | Sensitive data exposure | Critical |

## Quality Gate Evaluation (FJ-2702)

Pipeline gates evaluate task execution output:

```rust
use forjar::core::task::{evaluate_gate, GateResult};
use forjar::core::types::QualityGate;

let mut gate = QualityGate::default();
gate.parse = Some("json".into());
gate.field = Some("coverage".into());
gate.min = Some(80.0);

let result = evaluate_gate(&gate, 0, r#"{"coverage":95.0}"#);
assert_eq!(result, GateResult::Pass);
```

Gate modes: exit code (default), JSON field parsing, regex stdout matching, numeric thresholds.

Failure actions: `block` (default), `warn`, `skip_dependents`.

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_proof_security.rs` | 31 | 389 |
| `falsification_security_scan.rs` | 18 | 208 |
| `falsification_compliance_gate.rs` | 40 | 484 |
