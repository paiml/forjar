# Architecture

## Data Flow

```
forjar.yaml
    │
    ▼
┌─────────┐
│ parser   │  Parse YAML, validate schema
└────┬─────┘
     │
     ▼
┌─────────┐
│ recipe   │  Load recipes, validate inputs, expand into resources
└────┬─────┘
     │
     ▼
┌─────────┐
│ resolver │  Resolve {{params.X}} templates, build dependency DAG
└────┬─────┘
     │
     ▼
┌─────────┐
│ planner  │  Diff desired state vs BLAKE3 lock → execution plan
└────┬─────┘
     │
     ▼
┌─────────┐
│ codegen  │  Generate shell scripts per resource type
└────┬─────┘
     │
     ▼
┌─────────┐
│ executor │  Run scripts via local bash or SSH
└────┬─────┘
     │
     ▼
┌─────────┐
│ state    │  Atomic lock file write, event log append
└─────────┘
```

## Module Map

```
src/
  main.rs                CLI entry point
  lib.rs                 Library root
  cli/
    mod.rs               Subcommand dispatch (init, validate, plan, apply, drift, status)
  core/
    types.rs             All serde types (ForjarConfig, Resource, StateLock, etc.)
    parser.rs            YAML parsing + structural validation
    resolver.rs          Template resolution + Kahn's topological sort
    planner.rs           Desired-state diffing via BLAKE3 hash comparison
    codegen.rs           Shell script generation (dispatches to resources/)
    executor.rs          Orchestration loop (the main apply logic)
    state.rs             Lock file load/save (atomic write via temp+rename)
    recipe.rs            Recipe loading, input validation, namespaced expansion
  resources/
    mod.rs               Resource type registry
    package.rs           apt/cargo/pip package management
    file.rs              File, directory, symlink, absent
    service.rs           systemd service management
    mount.rs             NFS/bind mount management
  transport/
    mod.rs               Transport dispatch (local vs SSH)
    local.rs             Local bash execution
    ssh.rs               SSH execution (stdin pipe, no libssh2)
  tripwire/
    hasher.rs            BLAKE3 file/directory/string hashing
    drift.rs             Drift detection (hash comparison)
    eventlog.rs          Append-only JSONL provenance log
```

## DAG Resolution

Forjar uses **Kahn's algorithm** for topological sort with **alphabetical tie-breaking** to ensure deterministic execution order.

```
Input:  resources with depends_on edges
Output: linear execution order (deterministic)

Algorithm:
1. Build in-degree map from depends_on edges
2. Initialize min-heap with zero-degree nodes (sorted alphabetically)
3. While heap is not empty:
   a. Pop minimum node (alphabetical)
   b. Append to execution order
   c. Decrement in-degree of all dependents
   d. Push newly zero-degree nodes to heap
4. If |order| != |nodes|: cycle detected → error
```

## BLAKE3 Content Addressing

Every resource's desired state is hashed to a BLAKE3 digest:

```
hash = blake3(resource_type + "\0" + state + "\0" + provider + "\0" + ... + mode)
```

Format: `"blake3:{64 hex chars}"`

This hash is stored in the lock file. On the next `apply`, the planner computes the hash of the desired state and compares:
- **Match**: Skip (no-op)
- **Mismatch**: Update (re-apply)
- **Missing**: Create (first apply)

No API calls needed. Just local hash comparison.

## Transport

### Local Execution

For machines with `addr: 127.0.0.1` or `addr: localhost`:

```
bash <<< "generated script piped to stdin"
```

### SSH Execution

For all other addresses:

```
ssh -o BatchMode=yes -o ConnectTimeout=5 -o StrictHostKeyChecking=accept-new \
    [-i key_path] user@addr bash <<< "script piped to stdin"
```

Scripts are piped to `stdin` (not passed as arguments) to avoid:
- Argument length limits
- Shell metacharacter injection
- Command-line visibility in `ps`

## Atomic State

Lock files are written atomically:

1. Serialize to YAML
2. Write to `state/{machine}/state.lock.yaml.tmp`
3. `rename()` to `state/{machine}/state.lock.yaml`

On POSIX systems, `rename()` is atomic within the same filesystem. A crash during write leaves either the old lock or the new lock — never a corrupted file.

## Provenance Event Log

Every apply operation appends events to `state/{machine}/events.jsonl`:

```json
{"ts":"2026-02-16T14:00:00Z","event":"apply_started","machine":"gpu-box","run_id":"r-abc123","forjar_version":"0.1.0"}
{"ts":"2026-02-16T14:00:01Z","event":"resource_started","machine":"gpu-box","resource":"base-packages","action":"CREATE"}
{"ts":"2026-02-16T14:00:03Z","event":"resource_converged","machine":"gpu-box","resource":"base-packages","duration_seconds":2.1,"hash":"blake3:a7f2c9..."}
{"ts":"2026-02-16T14:00:03Z","event":"apply_completed","machine":"gpu-box","run_id":"r-abc123","resources_converged":1,"resources_unchanged":0,"resources_failed":0,"total_seconds":3.0}
```

Append-only. Never modified. Git-friendly.
