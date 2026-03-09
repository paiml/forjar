# Include Hardening & Deep Validation

Forjar provides hardened include merging and multi-layer deep validation for production safety.

## Include Hardening (FJ-2502)

Three protections for `includes:` directives:

| Protection | Mechanism |
|-----------|-----------|
| **Circular detection** | Visited path set — same file included twice is rejected |
| **Conflict warnings** | stderr warning when an include overwrites an existing key |
| **Provenance tracking** | `include_provenance` maps "resource:id" / "machine:id" to source file |

```yaml
# base.yaml
version: "1.0"
name: my-stack
includes:
  - machines.yaml
  - packages.yaml
resources: {}
```

Provenance is available after parsing:

```rust
use forjar::core::parser::parse_and_validate;

let config = parse_and_validate("base.yaml".as_ref()).unwrap();
// "resource:nginx" → "packages.yaml"
let source = config.include_provenance.get("resource:nginx");
```

### Restrictions

- Only single-level includes — nested includes are ignored with a warning
- Duplicate includes (same file twice) trigger a circular include error
- Provenance is not serialized to YAML (`#[serde(skip)]`)

## Deep Validation (FJ-2503)

`forjar validate --deep` runs 11 checks beyond structural validation:

| Check | What it detects |
|-------|----------------|
| `templates` | Unresolved `{{params.*}}` references |
| `overlaps` | Multiple resources targeting the same path |
| `circular-deps` | DAG cycles in `depends_on` chains |
| `secrets` | Hardcoded passwords, API keys, tokens in YAML |
| `naming` | Resources not matching kebab-case convention |
| `connectivity` | Remote machines missing hostname or addr |
| `machine-refs` | Resources referencing non-existent machines |
| `state-values` | Invalid state values (e.g., "running" on a file resource) |
| `drift-coverage` | Resources lacking `state_query` scripts |
| `idempotency` | Unknown resource types that may not be idempotent |
| `exhaustive` | Orphaned params, dangling refs, unresolved content params |

### Check Flags

```rust
use forjar::core::types::DeepCheckFlags;

let all = DeepCheckFlags::exhaustive(); // all 10 flags enabled
let minimal = DeepCheckFlags::default(); // all disabled
assert!(all.any_enabled());
assert!(!minimal.any_enabled());
```

### Validation Output

```rust
use forjar::core::types::{ValidateOutput, ValidationFinding, ValidationSeverity};

let findings = vec![
    ValidationFinding::error("missing packages")
        .for_resource("nginx")
        .for_field("packages"),
    ValidationFinding::warning("deprecated field")
        .with_suggestion("use 'ensure' instead"),
];
let output = ValidateOutput::from_findings(findings, 12, 3);
assert!(!output.valid);
assert_eq!(output.error_count(), 1);
assert_eq!(output.warning_count(), 1);
println!("{}", output.format_summary());
```

### Severity Model

Three levels: `Error > Warning > Hint`

- **Error**: Blocks validation and apply
- **Warning**: Printed to stderr, does not block by default
- **Hint**: Informational only

## Unknown Field Detection (FJ-2500)

Two-pass parsing detects typos with Levenshtein distance ≤2 suggestions:

```rust
use forjar::core::parser::check_unknown_fields;

let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packges: [curl]
"#;
let warnings = check_unknown_fields(yaml);
// "unknown field 'packges' — did you mean 'packages'?"
assert!(!warnings.is_empty());
```

### Field Suggestion

```rust
use forjar::core::types::FieldSuggestion;

let s = FieldSuggestion::new("packges", "packages", 1);
assert!(s.should_suggest()); // distance ≤ 2
```

## Cycle Detection

Kahn's algorithm with deterministic tie-breaking detects dependency cycles:

```rust
use forjar::core::parser::parse_config;
use forjar::core::resolver::build_execution_order;

let yaml = r#"
version: "1.0"
name: test
resources:
  a:
    type: package
    depends_on: [b]
  b:
    type: package
    depends_on: [a]
"#;
let config = parse_config(yaml).unwrap();
let result = build_execution_order(&config);
assert!(result.is_err()); // "dependency cycle detected"
```

## Falsification

```bash
cargo test --test falsification_include_deep_validation
```

Key invariants verified:
- Severity ordering: Error > Warning > Hint
- ValidationFinding builder chain: resource → field → suggestion
- ValidateOutput marks valid only when zero errors
- FieldSuggestion threshold: distance ≤ 2 suggests, > 2 does not
- DeepCheckFlags default all false, exhaustive all true
- Unknown field detection catches typos with Levenshtein suggestions
- Include provenance tracked across merge, not serialized
- Duplicate includes rejected as circular
- Nonexistent includes error cleanly
- DAG cycle detection via Kahn's algorithm
- Dangling machine and dependency references caught
