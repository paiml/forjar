# Planner: Proof Obligations, SAT Solving & Why Explanation

Falsification coverage for FJ-1385, FJ-1382, FJ-045, and FJ-1379.

## Proof Obligation Taxonomy (FJ-1385)

Every resource operation is classified into one of four formal categories:

| Category | Property | Safe? |
|----------|----------|-------|
| Idempotent | `f(f(x)) = f(x)` — safe to re-run | Yes |
| Monotonic | Only adds state, never removes | Yes |
| Convergent | Reaches same fixed point from any start | Yes |
| Destructive | Removes state that may not be reconstructable | No |

Classification by action:

| Action | File | Service | Package | Model | Docker |
|--------|------|---------|---------|-------|--------|
| NoOp | Idempotent | Idempotent | Idempotent | Idempotent | Idempotent |
| Create | Idempotent | Convergent | Idempotent | Monotonic | Convergent |
| Update | Idempotent | Convergent | Convergent | Convergent | Convergent |
| Destroy | Destructive | Convergent | Convergent | Destructive | Convergent |

```rust
use forjar::core::planner::proof_obligation::{classify, label, is_safe};

let po = classify(&ResourceType::File, &PlanAction::Destroy);
assert_eq!(label(&po), "destructive");
assert!(!is_safe(&po));
```

## Reversibility Classification (FJ-1382)

Operations are classified as Reversible or Irreversible:

- **NoOp, Create, Update** → always Reversible
- **Destroy** → depends on resource type and config:
  - File with `content` or `source` → Reversible (re-createable)
  - Bare file → Irreversible
  - Service, Package, Cron, Mount → Reversible
  - User, Network, Model, Task, Recipe → Irreversible

```rust
use forjar::core::planner::reversibility::{classify, count_irreversible, warn_irreversible};

let count = count_irreversible(&config, &plan);
let warnings = warn_irreversible(&config, &plan);
```

## SAT Dependency Resolution (FJ-045)

DPLL-style boolean satisfiability solver for dependency constraints:

- Each resource is a boolean variable (true = included)
- Dependencies become implications: `A depends_on B` → `(!A || B)`
- All resources get unit clauses (must be included)

```rust
use forjar::core::planner::sat_deps::{build_sat_problem, solve, SatResult};

let problem = build_sat_problem(&resources, &deps);
match solve(&problem) {
    SatResult::Satisfiable { assignment } => { /* all deps met */ }
    SatResult::Unsatisfiable { conflict_clause } => { /* conflict */ }
}
```

Features:
- Unit propagation for fast constraint simplification
- Backtracking search for complex dependency graphs
- Diamond dependency support
- Unknown dependency references safely skipped
- Conflict clause names in unsatisfiable results

## Why Explanation (FJ-1379)

Per-resource change explanation with hash comparison:

| State | Lock Status | Action | Reason |
|-------|-------------|--------|--------|
| absent | not in lock | NoOp | Nothing to destroy |
| absent | in lock | Destroy | Will be removed |
| present | no lock | Create | First apply |
| present | not in lock | Create | New resource |
| present | Failed | Update | Retry previous failure |
| present | Drifted | Update | Re-converge |
| present | hash match | NoOp | Already converged |
| present | hash changed | Update | Config changed |

```rust
use forjar::core::planner::why::{explain_why, format_why};

let reason = explain_why("nginx-conf", &resource, "web-01", &locks);
println!("{}", format_why(&reason));
// nginx-conf on web-01 -> Update
//   - hash changed: abc123... -> def456...
//   - content changed
```

Field diff detection: content_hash, path, version, packages.

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_planner_proof_rev.rs` | 20 | 369 |
| `falsification_planner_sat_why.rs` | 22 | 331 |
| **Total** | **42** | **700** |
