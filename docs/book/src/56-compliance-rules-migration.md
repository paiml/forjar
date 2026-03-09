# Compliance, Rules Engine, Migration & MC/DC

Falsification coverage for FJ-1387, FJ-3205, FJ-3208, FJ-3108, FJ-3106, FJ-3302, FJ-051, FJ-036, and FJ-044.

## Compliance Benchmarks (FJ-1387)

Four frameworks with structured rule evaluation:

| Framework | Rules | Focus |
|-----------|-------|-------|
| CIS | CIS-6.1.1, CIS-1.1.5, CIS-5.2.1, CIS-6.2.1 | File modes, services, packages |
| NIST 800-53 | AC-3, AC-6, CM-6, SC-28, SI-7 | Access control, config, integrity |
| SOC2 | CC6.1, CC7.2 | Logical access, monitoring |
| HIPAA | 164.312a, 164.312e | Access control, transmission security |

```rust
use forjar::core::compliance::{evaluate_benchmark, count_by_severity};

let findings = evaluate_benchmark("cis", &config);
let (critical, high, medium, low) = count_by_severity(&findings);
```

## Compliance Packs (FJ-3205)

YAML-defined rule bundles with five check types: `assert`, `deny`, `require`, `require_tag`, `script`.

```rust
use forjar::core::compliance_pack::{parse_pack, evaluate_pack};

let pack = parse_pack(yaml).unwrap();
let result = evaluate_pack(&pack, &resources);
println!("Pass rate: {:.0}%", result.pass_rate());
```

## Policy Coverage (FJ-3208)

Analyzes which resources have policies and which are uncovered:

```rust
use forjar::core::policy_coverage::{compute_coverage, format_coverage};

let cov = compute_coverage(&config);
println!("{}", format_coverage(&cov));  // "Policy Coverage: 80.0% (4/5)"
```

## Rulebook Validation (FJ-3108)

Validates rulebook YAML for semantic correctness:

- Duplicate names, empty events/actions
- Empty `apply.file`, empty `notify.channel`
- Zero cooldown warning, high retry warning
- bashrs lint + secret leak detection on scripts

## Rulebook Runtime (FJ-3106)

Event-driven evaluation with cooldown deduplication:

```rust
use forjar::core::rules_runtime::{evaluate_event, fired_actions};

let results = evaluate_event(&event, &config, &mut tracker);
let actions = fired_actions(&event, &config, &mut tracker);
```

## Ephemeral Values (FJ-3302)

Hash-and-discard secret pipeline with drift detection:

- `resolve_ephemerals`: Provider chain resolution
- `to_records`: Strip plaintext, keep BLAKE3 hash
- `check_drift`: Unchanged/Changed/New status
- `substitute_ephemerals`: Template `{{ephemeral.KEY}}` replacement

## MC/DC Analysis (FJ-051)

DO-178C DAL-A structural coverage test generation:

```rust
use forjar::core::mcdc::{build_decision, generate_mcdc_and};

let d = build_decision("a && b && c", &["a", "b", "c"]);
let report = generate_mcdc_and(&d);
// 3 conditions → 3 pairs, 4 minimum tests
```

## Shell Purification (FJ-036)

Three levels of shell safety via bashrs integration:
- `validate_script()` — lint errors only
- `lint_script()` — full diagnostics
- `purify_script()` — parse → purify AST → reformat

## Docker Migration (FJ-044)

Docker → pepita kernel isolation conversion:

| Docker | Pepita |
|--------|--------|
| `image` | Extract rootfs → `overlay_lower` |
| `ports` | `netns: true` + manual iptables |
| `running` | `present` |
| `stopped` | `absent` |

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_compliance_policy.rs` | 12 | 474 |
| `falsification_rules_engine_runtime.rs` | 24 | 464 |
| `falsification_ephemeral_mcdc_migrate.rs` | 38 | 470 |
| **Total** | **74** | **1,408** |
