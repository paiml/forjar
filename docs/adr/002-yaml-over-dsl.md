# ADR-002: YAML Configuration Over Custom DSL

## Status
Accepted

## Context
Infrastructure configuration needs a human-readable format. Options:
- Custom DSL (like Terraform's HCL): Maximum expressiveness, highest learning curve
- General-purpose language (like Pulumi's TypeScript): Full programming power, complex toolchain
- YAML: Universal, no new syntax, excellent tooling support
- TOML: Simpler than YAML but poor support for nested structures

## Decision
Use YAML for all configuration. Recipes are YAML. Lock files are YAML. No custom DSL.

## Consequences
- **Positive**: Zero learning curve for anyone who knows YAML
- **Positive**: Every editor has YAML support, syntax highlighting, validation
- **Positive**: serde_yaml provides type-safe deserialization directly to Rust structs
- **Negative**: YAML has implicit type coercion gotchas (`no` → `false`, `1.0` → float)
- **Negative**: No conditionals or loops in config (by design — use recipes for composition)

## Falsification
This decision would be wrong if:
1. Users frequently need conditional logic that recipes can't express
2. YAML parsing becomes a performance bottleneck (measure with benchmarks)
3. serde_yaml's error messages are too cryptic for users (monitor feedback)
