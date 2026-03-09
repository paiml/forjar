# Parser Validation, Policy & Recipe Expansion

Falsification coverage for FJ-2501, FJ-2500, FJ-220, FJ-3200, FJ-3207, and FJ-004.

## Format Validation (FJ-2501)

Validates string formats that serde accepts as `String` but have domain constraints:

| Check | Rule |
|-------|------|
| Mode | Exactly 4 octal digits (0-7), e.g., `0644`, `1755` |
| Port | Integer 1-65535 |
| Path | Must be absolute (start with `/`) |
| Owner/Group | Unix name: `^[a-z_][a-z0-9_-]{0,31}$` |
| Cron schedule | 5 fields with valid ranges, or `@keyword` |
| Machine addr | Non-empty, no spaces, or special markers |

Template expressions (`{{...}}`) are skipped during validation.

### deny_paths

Policy glob patterns block resource paths:

```yaml
policy:
  deny_paths:
    - "/proc/**"
    - "/sys/**"
```

## Unknown Field Detection (FJ-2500)

Two-pass parsing: after serde deserializes, walks raw YAML keys against known field sets.

```rust
use forjar::core::parser::check_unknown_fields;

let warnings = check_unknown_fields(yaml);
// "unknown field 'packges' — did you mean 'packages'?"
```

- Levenshtein distance ≤ 2 triggers "did you mean?" suggestions
- Substring fallback for longer typos
- Covers: config, resource, machine, policy, recipe fields

## Policy-as-Code (FJ-220 / FJ-3200)

Five rule types with scope filtering:

| Type | Semantics |
|------|-----------|
| `require` | Field must be set |
| `deny` | Condition match blocks apply |
| `warn` | Advisory warning, apply proceeds |
| `assert` | Condition must be true |
| `limit` | Field count within min/max bounds |

```rust
use forjar::core::parser::{evaluate_policies_full, policy_check_to_json};

let result = evaluate_policies_full(&config);
let json = policy_check_to_json(&result);
```

### SARIF Output (FJ-3207)

```rust
use forjar::core::parser::policy_check_to_sarif;

let sarif = policy_check_to_sarif(&result);
// Valid SARIF 2.1.0 for GitHub Code Scanning
```

## Recipe Expansion (FJ-004)

### Parsing

```rust
use forjar::core::recipe::parse_recipe;

let recipe_file = parse_recipe(yaml)?;
```

### Input Validation

Type-checked inputs with bounds and enum constraints:

| Type | Validation |
|------|-----------|
| `string` | Any value |
| `int` | Optional min/max bounds |
| `bool` | Must be boolean |
| `path` | Must be absolute |
| `enum` | Must match choices list |

### Expansion

`expand_recipe` produces namespaced resources with resolved templates:

```rust
use forjar::core::recipe::expand_recipe;

let expanded = expand_recipe("web", &recipe_file, &machine, &inputs, &[])?;
// "web/pkg", "web/conf" — with {{inputs.*}} resolved
```

- Internal `depends_on` references namespaced automatically
- External dependencies added to first resource
- Machine target propagated to all resources

### Terminal ID

```rust
let tid = recipe_terminal_id("web", &recipe_file);
// Some("web/conf") — last resource for external dependency chaining
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_parser_format_unknown.rs` | 23 | 427 |
| `falsification_parser_unknown_fields.rs` | 10 | 170 |
| `falsification_parser_policy.rs` | 18 | 373 |
| **Total** | **51** | **970** |
