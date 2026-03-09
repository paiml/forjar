# SAT Solver, Minimal Changeset & State Reconstruction

Forjar uses formal methods for dependency verification, minimal change
computation, and point-in-time state recovery.

## SAT Dependency Solver (FJ-045)

A DPLL-style boolean satisfiability solver verifies that resource
dependency constraints are satisfiable before plan execution.

Each resource is a boolean variable (true = included). Dependencies
become implications: `A depends_on B` → `(!A || B)` in CNF.

```rust
use forjar::core::planner::sat_deps::{build_sat_problem, solve, SatResult};

let resources = vec!["nginx".into(), "app".into(), "db".into()];
let deps = vec![
    ("app".into(), "nginx".into()),   // app needs nginx
    ("app".into(), "db".into()),       // app needs db
];

let problem = build_sat_problem(&resources, &deps);
match solve(&problem) {
    SatResult::Satisfiable { assignment } => {
        // All resources can be deployed together
        assert!(assignment.values().all(|&v| v));
    }
    SatResult::Unsatisfiable { conflict_clause } => {
        // Contradictory constraints — report conflict
        eprintln!("Conflict: {:?}", conflict_clause);
    }
}
```

When constraints are unsatisfiable, the solver reports the first
conflicting clause with named variables for diagnostics.

## Minimal Changeset (FJ-046)

Computes the provably minimal set of resource mutations needed to
transition from current state to desired state.

```rust
use forjar::core::planner::minimal_changeset::{
    compute_minimal_changeset, verify_minimality,
};
use std::collections::BTreeMap;

let resources = vec![
    ("nginx".into(), "web".into(), "h-new".into()),
    ("mysql".into(), "db".into(),  "h-same".into()),
    ("app".into(),   "web".into(), "h-same".into()),
];

let mut locks = BTreeMap::new();
locks.insert("nginx@web".into(), "h-old".into());  // changed
locks.insert("mysql@db".into(),  "h-same".into()); // unchanged
locks.insert("app@web".into(),   "h-same".into()); // unchanged

// app depends on nginx — if nginx changes, app must re-apply
let deps = vec![("app".into(), "nginx".into())];

let changeset = compute_minimal_changeset(&resources, &locks, &deps);
assert_eq!(changeset.changes_needed, 2);  // nginx + app (propagated)
assert_eq!(changeset.changes_skipped, 1); // mysql
assert!(verify_minimality(&changeset));
```

A change is necessary if:
1. No lock entry exists (new resource)
2. Current hash differs from desired hash
3. A dependency changed and this resource depends on it

Dependency propagation is transitive: if A→B→C and A changes,
both B and C are marked as necessary.

## State Reconstruction (FJ-1280)

Event-sourced state recovery replays `events.jsonl` up to a given
timestamp to rebuild a `StateLock` at any point in time.

```rust
use forjar::core::state::reconstruct;

let lock = reconstruct::reconstruct_at(
    &state_dir,
    "web",
    "2026-03-09T15:00:00Z",
)?;
// lock contains all resources converged before 15:00
```

Supported events:
- `ResourceConverged` — sets resource to Converged with hash
- `ResourceFailed` — sets resource to Failed with error
- `DriftDetected` — updates resource status to Drifted
- `ApplyStarted` — captures hostname

This enables point-in-time recovery and audit trails.

## Rulebook Event Log (FJ-3107)

Records triggered rulebook events and outcomes to
`rulebook-events.jsonl` for audit and debugging.

```rust
use forjar::core::state::rulebook_log;
use forjar::core::types::*;

let event = InfraEvent {
    event_type: EventType::FileChanged,
    timestamp: "2026-03-09T12:00:00Z".into(),
    machine: Some("web-01".into()),
    payload: HashMap::new(),
};

let entry = rulebook_log::make_entry(
    &event, "config-repair", "apply", true, None,
);
rulebook_log::append_entry(&state_dir, &entry)?;

let entries = rulebook_log::read_entries(&state_dir)?;
```

Each entry records: timestamp, rulebook name, event type, machine,
action type, success/failure, and optional error message.

## Falsification Example

```bash
cargo run --example sat_changeset_falsification
```
