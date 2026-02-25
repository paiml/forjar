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
│ executor │  Run scripts via local bash, SSH, or container exec
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
  build.rs               Compile-time contract binding verification
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
    package.rs           apt/cargo/uv package management
    file.rs              File, directory, symlink, absent
    service.rs           systemd service management
    mount.rs             NFS/bind mount management
    user.rs              User/group management (useradd/usermod)
    docker.rs            Docker container lifecycle
    cron.rs              Crontab scheduled tasks
    network.rs           Firewall rules (ufw)
  transport/
    mod.rs               Transport dispatch (container > local > SSH)
    local.rs             Local bash execution
    ssh.rs               SSH execution (stdin pipe, no libssh2)
    container.rs         Container execution (docker/podman exec -i)
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

All three transports share the same mechanism: pipe a shell script to bash stdin, capture stdout/stderr/exit_code. Transport selection is automatic based on machine configuration.

### Selection Priority

| Priority | Condition | Transport |
|----------|-----------|-----------|
| 1 | `transport: container` or `addr: container` | Container exec |
| 2 | `addr: 127.0.0.1` or `addr: localhost` | Local bash |
| 3 | Any other address | SSH |

### Local Execution

For machines with `addr: 127.0.0.1` or `addr: localhost`:

```
bash <<< "generated script piped to stdin"
```

### SSH Execution

For remote machines:

```
ssh -o BatchMode=yes -o ConnectTimeout=5 -o StrictHostKeyChecking=accept-new \
    [-i key_path] user@addr bash <<< "script piped to stdin"
```

### Container Execution

For container machines (`transport: container`):

```
docker exec -i <container-name> bash <<< "script piped to stdin"
```

The executor manages container lifecycle automatically:
1. `ensure_container` — inspect, create if needed (`docker run -d --name <name> --init <image> sleep infinity`)
2. `exec_container` — pipe script to `docker exec -i <name> bash`
3. `cleanup_container` — `docker rm -f <name>` (ephemeral mode only, runs even on failure)

Scripts are piped to `stdin` (not passed as arguments) to avoid:
- Argument length limits
- Shell metacharacter injection
- Command-line visibility in `ps`

### Container Lifecycle

The executor manages the full container lifecycle during `apply`:

```
apply_machine("test-box")
    │
    ├── ensure_container()
    │   ├── docker inspect -f '{{.State.Running}}' forjar-test-box
    │   │   ├── "true" → container already running, skip creation
    │   │   └── failure/false → create new container
    │   │       └── docker run -d --name forjar-test-box [--init] [--privileged] ubuntu:22.04 sleep infinity
    │   └── return Ok(())
    │
    ├── for resource in execution_order:
    │   └── exec_container()
    │       └── docker exec -i forjar-test-box bash <<< "check/apply script"
    │
    └── cleanup_container()  (ephemeral only, runs even on failure)
        └── docker rm -f forjar-test-box
```

**Ephemeral containers** (`ephemeral: true`, the default) are created fresh for each apply run and destroyed afterward. This guarantees a clean environment for CI/CD testing.

**Attached containers** (`ephemeral: false`) persist between applies. The executor verifies the container is running but does not destroy it. Use this for long-lived dev environments.

**Container naming**: If `container.name` is set, that name is used directly. Otherwise, the name is derived as `forjar-{machine-key}` (e.g., machine key `test-box` becomes `forjar-test-box`).

**Runtime selection**: Set `container.runtime` to `docker` (default) or `podman`. The runtime binary is used for all lifecycle operations (inspect, run, exec, rm).

**Flags**:
- `--init` (default: true) — adds a PID 1 init process for proper signal handling and zombie reaping
- `--privileged` (default: false) — grants full host capabilities (needed for systemd-in-container testing)

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

### Event Types

| Event | Fields | Description |
|-------|--------|-------------|
| `apply_started` | `machine`, `run_id`, `forjar_version` | Apply run begins |
| `resource_started` | `machine`, `resource`, `action` | Resource execution begins |
| `resource_converged` | `machine`, `resource`, `duration_seconds`, `hash` | Resource applied successfully |
| `resource_unchanged` | `machine`, `resource`, `hash` | Resource already at desired state |
| `resource_failed` | `machine`, `resource`, `error` | Resource execution failed |
| `apply_completed` | `machine`, `run_id`, `resources_converged`, `resources_unchanged`, `resources_failed`, `total_seconds` | Apply run ends |

Every event has a `ts` field (ISO 8601 UTC timestamp, e.g., `2026-02-25T14:30:00Z`).

### Querying Events

```bash
# Last 20 events
forjar history -n 20

# JSON output for dashboards
forjar history --json | jq '.events[] | {ts: .ts, event: .event}'

# Anomaly detection (z-score on resource churn)
forjar anomaly --state-dir state --json | jq '.anomalies'
```

## Provable Contracts

Forjar integrates with the `provable-contracts` framework for formal invariant verification. Ten core functions are annotated with `#[contract]` attributes that bind them to YAML contract equations.

### Verification Layers

| Layer | Mechanism | When |
|-------|-----------|------|
| Compile-time | `build.rs` verifies all 13 bindings | Every `cargo build` |
| Falsification | 15 proptest-based tests | Every `cargo test` |
| Model checking | Kani harnesses (Phase 2) | `cargo kani` |

### Contract Coverage

| Contract | Invariants | Functions |
|----------|-----------|-----------|
| `blake3-state-v1` | I3: Content addressing | `hash_string`, `hash_file`, `composite_hash` |
| `dag-ordering-v1` | I5: Topological sort | `build_execution_order` |
| `execution-safety-v1` | I4, I7: Atomicity + Jidoka | `save_lock` |
| `recipe-determinism-v1` | I11, I12: Expansion + validation | `validate_inputs`, `expand_recipe` |
| `codegen-dispatch-v1` | I2: Dispatch completeness | `check_script`, `apply_script`, `state_query_script` |

### Annotation Example

```rust
use provable_contracts_macros::contract;

#[contract("blake3-state-v1", equation = "hash_string")]
pub fn hash_string(input: &str) -> String {
    let hash = blake3::hash(input.as_bytes());
    format!("blake3:{}", hash.to_hex())
}
```

The `build.rs` reads `binding.yaml` and sets `CONTRACT_*` env vars consumed by the proc macro at compile time. Missing bindings produce compile warnings.
