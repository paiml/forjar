# 13: Configuration Validation

> YAML parsing, structural validation, type checking, and the path to zero silent failures.

**Spec ID**: FJ-2500–FJ-2504 | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Overview

Forjar configurations are YAML files that declare machines, resources, policies, and data sources. A single typo in a resource field can cause silent misconfiguration — the wrong package installed, a service not started, a file written to the wrong path. The validation pipeline must catch these errors **before** any script reaches a target machine.

### Current Pipeline

```
YAML file
    │
    ▼
┌─────────────────────┐
│ 1. Parse             │  serde_yaml_ng::from_str()
│    Syntax errors     │  Type mismatches
│    ✗ Unknown fields  │  (#[serde(default)] everywhere)
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ 2. Include Merge     │  merge_includes()
│    File existence    │  Key-based merge (last wins)
│    ✗ Circular refs   │  ✗ Semantic conflicts
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ 3. Structural        │  validate_config()
│    Version == "1.0"  │  Name not empty
│    Machine refs      │  depends_on refs
│    Resource types    │  State enums
│    ✗ Unknown fields  │  ✗ Format validation
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ 4. Recipe Expansion  │  expand_recipes()
│    Input types       │  Bounds, enums, paths
│    ✗ Template syntax │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ 5. Resource Expand   │  expand_resources()
│    count/for_each    │  Template substitution
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ 6. Data Sources      │  resolve_data_sources()
│    File readability  │  Command execution
│    DNS resolution    │  State lock parsing
│    ✗ Only at apply   │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ 7. Pre-Apply Gates   │  apply_pre_validate()
│    State integrity   │  Policy violations
│    Security scanner  │  Destructive confirm
│    ✗ Only at apply   │
└────────┘

    ✗ = gap or limitation
```

### Design Principle

Validation has three tiers, each with different cost/coverage tradeoffs:

| Tier | When | Cost | Coverage |
|------|------|------|----------|
| **Fast** | Every keystroke (LSP) | <10ms | Syntax, field hints |
| **Full** | `forjar validate` | <500ms | Structural, refs, types |
| **Deep** | `forjar validate --exhaustive` | <5s | Templates, deps, connectivity |

---

## FJ-2500: Parse-Time Validation

### YAML Deserialization

```rust
pub fn parse_config(yaml: &str) -> Result<ForjarConfig, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("YAML parse error: {e}"))
}
```

**What this catches:**
- Malformed YAML (bad indentation, unmatched brackets)
- Type mismatches (`version: 123` instead of `version: "1.0"`)
- Invalid enum variants (serde rejects unknown variants)

**Critical gap: unknown fields are silently dropped.**

Every struct uses `#[serde(default)]` on optional fields and no struct uses `#[serde(deny_unknown_fields)]`. This means:

```yaml
resources:
  nginx:
    type: package
    packges: [nginx]     # TYPO: "packges" instead of "packages"
    provider: apt
```

This parses successfully. `packages` defaults to `[]` (empty). The resource deploys with zero packages — silent data loss.

### Gap G1: Unknown Field Detection

**Current state**: All unknown fields silently ignored.

**Target**: Warn on unknown fields at parse time. Error in strict mode.

**Approach**: Two-pass parsing.

```
Pass 1: serde_yaml_ng::from_str::<ForjarConfig>(yaml)     → typed config
Pass 2: serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml) → raw tree

Compare raw keys against known struct fields.
Any key in raw tree not consumed by typed parse → unknown field warning.
```

**Known field sets per struct:**

| Struct | Known Fields |
|--------|-------------|
| `ForjarConfig` | `version`, `name`, `params`, `machines`, `resources`, `includes`, `policy`, `policies`, `outputs`, `data` |
| `Resource` | `type`, `name`, `machine`, `state`, `depends_on`, `triggers`, `tags`, `arch`, `count`, `for_each`, `packages`, `provider`, `path`, `content`, `source`, `target`, `mode`, `owner`, `group`, `command`, `schedule`, `port`, `protocol`, `action`, `image`, `driver_version`, `format`, `quantization`, `checksum`, `cache_dir`, `recipe`, `timeout`, `template`, ... |
| `Machine` | `hostname`, `addr`, `user`, `arch`, `transport`, `container`, `gpu_backend`, `rocm_version`, `allowed_operators` |

**Implementation**: `validate_unknown_fields(raw: &Value, known: &[&str]) -> Vec<String>`

**Behavior by mode:**

| Mode | Unknown field | Effect |
|------|--------------|--------|
| Default | Warning | Printed to stderr, does not block |
| `--strict` | Error | Blocks validation |
| `--deny-unknown-fields` | Error | Blocks validation |
| LSP | Diagnostic (Warning) | Yellow squiggly |

---

## FJ-2501: Structural Validation

### Version and Name

```rust
pub fn validate_config(config: &ForjarConfig) -> Vec<ValidationError>
```

| Check | Rule | Error |
|-------|------|-------|
| Version | Must be exactly `"1.0"` | `"unsupported version: {v}, expected 1.0"` |
| Name | Must not be empty | `"config name is required"` |

### Machine Validation

```rust
fn validate_machine(key: &str, machine: &Machine, errors: &mut Vec<ValidationError>)
```

| Check | Rule |
|-------|------|
| Architecture | Must be in: `x86_64`, `aarch64`, `armv7l`, `riscv64`, `s390x`, `ppc64le` |
| Container transport | Must have `container:` block |
| Container runtime | Must be `docker` or `podman` |
| Ephemeral container | Must have `image:` field |

### Resource Reference Validation

```rust
fn validate_resource_refs(config, id, resource, errors)
```

| Check | Rule |
|-------|------|
| Machine refs | Must exist in `machines` or be `"localhost"` |
| depends_on | Must reference existing resources (skip if contains `{{item}}` or `{{index}}`) |
| triggers | Same as depends_on |
| Self-reference | `depends_on: [self]` forbidden |
| count/for_each | Cannot have both; count >= 1; for_each non-empty |

### Type-Specific Required Fields

```rust
fn validate_resource_type(id, resource, errors)
```

| Type | Required Fields | Valid States |
|------|-----------------|-------------|
| `package` | `packages` (non-empty), `provider` | — |
| `file` | `path`; cannot have both `content` AND `source` | `file`, `directory`, `symlink`, `absent` |
| `service` | `name` | `running`, `stopped`, `enabled`, `disabled` |
| `mount` | `source`, `path` | `mounted`, `unmounted`, `absent` |
| `user` | `name` | — |
| `docker` | `name`, `image` (unless state=absent) | `running`, `stopped`, `absent` |
| `cron` | `name`, `schedule` (5 fields), `command` (unless state=absent) | `present`, `absent` |
| `network` | `port`; protocol in `tcp`/`udp`; action in `allow`/`deny`/`reject` | — |
| `pepita` | `name`; cpuset non-empty if provided | `present`, `absent` |
| `model` | `name` | `present`, `absent` |
| `gpu` | `driver_version` | `present`, `absent` |
| `recipe` | `recipe` | — |
| `task` | `command`; timeout > 0 if set | — |

### Gap G2: Format Validation

Fields accepted as strings but never format-checked:

| Field | Expected Format | Current | Target |
|-------|----------------|---------|--------|
| `mode` | Octal string (`"0644"`) | Any string | Regex `^0[0-7]{3}$` |
| `port` | 1-65535 | Any integer | Range check |
| `schedule` | 5 cron fields | Field count only | Validate each field (ranges, wildcards) |
| `path` | Absolute path | Any string | Must start with `/` (except relative `source:`) |
| `addr` | IP or hostname | Any string | Regex or DNS-parseable |
| `owner`/`group` | Unix name | Any string | Regex `^[a-z_][a-z0-9_-]*$` |

**Implementation**: Add `validate_field_formats(id, resource, errors)` called after type validation.

---

## FJ-2502: Include Merging

### Merge Strategy

```rust
pub fn merge_includes(base: ForjarConfig, base_dir: &Path) -> Result<ForjarConfig, String>
```

| Section | Strategy | Conflict Resolution |
|---------|----------|-------------------|
| `params` | Insert/overwrite by key | Later wins |
| `machines` | Insert/overwrite by key | Later wins |
| `resources` | Insert/overwrite by key | Later wins |
| `policy` | Replace wholesale | Later wins (not merged) |
| `policies` | Extend (append) | Accumulated |
| `outputs` | Insert/overwrite by key | Later wins |
| `data` | Insert/overwrite by key | Later wins |

**Constraint**: Includes are **not recursive**. Included files cannot themselves have `includes:`. Only the base file can include.

### Gap G3: Include Validation

| Gap | Risk | Fix |
|-----|------|-----|
| No circular reference detection | Stack overflow on `a.yaml includes b.yaml includes a.yaml` | Track visited paths in merge |
| No conflict reporting | User doesn't know machine X was overwritten by include | Warn when key exists in both |
| Policy replaced silently | Include B's policy replaces A's with no warning | Warn on policy replacement |
| No include depth tracking | Unclear which include introduced a resource | Include provenance in ValidationError |

---

## FJ-2503: Deep Validation

Deep validation checks are available via `forjar validate --check-*` flags. These are **OFF by default** because they may require network access, state directory access, or significant computation.

### Template Validation (`--check-templates`)

Validates that all `{{...}}` references resolve:

```yaml
resources:
  app:
    type: file
    path: /opt/{{params.app_name}}/config.yaml   # Does params.app_name exist?
    content: "port: {{inputs.port}}"               # Does inputs.port exist?
```

| Template Namespace | Source | Validated Against |
|-------------------|--------|-------------------|
| `{{params.*}}` | `params:` section | Key existence |
| `{{inputs.*}}` | Recipe inputs | Input declarations |
| `{{item}}` | `for_each:` expansion | for_each existence on resource |
| `{{index}}` | `count:` expansion | count existence on resource |
| `{{secrets.*}}` | Secret providers | Provider configured (not secret existence) |
| `{{data.*}}` | `data:` section | Data source declarations |

### Dependency Validation (`--check-circular-deps`)

Builds the DAG and checks for cycles:

```
fn detect_cycles(resources) -> Vec<Cycle>:
    for each resource:
        DFS with path tracking
        if visited node in current path → cycle found
```

Reports the full cycle path: `A → B → C → A`.

### Connectivity Validation (`--check-connectivity`)

For each machine, attempts SSH connection:

```
fn check_connectivity(machine) -> Result<(), String>:
    ssh -o ConnectTimeout=5 -o BatchMode=yes {user}@{addr} "echo ok"
```

### Secret Validation (`--check-secrets`)

Scans for hardcoded secrets in resource content:

| Pattern | Description |
|---------|-------------|
| `password:.*[^\{]` | Inline password (not a `{{ }}` template) |
| `secret:.*[^\{]` | Inline secret |
| `api_key:.*[^\{]` | Inline API key |
| `token:.*[^\{]` | Inline token |
| Base64-encoded strings >40 chars | Potential encoded credentials |

### Overlap Validation (`--check-overlaps`)

Detects conflicting resource targets on the same machine:

| Conflict | Example |
|----------|---------|
| Same file path | Two file resources writing `/etc/nginx/nginx.conf` |
| Same port | Two network resources claiming port 8080 |
| Same service name | Two service resources managing `nginx` |
| Same mount path | Two mount resources at `/data` |

### All Deep Checks

| Flag | Spec | Description |
|------|------|-------------|
| `--check-templates` | FJ-421 | Template variable resolution |
| `--check-circular-deps` | FJ-757 | Circular dependency detection |
| `--check-machine-refs` | FJ-761 | Machine reference validation |
| `--check-state-values` | FJ-769 | Valid state field values |
| `--check-connectivity` | FJ-411 | SSH connectivity test |
| `--check-secrets` | FJ-441 | Hardcoded secret scan |
| `--check-overlaps` | FJ-2503 | Conflicting resource targets |
| `--check-drift-coverage` | FJ-2503 | All resources have state_query |
| `--check-idempotency` | FJ-2503 | Script idempotency simulation |
| `--check-naming` | FJ-2503 | Resource naming conventions |
| `--exhaustive` | FJ-391 | Run all deep checks |

### Gap G4: Deep Checks Are Off by Default

The deep validation exists but nobody runs it unless they know the flags. Most users type `forjar validate` and get only structural checks.

**Target**: `forjar validate` runs fast + full checks. `forjar validate --deep` runs all checks. Individual `--check-*` flags for selective deep validation.

**Default escalation:**

| Severity | Behavior |
|----------|----------|
| Error (structural) | Always checked, blocks apply |
| Warning (unknown fields) | Always checked, printed to stderr |
| Deep (templates, deps) | Only with `--deep` or `--exhaustive` |
| Network (connectivity) | Only with `--check-connectivity` (requires network) |

---

## FJ-2504: LSP Real-Time Validation

### Current LSP Validation

```rust
fn validate_yaml(content: &str) -> Vec<Diagnostic>
```

| Check | Severity | Trigger |
|-------|----------|---------|
| YAML parse error | Error | Malformed syntax |
| Tab character | Warning | Any line with tab indentation |
| Invalid `ensure:` value | Warning | Not in `[present, absent, latest, running, stopped]` |

### Target LSP Validation

The LSP should provide the same fast-tier validation as `forjar validate` without the `--deep` checks:

| Check | Severity | New? |
|-------|----------|------|
| YAML parse error | Error | Existing |
| Tab character | Warning | Existing |
| Unknown fields | Warning | **New** |
| Missing required fields per type | Error | **New** |
| Invalid state values | Error | **New** |
| Machine reference missing | Error | **New** |
| depends_on reference missing | Warning | **New** |
| File mode format | Warning | **New** |
| Port out of range | Warning | **New** |
| Version mismatch | Error | **New** |

### LSP Diagnostic Format

```rust
pub struct Diagnostic {
    pub line: u32,
    pub character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub severity: DiagnosticSeverity,  // Error, Warning, Information, Hint
    pub message: String,
    pub source: String,                // "forjar"
}
```

Diagnostics published via `textDocument/publishDiagnostics` on every `didOpen` and `didChange`.

### JSON Schema Export (MCP)

Forjar exports JSON Schema for all MCP tool inputs/outputs via `schemars`:

```rust
fn export_schema() -> serde_json::Value
```

| Tool | Input Schema | Output Schema |
|------|-------------|---------------|
| `forjar_validate` | `{ path: String }` | `{ valid, resource_count, machine_count, errors }` |
| `forjar_plan` | `{ path, state_dir?, resource?, tag? }` | `{ changes, to_create, to_update, to_destroy }` |
| `forjar_drift` | `{ path, state_dir?, machine? }` | `{ drifted, findings }` |
| `forjar_lint` | `{ path }` | `{ warnings, warning_count, error_count }` |

**Future**: Export `ForjarConfig` JSON Schema for editor autocompletion (VS Code, Neovim).

---

## Pre-Apply Validation Gates

Before any script executes, `apply_pre_validate()` runs these gates:

```rust
fn apply_pre_validate(config, state_dir, ...) -> Result<(), String>
```

| Gate | Check | Blocks Apply? |
|------|-------|--------------|
| State integrity | BLAKE3 sidecar verification | Yes |
| Pre-apply drift | Local file hash comparison | Warning only |
| Destructive confirm | Destroy actions require `--yes` or `--confirm-destructive` | Yes |
| Policy violations | `require`/`deny` rules evaluated | Yes (deny), warning (warn) |
| Security scanner | Threshold-based security gate | Yes (if threshold exceeded) |
| Pre-apply hook | Custom `policy.pre_apply` script | Yes (if hook fails) |
| User confirmation | Interactive `[y/N]` prompt | Yes (if no `--yes`) |

These gates are **not optional** — they run on every `forjar apply`.

---

## Recipe Input Validation

```rust
fn validate_inputs(recipe, provided) -> Result<HashMap<String, String>, String>
```

| Input Type | Validation | Example |
|------------|-----------|---------|
| `string` | Any value accepted | `name: "my-app"` |
| `int` | Must be numeric; optional min/max bounds | `port: 8080` (min: 1, max: 65535) |
| `bool` | Must be YAML boolean | `enabled: true` |
| `path` | Must start with `/` (absolute) | `data_dir: /opt/data` |
| `enum` | Must be in `choices` list | `backend: nvidia` (choices: [nvidia, rocm, cpu]) |

Missing required inputs with no default → error. Unknown input type → error.

---

## Error Reporting

### Validation Output Format

**Default (human-readable):**
```
error: resource "nginx" missing required field "packages" for type "package"
error: resource "app-config" references unknown machine "staging"
warning: resource "db" has unknown field "packges" (did you mean "packages"?)
warning: file resource "config" mode "banana" is not valid octal (expected "0644")

2 errors, 2 warnings
```

**JSON (`--json`):**
```json
{
  "valid": false,
  "errors": [
    {"resource": "nginx", "field": "packages", "message": "missing required field for type package"},
    {"resource": "app-config", "field": "machine", "message": "references unknown machine 'staging'"}
  ],
  "warnings": [
    {"resource": "db", "field": "packges", "message": "unknown field (did you mean 'packages'?)"}
  ],
  "resource_count": 12,
  "machine_count": 3
}
```

### "Did You Mean?" Suggestions

For unknown fields, compute Levenshtein distance against known fields:

| Typo | Known Field | Distance | Suggestion |
|------|------------|----------|------------|
| `packges` | `packages` | 1 | Yes |
| `maching` | `machine` | 1 | Yes |
| `dependson` | `depends_on` | 1 | Yes |
| `provder` | `provider` | 1 | Yes |
| `foobar` | — | >3 | No suggestion |

Threshold: suggest if Levenshtein distance <= 2.

---

## Validation Pipeline Summary

```
                Fast (<10ms)              Full (<500ms)              Deep (<5s)
                ─────────────             ────────────────           ──────────────
LSP             YAML syntax               Unknown fields             —
                Tab detection             Missing required fields
                ensure: values            State enum validation
                                          Machine references

forjar validate YAML syntax               Unknown fields             Templates
                Version                   Required fields            Circular deps
                Name                      State enums                Overlaps
                                          Machine refs               Secrets scan
                                          depends_on refs            Naming
                                          Type-specific rules
                                          Format validation

forjar apply    (all of above)            Pre-apply gates            —
                                          State integrity
                                          Policy violations
                                          Security scanner
```

---

## Implementation

### Phase 23: Unknown Field Detection (FJ-2500)
- [ ] Two-pass parsing: typed + raw Value comparison
- [ ] Known field sets per struct
- [ ] Levenshtein "did you mean?" suggestions
- [ ] `--deny-unknown-fields` flag for strict mode
- [ ] LSP unknown field diagnostics
- **Deliverable**: Typos in field names produce warnings (default) or errors (strict)

### Phase 24: Format Validation (FJ-2501)
- [ ] `validate_field_formats()` for mode, port, path, addr, owner
- [ ] Cron schedule field range validation
- [ ] LSP format diagnostics
- **Deliverable**: `mode: "banana"` is an error, not silently accepted

### Phase 25: Include Hardening (FJ-2502)
- [ ] Circular include detection (visited path set)
- [ ] Conflict warnings on key overwrite
- [ ] Include provenance tracking
- **Deliverable**: Circular includes fail with clear error

### Phase 26: Default Deep Validation (FJ-2503)
- [ ] `forjar validate` runs fast + full by default
- [ ] `forjar validate --deep` runs all checks
- [ ] Template validation in `--deep`
- [ ] Circular dependency detection in `--deep`
- [ ] Overlap detection in `--deep`
- **Deliverable**: `forjar validate --deep` catches template and dependency errors

### Phase 27: LSP Enrichment (FJ-2504)
- [ ] Full structural validation in LSP
- [ ] `ForjarConfig` JSON Schema export for editor autocompletion
- [ ] Real-time unknown field + "did you mean?" diagnostics
- **Deliverable**: VS Code / Neovim show errors as you type
