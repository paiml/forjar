# DAG Ordering, Minimal Changeset & Template Resolution

Forjar computes a deterministic execution order from resource dependencies,
identifies the provably minimal set of changes, and resolves template variables
across all resource fields.

## Execution Order (FJ-216)

Kahn's topological sort with alphabetical tie-breaking:

```rust
use forjar::core::resolver::{build_execution_order, compute_parallel_waves};

let order = build_execution_order(&config)?;
// ["nginx-pkg", "nginx-conf", "nginx-svc"]

let waves = compute_parallel_waves(&config)?;
// [["nginx-pkg"], ["nginx-conf"], ["nginx-svc"]]
// Resources in the same wave can execute concurrently.
```

Cycle detection returns an error naming the involved resources.

## Minimal Changeset (FJ-046)

Computes the minimum mutations needed to reach desired state:

```rust
use forjar::core::planner::minimal_changeset::{compute_minimal_changeset, verify_minimality};

let cs = compute_minimal_changeset(&resources, &locks, &deps);
// cs.changes_needed = 1 (only nginx-pkg hash changed)
// cs.changes_skipped = 2 (conf and svc unchanged)

// With dependency propagation:
// If nginx-pkg changed and nginx-conf depends on it,
// nginx-conf is also marked as necessary.
assert!(verify_minimality(&cs));
```

A change is necessary if:
1. No lock entry exists (new resource)
2. Current hash differs from desired hash
3. A dependency was marked necessary (transitive propagation)

## Template Resolution (FJ-003)

Resolves `{{params.*}}` and `{{machine.*.*}}` variables:

```rust
use forjar::core::resolver::{resolve_template, resolve_resource_templates};

let result = resolve_template(
    "server {{machine.web-01.hostname}} port={{params.port}}",
    &params, &machines,
)?;
// "server web-01.example.com port=8080"

let resolved = resolve_resource_templates(&resource, &params, &machines)?;
// All Option<String> fields resolved: path, content, owner, mode, etc.
```

Machine fields: `addr`, `hostname`, `user`, `arch`.

## State Reconstruction (FJ-1280)

Replay the event log to any point in time:

```rust
use forjar::core::state::reconstruct::reconstruct_at;

let lock = reconstruct_at(state_dir, "web-01", "2026-03-01T00:00:00Z")?;
// Returns StateLock as it was at that timestamp
```

## Drift Detection (FJ-016)

Compare live filesystem state to lock hashes:

```rust
use forjar::tripwire::drift::check_file_drift;

let finding = check_file_drift("my-config", "/etc/app.conf", &expected_hash);
// None = no drift, Some(DriftFinding) = content changed or missing
```

## Falsification Tests

```bash
cargo run --example dag_changeset_resolve
cargo test --test falsification_dag_changeset
cargo test --test falsification_template_drift_reconstruct
```
