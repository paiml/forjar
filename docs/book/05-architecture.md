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
┌──────────┐
│ purifier │  bashrs validation + purification (FJ-036)
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
    purifier.rs          bashrs shell validation + purification (FJ-036)
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

### Worked Example

Given these resources:

```yaml
resources:
  packages:    { depends_on: [] }
  config:      { depends_on: [packages] }
  service:     { depends_on: [config] }
  firewall:    { depends_on: [] }
  monitoring:  { depends_on: [service, firewall] }
```

Step-by-step:

```
Initial in-degrees:  packages=0, config=1, service=1, firewall=0, monitoring=2
Heap (degree 0):     [firewall, packages]    ← alphabetical

Pop "firewall" → order: [firewall]
  monitoring: 2→1
Heap: [packages]

Pop "packages" → order: [firewall, packages]
  config: 1→0 → push to heap
Heap: [config]

Pop "config" → order: [firewall, packages, config]
  service: 1→0 → push to heap
Heap: [service]

Pop "service" → order: [firewall, packages, config, service]
  monitoring: 1→0 → push to heap
Heap: [monitoring]

Pop "monitoring" → order: [firewall, packages, config, service, monitoring]

Final: firewall → packages → config → service → monitoring
```

This is deterministic — the same config always produces the same order. Alphabetical tie-breaking means `firewall` runs before `packages` even though both have zero dependencies.

### Cycle Detection

If resources form a cycle (`A → B → C → A`), Kahn's algorithm detects this when the output length doesn't match the input count. The error message includes the cycle participants:

```
Error: dependency cycle detected involving: A, B, C
```

## Template Resolution

Templates use `{{...}}` syntax and are resolved before codegen:

| Template | Source | Example |
|----------|--------|---------|
| `{{params.X}}` | `params:` block in config | `{{params.env}}` → `production` |
| `{{secrets.X}}` | `FORJAR_SECRET_*` env vars | `{{secrets.db-pass}}` → `hunter2` |
| `{{machine.NAME.FIELD}}` | Machine properties | `{{machine.web.addr}}` → `10.0.0.5` |

Resolution is applied to all string fields — content, path, name, command, port, image, environment variables, and more. Templates that don't match any known pattern are passed through unchanged.

## BLAKE3 Content Addressing

Every resource's desired state is hashed to a BLAKE3 composite digest. All relevant fields are included:

```
hash = blake3(type + "\0" + state + "\0" + provider + "\0" + packages + "\0"
            + path + "\0" + content + "\0" + name + "\0" + owner + "\0"
            + mode + "\0" + image + "\0" + ports + "\0" + command + "\0"
            + schedule + "\0" + port + "\0" + protocol + ...)
```

Format: `"blake3:{64 hex chars}"` (71 characters total)

This hash is stored in the lock file. On the next `apply`, the planner computes the hash of the desired state and compares:
- **Match**: Skip (no-op)
- **Mismatch**: Update (re-apply)
- **Missing**: Create (first apply)

No API calls needed. Just local hash comparison. Changing any field — content, permissions, image tag, port number, cron schedule — produces a different hash and triggers an update.

## Shell Purification (FJ-036)

Every shell script forjar generates passes through the **bashrs** purification pipeline. This enforces Invariant I8: no raw shell execution.

### Three Safety Levels

| Level | Function | Purpose |
|-------|----------|---------|
| **Validate** | `purifier::validate_script()` | Lint-based check; fails on Error-severity diagnostics |
| **Lint** | `purifier::lint_script()` | Full diagnostic pass; returns all findings with severity |
| **Purify** | `purifier::purify_script()` | Parse → purify AST → reformat (strongest guarantee) |

### bashrs Integration Points

1. **`core/purifier.rs`** — Thin wrapper around `bashrs::linter`, `bashrs::validation`, `bashrs::bash_parser`, and `bashrs::bash_transpiler`
2. **`forjar lint`** — Runs bashrs linter on all generated scripts (check, apply, state_query) and reports SEC/DET/IDEM violations
3. **`examples/shell_purifier.rs`** — Demonstrates all three safety levels

### Diagnostic Categories

bashrs diagnostics follow ShellCheck conventions with additional categories:

| Prefix | Meaning | Example |
|--------|---------|---------|
| **SEC** | Security violation (injection, unquoted vars) | SEC002: Unquoted variable |
| **DET** | Non-determinism (date, random, pid) | DET001: Non-deterministic command |
| **IDEM** | Idempotency violation (creates without checking) | IDEM001: Non-idempotent operation |
| **SC** | ShellCheck-equivalent rules | SC2162: read without -r |

### Example: Validating Generated Scripts

```rust
use forjar::core::{codegen, purifier};

let script = codegen::check_script(&resource).unwrap();
match purifier::validate_script(&script) {
    Ok(()) => println!("Script is clean"),
    Err(e) => eprintln!("Lint errors: {e}"),
}

// Full purification (parse → purify → reformat)
let purified = purifier::purify_script(&script).unwrap();
```

### Known Patterns

The `$SUDO` privilege escalation idiom intentionally uses unquoted expansion:
```bash
SUDO=""
[ "$(id -u)" -ne 0 ] && SUDO="sudo"
$SUDO apt-get install -y curl    # $SUDO disappears when empty
```

This triggers SEC002 but is a safe, standard shell pattern. bashrs reports it as a known warning in `forjar lint` output.

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

## Error Handling Architecture

Forjar uses a layered error model — each layer validates its scope and returns descriptive errors to the layer above:

```
CLI Layer (cli/mod.rs)
  ├── parse_and_validate() → config or error string
  ├── apply() → Vec<ApplyResult> or error string
  └── user-facing error formatting + exit codes

Parser Layer (parser.rs)
  ├── YAML parse errors → "cannot read" / "invalid YAML"
  ├── Structural validation → Vec<ValidationError>
  └── Compound error formatting: "validation errors:\n  - error1\n  - error2"

Executor Layer (executor.rs)
  ├── Transport errors → "transport error: {details}"
  ├── Script failures → "exit code {N}: {stderr}"
  └── Jidoka (stop-on-first) vs continue-independent policy

State Layer (state.rs)
  ├── Read errors → "cannot read {path}: {io_error}"
  ├── YAML parse errors → "invalid lock file {path}: {parse_error}"
  └── Write errors → "cannot write" / "cannot rename"
```

### Validation Error Accumulation

The parser collects ALL validation errors before reporting, rather than stopping at the first error. This gives users a complete picture:

```bash
$ forjar validate -f broken.yaml
validation errors:
  - resource 'web-pkg' (package) has no packages
  - resource 'web-pkg' (package) has no provider
  - resource 'nginx-conf' references unknown machine 'web-server'
  - resource 'backup' (cron) schedule '0 2 *' must have exactly 5 fields
```

### Jidoka (Stop-on-First)

Named after the Toyota Production System principle of "stop and fix," the default failure policy halts execution on the first resource failure:

```
FailurePolicy::StopOnFirst  (default)
  → First resource fails → stop applying → preserve partial state
  → Event log records: resource_failed, then apply_completed

FailurePolicy::ContinueIndependent
  → First resource fails → continue with non-dependent resources
  → All independent resources still get applied
  → Final apply_completed shows converged + failed counts
```

This prevents cascading failures. A failed package install won't trigger service restarts or mount operations that depend on it.

## Testing Architecture

Forjar's test suite validates every layer independently and in integration:

### Test Categories

| Category | Count | Location | What It Tests |
|----------|-------|----------|---------------|
| Unit tests | ~700 | `#[cfg(test)]` in each module | Individual functions |
| Falsification tests | ~15 | `proptest!` blocks | Invariant properties with random input |
| Integration tests | ~50 | `executor.rs` tests | Full apply→drift→reapply cycles |
| Contract tests | 13 | `build.rs` binding verification | Compile-time invariants |
| Examples | 15 | `examples/*.rs` | Runnable API demonstrations |

### Script Safety Testing

Every resource handler generates three script types, and every script is tested:

```
check_script(resource)       → precondition verification
apply_script(resource)       → state convergence
state_query_script(resource) → live state query for drift

Tests verify:
  ✓ All scripts begin with set -euo pipefail
  ✓ SUDO detection: SUDO="" ; [ "$(id -u)" -ne 0 ] && SUDO="sudo"
  ✓ Heredoc quoting prevents variable expansion: <<'FORJAR_EOF'
  ✓ All resource fields appear in generated scripts
  ✓ Absent state produces cleanup commands
```

### Drift Detection Testing

The test suite includes end-to-end drift validation:

```
1. Apply a file resource (creates file on disk)
2. Verify detect_drift returns empty (no drift)
3. Tamper with the file content externally
4. Verify detect_drift finds the change
5. Force re-apply to fix the drift
6. Verify detect_drift returns empty again
```

This cycle runs in isolated temp directories and verifies the full BLAKE3 hashing pipeline from apply through drift detection.

## Security Model

### Script Injection Prevention

All generated scripts use defensive patterns to prevent injection:

```bash
# Heredoc with single quotes prevents variable expansion
cat <<'FORJAR_EOF' > /etc/config
user-provided content here — $VARS and $(commands) are literal
FORJAR_EOF

# Values are single-quoted in commands
chown 'user':'group' '/path/to/file'
systemctl start 'service-name'
```

Scripts are piped to `stdin` (not passed as arguments) to avoid:
- **Argument length limits** — scripts can be arbitrarily long
- **Shell metacharacter injection** — no interpretation of special chars in `ps` output
- **Command-line visibility** — secrets in scripts aren't visible in process listings

### Secret Handling

Secrets flow through environment variables, never stored in config:

```
FORJAR_SECRET_DB_PASSWORD → {{secrets.db-password}} → resolved at template time
```

The resolution happens in memory. Resolved values appear in generated scripts (which are piped to stdin), but never in:
- Config files on disk
- Lock files (only hashes are stored)
- Event logs (only event metadata)
- Command-line arguments

### Transport Security

| Transport | Authentication | Encryption |
|-----------|---------------|------------|
| Local | Process user | N/A (same machine) |
| SSH | Key-based (BatchMode=yes) | SSH tunnel |
| Container | Docker socket | N/A (same host) |

SSH connections use `StrictHostKeyChecking=accept-new` (accept on first connection, reject changes) and `ConnectTimeout=5` to prevent hanging on unreachable hosts.

## Concurrency Model

### Per-Machine Sequential Execution

Within a single machine, resources execute sequentially in dependency order. This is by design — resource operations are not thread-safe on the target machine (e.g., two concurrent `apt-get install` calls would conflict).

### Cross-Machine Parallelism

When `--parallel` is specified, machines execute concurrently. Each machine gets its own thread with an independent execution context:

```
Machine A: pkg → config → service  (sequential)
Machine B: pkg → config → service  (sequential, concurrent with A)
Machine C: user → ssh-keys         (sequential, concurrent with A and B)
```

### State Isolation

Each machine has its own:
- Lock file (`state/{machine}/state.lock.yaml`)
- Event log (`state/{machine}/events.jsonl`)
- Transport connection (separate SSH session per machine)

No shared mutable state between machines during apply.

## Kernel Isolation (FJ-040)

### Pepita Resource Architecture

The pepita resource type provides bare-metal kernel isolation using Linux primitives. Unlike Docker (which requires a container runtime), pepita generates shell scripts that directly interact with kernel interfaces:

```
forjar.yaml (type: pepita)
    │
    ▼
┌─────────┐
│ codegen  │  Generate isolation scripts
└────┬─────┘
     │
     ▼
┌────────────────────────────────────────┐
│ Kernel Interfaces                      │
│                                        │
│  cgroups v2 ─── memory.max, cpuset    │
│  overlayfs  ─── lowerdir/upperdir     │
│  netns      ─── ip netns add/exec     │
│  chroot     ─── mkdir + chroot(2)     │
│  seccomp    ─── syscall filtering     │
└────────────────────────────────────────┘
```

### Isolation Feature Matrix

| Feature | Kernel Interface | Apply Script | Teardown Script |
|---------|-----------------|-------------|-----------------|
| Memory limits | cgroups v2 `memory.max` | `echo <bytes> > cgroup/memory.max` | `rmdir cgroup/` |
| CPU binding | cgroups v2 `cpuset.cpus` | `echo <cpus> > cgroup/cpuset.cpus` | `rmdir cgroup/` |
| Filesystem | overlayfs | `mount -t overlay` | `umount <merged>` |
| Network | network namespaces | `ip netns add` | `ip netns del` |
| Filesystem root | chroot | `mkdir -p <dir>` | `rm -rf <dir>` |
| Syscall filter | seccomp-bpf | Informational flag | — |

### Design Decision: Shell Scripts vs pepita Crate

Forjar generates shell scripts for isolation (matching the architecture of all other resource types) rather than linking the pepita crate directly. This ensures:

1. **Uniform execution model** — all resources are shell scripts piped through transport
2. **Auditability** — generated scripts can be reviewed before apply
3. **Transport agnostic** — isolation scripts work over SSH, container exec, or local bash
4. **No runtime dependency** — the pepita crate is not required at apply time

The pepita crate's types (`JailerConfig`, `MicroVm`) inform the resource schema design but aren't linked as a runtime dependency.

## Extension Points

### Resource Type Registry

Adding a new resource type requires:

1. Create `src/resources/new_type.rs` with three functions:
   - `check_script(resource) → String`
   - `apply_script(resource) → String`
   - `state_query_script(resource) → String`

2. Add the type to `ResourceType` enum in `types.rs`

3. Add dispatch arms in `codegen.rs` for all three functions

4. Add validation rules in `parser.rs`

5. The contract system (`build.rs`) will flag missing dispatch arms at compile time

### Custom Transport

Adding a new transport follows the same pattern as container:

1. Create `src/transport/new_transport.rs` with `exec_script(machine, script) → Result`
2. Add dispatch in `transport/mod.rs`
3. Add validation in `parser.rs` for the new transport type

## Error Handling Strategy

### Error Accumulation

Validation collects ALL errors before reporting. This is critical for UX — users should see every problem at once, not play whack-a-mole:

```rust
// Parser validates all resources, collecting errors
let mut errors: Vec<String> = Vec::new();
for (id, resource) in &config.resources {
    if let Err(e) = validate_resource(id, resource) {
        errors.push(e);
    }
}
if !errors.is_empty() {
    return Err(format!("validation errors:\n  - {}", errors.join("\n  - ")));
}
```

### Error Propagation by Phase

| Phase | Error Behavior |
|-------|---------------|
| Parse | Fail immediately on invalid YAML syntax |
| Validate | Accumulate all errors, report together |
| Resolve | Fail on first unresolvable template (future: accumulate) |
| Plan | Pure computation, cannot fail (returns empty plan for unknown states) |
| Apply | Configurable via policy: `stop_on_first` or `continue_independent` |
| Drift | Accumulate all findings, report together |

### Failure Policy Deep Dive

**stop_on_first** (default, Jidoka-inspired):
```
Resource A → Converged
Resource B → Failed ← STOP HERE
Resource C → Skipped (never attempted)
```

Partial state is saved — A's lock entry is written, B is marked Failed, C has no entry.

**continue_independent**:
```
Resource A → Converged
Resource B → Failed
Resource C → Converged (C doesn't depend on B, so it continues)
Resource D → Skipped (D depends on B, which failed)
```

Only resources in the failed resource's dependency subtree are skipped. Independent branches continue executing.

## Data Flow

### Apply Data Flow

The complete data flow during `forjar apply`:

```
forjar.yaml
    │
    ▼
Parse (YAML → ForjarConfig)
    │
    ▼
Validate (structural checks, accumulate errors)
    │
    ▼
Expand Recipes (type: recipe → expanded resources)
    │
    ▼
Resolve Templates ({{params.X}}, {{secrets.X}}, {{machine.X.Y}})
    │
    ▼
Build DAG (Kahn's topological sort)
    │
    ▼
Plan (compare hash_desired vs lock_hash → Create/Update/NoOp/Destroy)
    │
    ▼
For each machine:
    │
    ├── Load lock (state/{machine}/state.lock.yaml)
    │
    ├── For each resource (in topo order):
    │   ├── Codegen: check_script → apply_script → state_query_script
    │   ├── Transport: exec_script(machine, script)
    │   ├── Hash: blake3(applied state)
    │   └── Record: lock entry + event log
    │
    ├── Save lock (atomic write)
    └── Update global lock
```

### State File Hierarchy

```
state/
├── forjar.lock.yaml          # Global lock: machine summaries
├── web-server/
│   ├── state.lock.yaml       # Per-machine: resource hashes + status
│   └── events.jsonl          # Append-only event log
├── db-server/
│   ├── state.lock.yaml
│   └── events.jsonl
└── cache-server/
    ├── state.lock.yaml
    └── events.jsonl
```

## Design Decisions

### Why Shell Scripts?

Forjar generates shell scripts rather than using a remote API or agent. This is a deliberate design choice:

1. **Zero dependencies on target** — Needs only `bash` and standard Unix utilities. No agent, no runtime, no package manager.
2. **Auditable** — Every action is a readable shell command. Run `forjar plan --show-scripts` to see exactly what will execute.
3. **Transportable** — Same script works over SSH, inside containers, or locally. The transport layer just pipes stdin.
4. **Debuggable** — If something fails, you can copy the script and run it manually on the target machine.

### Why BLAKE3?

BLAKE3 was chosen over SHA-256 for:
- **Speed**: 4-14x faster depending on hardware (SIMD-accelerated)
- **Streaming**: Built-in streaming support with constant memory
- **Deterministic**: No initialization vector variations
- **Modern**: Released 2020, security-audited, no known weaknesses

### Why YAML?

Despite YAML's complexity pitfalls, it was chosen because:
- Infrastructure engineers already know YAML from Kubernetes, Ansible, Docker Compose
- Multi-line strings (for `content` fields) are natural
- Comments are supported (unlike JSON)
- Mature Rust parsing ecosystem (serde_yaml_ng)

### Why Not HCL/Nix/TOML?

| Format | Why Not |
|--------|---------|
| HCL | Terraform lock-in perception; complex interpolation syntax |
| Nix | Steep learning curve; requires Nix toolchain |
| TOML | Poor multi-line string support; awkward for nested structures |
| JSON | No comments; verbose; poor ergonomics for human editing |

## Transport Layer

### Transport Abstraction

All three transports share a single interface:

```rust
pub fn exec_script(machine: &Machine, script: &str) -> Result<ScriptOutput, String>
```

The dispatch logic:
1. **Container** (`transport == "container"`): `docker exec -i <name> bash`
2. **Local** (`addr == "127.0.0.1"` or `"localhost"`): Direct `bash -c`
3. **SSH** (everything else): `ssh -o StrictHostKeyChecking=no user@addr bash`

### Script Piping Pattern

Every transport uses the same mechanism — pipe the script to bash's stdin:

```
echo "#!/bin/bash\nset -euo pipefail\n<script>" | bash
```

This is critical for:
- **Security**: No script files left on target machines
- **Atomicity**: Entire script executes in one process
- **Cleanup**: No artifacts to remove after execution

### Container Lifecycle

Container transport has an additional lifecycle:

```
ensure_container() → exec_script() → cleanup_container()
```

- **Ephemeral** (default): Container created before first resource, destroyed after all resources complete
- **Attached**: Container must already be running, not destroyed after

### SSH Multiplexing

For multi-resource machines, SSH connections are reused via `ControlMaster`:

```
ssh -o ControlMaster=auto -o ControlPath=/tmp/forjar-%h -o ControlPersist=60
```

This avoids the TCP+SSH handshake overhead per resource.

## Concurrency Model

### Sequential by Default

Resources are applied in topological order (from DAG) within each machine. Cross-machine parallelism is supported via the `parallel` policy:

```yaml
policy:
  parallel: true       # Apply machines in parallel
  failure: continue    # Don't stop on first failure
```

### Error Accumulation

When `failure: continue_independent` is set, forjar collects all errors and reports them together:

```
✗ web: 2 failed, 3 converged
  - nginx-conf: permission denied
  - ssl-cert: file not found
✓ db: 5 converged
```

The executor tracks which resources depend on failed ones and skips them transitively.

## Contract System

### Compile-Time Verification

Forjar uses `provable_contracts_macros` to verify bindings at compile time:

```rust
#[contract("dag-ordering-v1", equation = "topological_sort")]
pub fn build_execution_order(config: &ForjarConfig) -> Result<Vec<String>, String> { ... }
```

The `build.rs` script verifies all 13 contract bindings exist and are correctly annotated. This ensures:
- Critical algorithms (DAG sort, hash computation, script generation) are marked
- Refactoring doesn't accidentally remove or rename contracted functions
- CI catches contract violations before deployment

### Current Contracts

| Contract | Equation | Function |
|----------|----------|----------|
| dag-ordering-v1 | topological_sort | `build_execution_order` |
| state-lock-v1 | atomic_update | `update_lock` |
| hash-desired-v1 | composite_hash | `hash_desired_state` |
| hash-file-v1 | blake3_file | `hash_file` |
| codegen-v1 | check_script | `check_script` |
| codegen-v1 | apply_script | `apply_script` |
| codegen-v1 | state_query | `state_query_script` |
| drift-v1 | detect_drift | `detect_drift` |
| exec-v1 | apply_machine | `apply_machine` |
| parse-v1 | validate | `validate_config` |
| transport-v1 | exec_script | `exec_script` |
| eventlog-v1 | append_event | `append_event` |
| plan-v1 | plan | `plan` |
| mcp-v1 | forjar_validate | `ValidateHandler` |
| mcp-v1 | forjar_plan | `PlanHandler` |
| mcp-v1 | forjar_drift | `DriftHandler` |

## MCP Integration (FJ-063)

Forjar exposes its operations as MCP (Model Context Protocol) tools via the
pforge framework. This enables AI agents and LLM-powered tools to manage
infrastructure through the same validated pipeline as the CLI.

### Architecture

```
┌─────────────────────────────────┐
│  AI Agent (Claude, etc.)        │
│  MCP Client                     │
└──────────┬──────────────────────┘
           │ MCP Protocol (stdio)
           ▼
┌─────────────────────────────────┐
│  pforge McpServer               │
│  └── pmcp protocol handler      │
│  └── HandlerRegistry (O(1))     │
└──────────┬──────────────────────┘
           │ dispatch(tool, params)
           ▼
┌─────────────────────────────────┐
│  forjar MCP Handlers            │
│  ├── ValidateHandler            │
│  ├── PlanHandler                │
│  ├── DriftHandler               │
│  ├── LintHandler                │
│  ├── GraphHandler               │
│  ├── ShowHandler                │
│  └── StatusHandler              │
└──────────┬──────────────────────┘
           │ calls forjar core
           ▼
┌─────────────────────────────────┐
│  parser → resolver → planner    │
│  → codegen → executor           │
└─────────────────────────────────┘
```

### Tool Registry

Each handler implements the pforge `Handler` trait with typed input/output:

```rust
#[async_trait]
impl Handler for ValidateHandler {
    type Input = ValidateInput;    // { path: String }
    type Output = ValidateOutput;  // { valid, resource_count, errors }
    type Error = pforge_runtime::Error;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output> {
        let config = parser::parse_and_validate(&PathBuf::from(&input.path))?;
        Ok(ValidateOutput { valid: true, ... })
    }
}
```

JSON Schema is auto-generated from the Rust types via `schemars`, enabling
MCP clients to discover tool parameters without documentation.

### Available Tools

| Tool | Description | Input |
|------|-------------|-------|
| `forjar_validate` | Validate forjar.yaml | `{ path }` |
| `forjar_plan` | Show execution plan | `{ path, state_dir?, resource?, tag? }` |
| `forjar_drift` | Detect configuration drift | `{ path, state_dir?, machine? }` |
| `forjar_lint` | Lint config + shell safety | `{ path }` |
| `forjar_graph` | Generate dependency graph | `{ path, format? }` |
| `forjar_show` | Show resolved config | `{ path, resource? }` |
| `forjar_status` | Show state from locks | `{ state_dir?, machine? }` |

### Starting the MCP Server

```bash
forjar mcp
```

This starts a stdio MCP server using pforge's McpServer. Configure in
your MCP client (e.g., Claude Desktop, VS Code) with:

```json
{
  "mcpServers": {
    "forjar": {
      "command": "forjar",
      "args": ["mcp"]
    }
  }
}
```
