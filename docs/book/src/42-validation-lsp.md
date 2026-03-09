# Format Validation & LSP

Forjar provides multi-level validation for YAML configs and a Language Server Protocol (LSP) implementation for editor integration.

## Format Validation (FJ-2501)

Six format validators run on every parsed config:

| Validator | Field | Rule |
|-----------|-------|------|
| Mode | `mode` | Exactly 4 octal digits (0–7), e.g., `0644`, `1755` |
| Port | `port` | Integer 1–65535 |
| Path | `path` | Must start with `/` (absolute) |
| Owner/Group | `owner`, `group` | Unix name: `^[a-z_][a-z0-9_-]*$`, max 32 chars |
| Cron Schedule | `schedule` | 5 fields with valid ranges, or `@daily`/`@hourly` etc. |
| Machine Address | `addr` | Non-empty, no spaces (IP or hostname) |

Template expressions (`{{params.*}}`) skip validation automatically.

```rust
use forjar::core::parser::{parse_config, validate_config};

let yaml = r#"
version: "1.0"
name: test
resources:
  cfg:
    type: file
    path: /etc/app.conf
    mode: "0644"
    owner: www-data
    content: "key: value"
"#;
let config = parse_config(yaml).unwrap();
let errors = validate_config(&config);
assert!(errors.is_empty());
```

## Unknown Field Detection (FJ-2500)

Two-pass parsing detects typos and unknown YAML keys:

1. **First pass**: serde deserialization (ignores unknown fields)
2. **Second pass**: raw YAML walk against known field sets

Includes Levenshtein distance ≤2 for "did you mean?" suggestions.

```rust
use forjar::core::parser::check_unknown_fields;

let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packages: [curl]
    bogus_field: true
"#;
let warnings = check_unknown_fields(yaml);
assert!(!warnings.is_empty()); // "unknown field 'bogus_field'"
```

## LSP Server (FJ-2504)

Lightweight LSP server over stdio (JSON-RPC 2.0):

- **Completion**: top-level keys, resource types, resource fields
- **Hover**: documentation for known keys
- **Diagnostics**: YAML parse errors, unknown `ensure` values, tab warnings, unknown fields, structural validation

```rust
use forjar::cli::lsp::{LspServer, validate_yaml, DiagnosticSeverity};

// Validate a document directly
let diags = validate_yaml("{ bad: [yaml");
assert_eq!(diags[0].severity, DiagnosticSeverity::Error);

// Or use the full LSP server
let mut server = LspServer::new();
// Handle initialize, didOpen, completion, hover, etc.
```

## Deny Path Policy (FJ-2300)

Resources can be blocked by `policy.deny_paths` glob patterns:

```yaml
policy:
  deny_paths:
    - /etc/shadow
    - /root/**
```

Supports `*` (single segment) and `**` (any depth) globs.

## Falsification

```bash
cargo run --example validation_convergence_falsification
```

Key invariants verified:
- Valid octal modes accepted, invalid modes (digits 8/9, wrong length) rejected
- Port range 1–65535 enforced
- Relative paths rejected, absolute paths accepted
- Unix owner/group names validated against `^[a-z_][a-z0-9_-]*$`
- Cron schedules validated: field count, ranges, steps, keywords
- Unknown YAML fields flagged with suggestions
- LSP initialize handshake returns capabilities
- LSP diagnostics detect parse errors, tabs, unknown ensure values
