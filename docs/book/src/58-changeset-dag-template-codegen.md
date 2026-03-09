# Changeset, DAG Ordering, Templates & Codegen

Falsification coverage for FJ-046, FJ-216, FJ-2300, and FJ-005.

## Minimal Changeset (FJ-046)

Computes the provably minimal set of resource mutations needed to transition from current state to desired state:

- Phase 1: Compare hashes — mark resources with changed or missing hashes
- Phase 2: Propagate through dependency graph — mark transitive dependents

```rust
use forjar::core::planner::minimal_changeset::{compute_minimal_changeset, verify_minimality};

let changeset = compute_minimal_changeset(&resources, &locks, &deps);
assert!(verify_minimality(&changeset));
println!("{} changes needed", changeset.changes_needed);
```

Key properties:
- No changes when all hashes match
- New resources (missing from locks) always marked necessary
- Dependency propagation is transitive (A→B→C: if A changes, C also changes)
- `is_provably_minimal` always true by construction

## DAG Ordering (FJ-216)

### Topological Order

Kahn's algorithm with alphabetical tie-breaking for deterministic execution order:

```rust
use forjar::core::resolver::build_execution_order;

let order = build_execution_order(&config)?;
// Dependencies always appear before dependents
```

### Parallel Waves

Groups resources into concurrent execution waves:

```rust
use forjar::core::resolver::compute_parallel_waves;

let waves = compute_parallel_waves(&config)?;
// Wave 1: [nginx]  Wave 2: [certbot]  Wave 3: [webapp]
```

Error detection: cycle detection, unknown dependency references.

## Template Resolution (FJ-2300)

Resolves `{{params.key}}` and `{{machine.name.field}}` templates:

```rust
use forjar::core::resolver::resolve_template;

let result = resolve_template(
    "host: {{machine.web.addr}}:{{params.port}}",
    &params, &machines,
)?;
```

Machine fields: `addr`, `hostname`, `user`, `arch`.

### Secret Redaction

```rust
use forjar::core::resolver::redact_secrets;

let safe = redact_secrets(output, &["s3cret".into()]);
// All occurrences replaced with "***"
```

## Codegen Dispatch (FJ-005)

Script generation for all 15 resource types (Recipe rejects with "expand first"):

| Function | Purpose |
|----------|---------|
| `check_script` | Read current state |
| `apply_script` | Converge to desired state |
| `state_query_script` | Query observable state for hashing |

Sudo wrapping (FJ-1394): when `resource.sudo = true`, wraps script in `sudo bash` heredoc.

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_changeset_dag_staleness.rs` | 17 | 226 |
| `falsification_template_codegen.rs` | 24 | 314 |
| **Total** | **41** | **540** |
