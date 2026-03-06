# State Management

Forjar uses a file-based state system to track what has been applied to each machine. This enables idempotent applies, drift detection, and rollback.

## State Directory Layout

```
state/
  forjar.lock.yaml            # Global lock — summary of all machines
  intel/
    state.lock.yaml            # Per-machine lock — resource hashes and status
    events.jsonl               # Provenance event log — append-only audit trail
    trace.jsonl                # W3C trace spans from apply (FJ-050)
  web-server/
    state.lock.yaml
    events.jsonl
    trace.jsonl
```

Each machine gets its own subdirectory. The state directory defaults to `state/` but can be overridden with `--state-dir`.

## Lock Files

### Global Lock (`forjar.lock.yaml`)

Summarizes the most recent apply across all machines:

```yaml
schema: '1.0'
name: my-infrastructure
last_apply: 2026-02-25T14:00:00Z
generator: forjar 0.1.0
machines:
  intel:
    resources: 16
    converged: 16
    failed: 0
    last_apply: 2026-02-25T14:00:00Z
  web-server:
    resources: 8
    converged: 7
    failed: 1
    last_apply: 2026-02-25T14:00:05Z
```

### Per-Machine Lock (`state/{machine}/state.lock.yaml`)

Records the full state of every managed resource on a machine:

```yaml
schema: '1.0'
machine: intel
hostname: mac-server
generated_at: 2026-02-16T16:44:39Z
generator: forjar 0.1.0
blake3_version: '1.8'
resources:
  bash-aliases:
    type: file
    status: converged
    applied_at: 2026-02-16T16:32:55Z
    duration_seconds: 0.54
    hash: blake3:43b33ddd15c866b0d54f2144e8b66d96b88082...
    details:
      path: /home/noah/.bash_aliases
      content_hash: blake3:aae3de54118cd67a57432145e96802...
      live_hash: blake3:22035c315c17dcc46d45a57a0b97d003...
      mode: '0644'
      owner: noah
  cargo-tools:
    type: package
    status: converged
    applied_at: 2026-02-16T16:32:57Z
    duration_seconds: 0.85
    hash: blake3:c5fc7e8c095d8dc1ea5c245829bdab6fd0c4...
    details:
      live_hash: blake3:18a03cb3b066ae1b7f8e9a803a51b196...
```

### Resource Lock Fields

| Field | Description |
|-------|-------------|
| `type` | Resource type (file, package, service, mount, etc.) |
| `status` | `converged`, `failed`, or `drifted` |
| `applied_at` | ISO 8601 timestamp of last apply |
| `duration_seconds` | How long the apply took |
| `hash` | BLAKE3 hash of the resource's desired configuration |
| `details.path` | File path (file resources) |
| `details.content_hash` | BLAKE3 hash of file content on disk |
| `details.live_hash` | BLAKE3 hash of state query output (for drift comparison) |
| `details.mode` | File permissions |
| `details.owner` | File owner |

## BLAKE3 Content Hashing

Forjar uses BLAKE3 for all content hashing. BLAKE3 is:

- **Fast**: 4x faster than SHA-256, SIMD-accelerated
- **Deterministic**: Same content always produces the same hash
- **Collision-resistant**: 256-bit output, cryptographically secure

Hashes are stored with a `blake3:` prefix for clarity:

```
blake3:43b33ddd15c866b0d54f2144e8b66d96b88082178f02052b914f1d2fbeb08060
```

### What Gets Hashed

| Resource Type | Hash Source |
|---------------|-------------|
| File | File content on disk (`content_hash`) |
| Directory | Recursive directory listing |
| Package | Output of package query command |
| Service | `systemctl is-active` + `systemctl is-enabled` |
| Mount | `mountpoint` + `findmnt` output |
| User | `/etc/passwd` entry + group membership |
| Docker | `docker inspect` output |
| Cron | Crontab entry for user |
| Network | `ufw status` output |

## Drift Detection

Drift detection compares the current live state against the recorded lock state.

### File Resources

For file resources, forjar re-hashes the file on disk and compares the BLAKE3 hash against `details.content_hash`:

```bash
# Check for drift
forjar drift -f forjar.yaml

# Machine-specific
forjar drift -f forjar.yaml -m web-server

# CI mode — exit non-zero on any drift
forjar drift -f forjar.yaml --tripwire

# Auto-fix drift
forjar drift -f forjar.yaml --auto-remediate
```

### Non-File Resources

For packages, services, mounts, and other resource types, forjar re-runs the resource's `state_query_script` via transport and compares the BLAKE3 hash of the output against `details.live_hash`.

## Event Log

The event log (`events.jsonl`) is an append-only audit trail recording every operation. Each line is a JSON object:

```json
{"ts":"2026-02-16T16:32:54Z","event":"apply_started","machine":"intel","run_id":"r-c732cf4bbc73","forjar_version":"0.1.0"}
{"ts":"2026-02-16T16:32:54Z","event":"resource_started","machine":"intel","resource":"bash-aliases","action":"CREATE"}
{"ts":"2026-02-16T16:32:55Z","event":"resource_converged","machine":"intel","resource":"bash-aliases","duration_seconds":0.54,"hash":"blake3:43b33..."}
{"ts":"2026-02-16T16:44:39Z","event":"apply_completed","machine":"intel","run_id":"r-c732cf4bbc73","resources_converged":16,"resources_unchanged":0,"resources_failed":0,"total_seconds":10.5}
```

### Event Types

| Event | Description |
|-------|-------------|
| `apply_started` | Begin of an apply run (includes run_id, forjar_version) |
| `resource_started` | Resource apply begins (includes action: CREATE/UPDATE) |
| `resource_converged` | Resource successfully applied (includes duration, hash) |
| `resource_failed` | Resource apply failed (includes error message) |
| `apply_completed` | End of apply run (includes summary counts) |
| `drift_detected` | Drift found (includes expected/actual hash) |

### Querying History

```bash
# View recent apply history
forjar history

# Last 20 events
forjar history -n 20

# JSON output for scripting
forjar history --json

# Detect anomalies in event patterns
forjar anomaly --min-events 1
```

## Composite Hashing

Forjar uses composite hashing to create a single fingerprint for each resource's desired configuration. The composite hash combines the resource type with all relevant fields:

```
composite_hash("file", path, content, owner, group, mode) → blake3:a1b2c3...
```

This means the hash changes when **any** field changes — not just content. For example, changing a file's `mode` from `0644` to `0755` produces a different composite hash even though the file content is identical.

### Hash Stability

Hash comparison drives the planner's decision logic:

| Previous Hash | Current Hash | Action |
|---------------|--------------|--------|
| None (new) | blake3:abc... | CREATE |
| blake3:abc... | blake3:abc... | SKIP (converged) |
| blake3:abc... | blake3:xyz... | UPDATE |

This prevents unnecessary re-applies. If you re-run `forjar apply` with no config changes, every resource is skipped because the composite hashes match.

### Hashing by Resource Type

Each resource type contributes different fields to its composite hash:

```
File:    hash(type, path, content, source, owner, group, mode)
Package: hash(type, provider, packages, version)
Service: hash(type, name, state, enabled, restart_on)
Mount:   hash(type, path, source, fs_type, options, state)
User:    hash(type, name, uid, shell, home, groups, ssh_keys)
Docker:  hash(type, name, image, ports, env, volumes, restart)
Cron:    hash(type, name, schedule, command, owner)
Network: hash(type, port, protocol, action, from_addr)
```

## Bashrs Script Validation in State Pipeline

Forjar enforces shell safety through bashrs purification (FJ-036). Every shell script that forjar generates passes through a validation pipeline before execution. This validation is part of the state management lifecycle because the scripts that produce state transitions must themselves be correct.

### Where Validation Fits

The apply pipeline follows this sequence:

```
1. Config parse     → forjar.yaml → Resource structs
2. Codegen          → Resource → shell scripts (check, apply, state_query)
3. Script validation → purifier::validate_script() — lint for errors
4. Transport exec   → run script on target machine via local or SSH
5. Hash & store     → BLAKE3 hash output, write to state.lock.yaml
```

Step 3 is the bashrs gate. If a generated script contains Error-severity lint diagnostics, the apply is aborted before any changes reach the target machine. Warning-severity diagnostics are permitted because generated scripts may use patterns that trigger informational warnings (for example, `read` without `-r`).

### Validation Levels

The purifier module (`src/core/purifier.rs`) provides three levels of shell safety:

| Function | Behavior | Use Case |
|----------|----------|----------|
| `validate_script(script)` | Lint-only, fails on Error severity | Pre-execution gate in apply pipeline |
| `lint_script(script)` | Full lint pass, returns all diagnostics | Diagnostic reporting, CI checks |
| `purify_script(script)` | Parse, purify AST, reformat, validate | Strongest guarantee, injection prevention |

### Code Example

The following shows how codegen output flows through validation before execution and state capture:

```rust
use forjar::core::{codegen, purifier};
use forjar::core::types::Resource;

fn apply_resource_with_validation(resource: &Resource) -> Result<String, String> {
    // Step 1: Generate the apply script from the resource definition
    let script = codegen::apply_script(resource)?;

    // Step 2: Validate through bashrs linter (errors only)
    purifier::validate_script(&script)?;

    // Step 3: Execute the validated script via transport
    let output = transport::execute(&script)?;

    // Step 4: Hash the output for state storage
    let hash = blake3_hash(&output);

    // The hash is stored in state.lock.yaml as the resource's live_hash
    Ok(hash)
}
```

For the strongest safety guarantee, use `purify_script()` which parses the shell into an AST, applies purification transforms (proper quoting, injection prevention, deterministic ordering), reformats the AST back to shell, and validates the result:

```rust
// Full purification pipeline: parse → purify AST → reformat → validate
let purified = purifier::purify_script(&script)?;
// purified is now safe to execute — injection vectors removed
```

### State Query Scripts

State query scripts are also validated before execution. These scripts capture the live state of a resource (for example, `dpkg -l curl` for a package or `systemctl is-active nginx` for a service). The BLAKE3 hash of the state query output becomes the `live_hash` stored in the lock file. If the state query script itself were malformed, the captured hash would be meaningless.

```
codegen::state_query_script(resource)
  → purifier::validate_script(script)
  → transport::execute(script)
  → blake3::hash(output)
  → store as details.live_hash in state.lock.yaml
```

This ensures that every hash recorded in the state lock was produced by a validated script, maintaining the integrity of drift detection.

## Atomic Writes

Lock files are written atomically using a write-then-rename pattern:

1. Write to `state.lock.yaml.tmp`
2. Rename to `state.lock.yaml`

This prevents partial writes from corrupting state if forjar is interrupted. The rename is atomic on all POSIX filesystems — either the old or new state is visible, never a partial write.

## State Inspection

### Show Command

View the current state of a specific resource or machine:

```bash
# Show all resources on a machine
forjar show -f forjar.yaml -m intel

# JSON output for scripting
forjar show -f forjar.yaml --json

# Filter by resource
forjar show -f forjar.yaml -r bash-aliases
```

### Status Command

Quick summary of all machines:

```bash
forjar status -f forjar.yaml --state-dir state
```

Output shows per-machine counts: converged, failed, and drifted resources.

### Diff Command

Compare two state directories to see what changed between applies:

```bash
# Compare current state with a backup
forjar diff state/ state-backup/

# Compare before and after an apply
cp -r state/ /tmp/state-before/
forjar apply -f forjar.yaml
forjar diff /tmp/state-before/ state/
```

## Recovery

### Partial Apply Failure

If `forjar apply` fails midway (jidoka — stop on first failure):

1. Successfully converged resources are recorded in the lock with `status: converged`
2. The failed resource is recorded with `status: failed` and error details
3. Remaining resources are not attempted (no cascading damage)

Re-running `forjar apply` will:
- **Skip** converged resources (hash matches)
- **Retry** the failed resource
- **Continue** with remaining resources

```bash
# View what failed
forjar status -f forjar.yaml

# Retry
forjar apply -f forjar.yaml

# Force re-apply everything (including converged)
forjar apply -f forjar.yaml --force
```

### Corrupted Lock File

If a lock file becomes corrupted or desynchronized:

```bash
# Option 1: Delete and rebuild from scratch
rm state/{machine}/state.lock.yaml
forjar apply -f forjar.yaml --force

# Option 2: Delete a single machine's state
rm -rf state/web-server/
forjar apply -f forjar.yaml -m web-server

# Option 3: Import current live state
forjar import -f forjar.yaml -m web-server --state-dir state
```

### Selective Force Apply

Force re-apply specific resources without touching others:

```bash
# Re-apply only tagged resources
forjar apply -f forjar.yaml --force --tag web

# Re-apply a single resource
forjar apply -f forjar.yaml --force -r nginx-config
```

### Rollback

To revert to a previous configuration:

```bash
# Preview what would change
forjar rollback --dry-run

# Rollback to previous version
forjar rollback

# Rollback 3 versions back
forjar rollback -n 3
```

Rollback reads the previous `forjar.yaml` from git history and re-applies it with `--force`.

## Git Integration

State files are designed to be committed to git:

```bash
# After a successful apply
forjar apply -f forjar.yaml --auto-commit

# Or manually
git add state/
git commit -m "forjar: apply 2026-02-25"
```

This enables:
- **Audit trail**: Git history shows every state change
- **Rollback**: `forjar rollback` reads previous configs from git
- **Diff**: `forjar diff state-v1/ state-v2/` compares state snapshots
- **Team collaboration**: State is shared via the repository

### State in Monorepos

For teams managing multiple environments from one repo:

```
infra/
  forjar.yaml            # Production config
  forjar-staging.yaml    # Staging config
  state/
    prod-web/            # Production state
    staging-web/         # Staging state
```

Each environment uses a separate `--state-dir` or separate machine names. State files never conflict because they're keyed by machine name.

### State Cleanup

Over time, event logs grow. To manage size:

```bash
# Check event log sizes
du -sh state/*/events.jsonl

# Archive old events (keep last 1000 lines per machine)
for f in state/*/events.jsonl; do
  tail -1000 "$f" > "$f.tmp" && mv "$f.tmp" "$f"
done
```

Lock files are small (typically < 10KB) and do not grow over time — they represent current state only.

## State File Internals

### Lock File Schema

Every lock file follows this schema:

```yaml
schema: '1.0'                    # Lock file format version
machine: web-server              # Machine key from config
hostname: web1                   # Machine hostname
generated_at: 2026-02-25T14:00:00Z  # ISO 8601 UTC timestamp
generator: forjar 0.1.0         # Generator string
blake3_version: '1.8'           # BLAKE3 library version
resources:                       # Map of resource_id → ResourceLock
  resource-name:
    type: file                   # Resource type
    status: converged            # converged | failed | drifted | unknown
    applied_at: 2026-02-25T14:00:01Z
    duration_seconds: 0.54
    hash: blake3:...             # Composite hash of desired state
    details:                     # Type-specific metadata
      path: /etc/nginx/nginx.conf
      content_hash: blake3:...   # Hash of actual file contents
      live_hash: blake3:...      # Hash of state_query output
```

### State Lock Schema Reference

The following tables document every field in the `StateLock` and `ResourceLock` Rust structs (defined in `src/core/types.rs`). These structs are serialized directly to YAML for the per-machine lock files.

#### StateLock Fields

| Field | Rust Type | YAML Key | Default | Description |
|-------|-----------|----------|---------|-------------|
| `schema` | `String` | `schema` | (required) | Lock file format version, currently `"1.0"` |
| `machine` | `String` | `machine` | (required) | Machine key from the forjar.yaml config |
| `hostname` | `String` | `hostname` | (required) | Machine hostname as declared in config |
| `generated_at` | `String` | `generated_at` | (required) | ISO 8601 UTC timestamp of lock generation |
| `generator` | `String` | `generator` | (required) | Generator string, e.g. `"forjar 0.1.0"` |
| `blake3_version` | `String` | `blake3_version` | (required) | BLAKE3 library version used for hashing, e.g. `"1.8"` |
| `resources` | `IndexMap<String, ResourceLock>` | `resources` | (required) | Ordered map of resource ID to resource lock entry |

#### ResourceLock Fields

| Field | Rust Type | YAML Key | Default | Description |
|-------|-----------|----------|---------|-------------|
| `resource_type` | `ResourceType` | `type` | (required) | Resource type enum: `file`, `package`, `service`, `mount`, `user`, `docker`, `cron`, `network` |
| `status` | `ResourceStatus` | `status` | (required) | Convergence status: `converged`, `failed`, `drifted`, or `unknown` |
| `applied_at` | `Option<String>` | `applied_at` | `null` | ISO 8601 timestamp of last apply, absent if never applied |
| `duration_seconds` | `Option<f64>` | `duration_seconds` | `null` | Wall-clock duration of last apply in seconds |
| `hash` | `String` | `hash` | (required) | BLAKE3 composite hash of the resource's desired state |
| `details` | `HashMap<String, Value>` | `details` | `{}` | Resource-type-specific metadata (path, content_hash, live_hash, etc.) |

#### GlobalLock Fields

| Field | Rust Type | YAML Key | Default | Description |
|-------|-----------|----------|---------|-------------|
| `schema` | `String` | `schema` | (required) | Lock file format version, currently `"1.0"` |
| `name` | `String` | `name` | (required) | Infrastructure name from forjar.yaml |
| `last_apply` | `String` | `last_apply` | (required) | ISO 8601 timestamp of the most recent apply |
| `generator` | `String` | `generator` | (required) | Generator string, e.g. `"forjar 0.1.0"` |
| `machines` | `IndexMap<String, MachineSummary>` | `machines` | (required) | Ordered map of machine name to summary |

#### MachineSummary Fields

| Field | Rust Type | YAML Key | Default | Description |
|-------|-----------|----------|---------|-------------|
| `resources` | `usize` | `resources` | (required) | Total number of managed resources |
| `converged` | `usize` | `converged` | (required) | Number of resources successfully applied |
| `failed` | `usize` | `failed` | (required) | Number of resources that failed to apply |
| `last_apply` | `String` | `last_apply` | (required) | ISO 8601 timestamp of the machine's last apply |

#### ResourceStatus Enum

| Value | Rust Variant | Display | Description |
|-------|-------------|---------|-------------|
| `converged` | `ResourceStatus::Converged` | `CONVERGED` | Resource matches desired state |
| `failed` | `ResourceStatus::Failed` | `FAILED` | Last apply attempt failed |
| `drifted` | `ResourceStatus::Drifted` | `DRIFTED` | Live state differs from recorded state |
| `unknown` | `ResourceStatus::Unknown` | `UNKNOWN` | No prior state recorded |

### Status Lifecycle

Resources transition through these statuses:

```
                  ┌──────────┐
    first apply → │ converged │ ← successful re-apply
                  └─────┬─────┘
                        │
                  drift detected
                        │
                  ┌─────▼─────┐
                  │  drifted   │
                  └─────┬─────┘
                        │
                  re-apply (--force)
                        │
                  ┌─────▼─────┐
                  │ converged  │
                  └────────────┘

    apply failure → ┌──────┐
                    │ failed│ → retry → converged
                    └──────┘

    no prior state → ┌─────────┐
                     │ unknown  │ → first apply → converged
                     └──────────┘
```

### Details by Resource Type

Each resource type stores different metadata in the `details` map:

| Resource Type | Details Fields |
|---------------|---------------|
| **File** | `path`, `content_hash`, `live_hash`, `owner`, `group`, `mode` |
| **Package** | `live_hash` |
| **Service** | `service_name`, `live_hash` |
| **Mount** | `mount_path`, `live_hash` |
| **User** | `username`, `live_hash` |
| **Docker** | `container_name`, `live_hash` |
| **Cron** | `cron_name`, `live_hash` |
| **Network** | `live_hash` |

The `live_hash` is the BLAKE3 hash of the `state_query_script` output at apply time. During drift detection, forjar re-runs the state query and compares the new output hash against `live_hash`.

### Global Lock Schema

The global lock (`forjar.lock.yaml`) aggregates per-machine summaries:

```yaml
schema: '1.0'
name: my-infrastructure          # Config name
last_apply: 2026-02-25T14:00:00Z
generator: forjar 0.1.0
machines:
  web-server:
    resources: 8                 # Total resource count
    converged: 7                 # Successfully applied
    failed: 1                    # Failed to apply
    last_apply: 2026-02-25T14:00:05Z
```

## State Consistency Guarantees

### Atomic Write Protocol

Forjar ensures lock files are never corrupted, even during crashes:

```
1. Serialize ResourceLock → YAML string
2. Write to state.lock.yaml.tmp (temp file)
3. fsync() to flush to disk
4. rename() temp → state.lock.yaml (atomic on POSIX)
```

If forjar crashes between steps 2 and 4, the temp file remains and the original lock is untouched. On the next apply, forjar reads the intact original lock.

### Partial Apply State

When jidoka stops execution after a failure:

```
Resources:  A(ok) → B(ok) → C(FAIL) → D(skipped) → E(skipped)

Lock file after partial apply:
  A: status: converged, hash: blake3:...
  B: status: converged, hash: blake3:...
  C: status: failed, hash: blake3:...
  D: (not present — never attempted)
  E: (not present — never attempted)
```

On the next `forjar apply`:
- A and B are **skipped** (hashes match)
- C is **retried** (status is failed)
- D and E are **attempted** for the first time

This means partial applies are always safe to re-run.

### Event Log Durability

The event log (`events.jsonl`) is append-only:

```
1. Serialize event → JSON string
2. Open events.jsonl with O_APPEND
3. Write JSON line + newline
4. Close file
```

`O_APPEND` guarantees atomic appends on POSIX — even concurrent writers produce valid JSONL. Events are never modified or deleted by forjar.

## Concurrent Access Protection

Forjar's state files are designed to be safe under concurrent access, even though forjar itself does not use file locking. Safety comes from two mechanisms: atomic writes for lock files and append-only semantics for event logs.

### Lock File Atomicity

The `state::save_lock()` function (in `src/core/state.rs`) writes lock files using a temp-file-and-rename pattern:

```rust
// Simplified from src/core/state.rs
pub fn save_lock(state_dir: &Path, lock: &StateLock) -> Result<(), String> {
    let path = lock_file_path(state_dir, &lock.machine);
    let yaml = serde_yaml_ng::to_string(lock)?;

    // Write to temporary file first
    let tmp_path = path.with_extension("lock.yaml.tmp");
    std::fs::write(&tmp_path, &yaml)?;

    // Atomic rename — either old or new content is visible, never partial
    std::fs::rename(&tmp_path, &path)?;
    Ok(())
}
```

The `rename()` system call is atomic on all POSIX filesystems. This means that any process reading the lock file will see either the complete old version or the complete new version. There is no window where a partial write is visible.

If two forjar processes write to the same machine's lock file simultaneously, the last rename wins. The result is always a valid, complete lock file. No corruption occurs because neither process can produce a partial write through rename.

### Event Log Append Safety

The event log uses `OpenOptions::new().append(true)` which maps to the POSIX `O_APPEND` flag:

```rust
// Simplified from src/tripwire/eventlog.rs
let mut file = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(&path)?;
use std::io::Write;
writeln!(file, "{}", json)?;
```

The `O_APPEND` flag guarantees that each write positions the file offset at the end of the file atomically before writing. This means concurrent appenders produce interleaved but never overlapping writes. Each JSON line remains intact.

### Concurrent Machine Safety

Each machine has its own subdirectory under the state directory:

```
state/
  machine-a/state.lock.yaml    # Only written by machine-a apply
  machine-a/events.jsonl        # Only appended by machine-a events
  machine-b/state.lock.yaml    # Independent — no conflict with machine-a
  machine-b/events.jsonl
```

Because lock files are keyed by machine name and stored in separate directories, applying to different machines concurrently (for example, with `policy.parallel_machines: true`) never causes cross-machine file conflicts. Each machine's state is fully isolated.

### What Is Not Protected

Forjar does not implement advisory or mandatory file locking. Running two `forjar apply` commands against the same machine simultaneously can result in a last-write-wins scenario for the lock file. The lock file will be valid (atomic rename guarantees this), but it may reflect only one of the two applies. To avoid this:

- Do not run concurrent applies against the same machine
- Use CI/CD serialization (job queues) for production applies
- The event log will contain entries from both runs, providing a full audit trail even in the race condition case

## Advanced State Operations

### Comparing States Over Time

Use `forjar diff` to see what changed between applies:

```bash
# Save a snapshot before changing config
cp -r state/ /tmp/state-before/

# Make config changes and apply
forjar apply -f forjar.yaml --state-dir state/

# See what changed
forjar diff /tmp/state-before/ state/
```

Output:

```
DIFF: 2 change(s)

  web-server/nginx-config:
    status: converged → converged
    hash: blake3:a1b2... → blake3:c3d4...
    detail: content changed

  web-server/new-resource:
    status: (none) → converged
    detail: added
```

### Importing Live State

If you're adopting forjar on an existing machine, import the current state without making changes:

```bash
forjar import -f forjar.yaml -m web-server --state-dir state/
```

This runs state_query scripts to capture the current live hashes and creates a lock file without applying anything. Subsequent `forjar apply` runs will only change resources whose desired state differs from what was captured.

### State Directory Per Environment

For multi-environment setups, use separate state directories:

```bash
# Staging
forjar apply -f staging.yaml --state-dir state-staging/

# Production
forjar apply -f production.yaml --state-dir state-production/

# Compare staging vs production
forjar diff state-staging/ state-production/
```

### Programmatic State Access

Lock files are plain YAML — parse them with any YAML library:

```python
import yaml

with open("state/web-server/state.lock.yaml") as f:
    lock = yaml.safe_load(f)

for name, resource in lock["resources"].items():
    if resource["status"] == "failed":
        print(f"FAILED: {name}")
```

```bash
# jq-style queries with yq
yq '.resources | to_entries[] | select(.value.status == "failed") | .key' \
  state/web-server/state.lock.yaml
```

## State Internals

### Atomic Writes

Lock files are written atomically using a temp-file-and-rename pattern:

1. Write to `state/{machine}/state.lock.yaml.tmp`
2. Flush and sync to disk
3. Rename `*.tmp` → `state.lock.yaml` (atomic on POSIX)

This ensures a power failure or crash never leaves a corrupted lock file. The worst case is a stale-but-valid previous lock file.

### Hash Computation

Each resource's `hash` field is the BLAKE3 hash of its **desired state** — all configuration fields that affect what should be applied. This includes:

| Resource Type | Fields Hashed |
|--------------|---------------|
| File | path, content/source, owner, group, mode, state |
| Package | packages list, provider, version, state |
| Service | name, enabled, state |
| Mount | path, target, fs_type, options, state |
| User | name, uid, shell, home, groups, system_user, ssh_authorized_keys |
| Cron | name, schedule, command |
| Docker | name, image, ports, environment, volumes, restart |
| Network | name, port, protocol, action, from_addr |

The hash format is `blake3:<hex>` (e.g., `blake3:a1b2c3d4...`). When the desired-state hash matches the stored hash, the resource is skipped (unchanged).

### Content Hash vs Desired-State Hash

Two different hashes serve different purposes:

- **`hash`**: Hash of the desired state in the config file. Changes when you edit your `forjar.yaml`.
- **`content_hash`** (in details): Hash of the actual file content on disk. Changes when someone modifies the file on the machine.

Drift detection compares `content_hash` against the live file to detect out-of-band changes. The `hash` field determines whether a new apply is needed.

### Live Hash

For non-file resources (packages, services, etc.), the `live_hash` in details captures the hash of the `state_query_script` output at apply time. This enables drift detection for all resource types:

```yaml
resources:
  nginx-svc:
    type: service
    status: converged
    hash: "blake3:abc..."
    details:
      live_hash: "blake3:def..."
```

On drift check, forjar re-runs the state query script and compares the new output hash against the stored `live_hash`. Any difference indicates configuration drift.

## Event Log Deep Dive

### Event Schema

Each line in `events.jsonl` is a self-contained JSON object:

```json
{
  "timestamp": "2026-02-25T14:30:00.123Z",
  "event": "resource_converged",
  "resource_id": "nginx-conf",
  "resource_type": "file",
  "hash": "blake3:abc123...",
  "duration_seconds": 0.42,
  "machine": "web-01"
}
```

### Event Types

| Event | When | Key Fields |
|-------|------|------------|
| `resource_converged` | Apply succeeded | resource_id, hash, duration |
| `resource_failed` | Apply failed | resource_id, error, duration |
| `resource_skipped` | Hash unchanged | resource_id, reason |
| `drift_detected` | Drift check found changes | resource_id, expected_hash, actual_hash |

### Log Rotation

Event logs grow indefinitely by design — they're an audit trail. For large deployments, manage with standard log rotation:

```bash
# Rotate logs older than 90 days
find state/ -name "events.jsonl" -exec sh -c '
  tail -n 10000 "$1" > "$1.tmp" && mv "$1.tmp" "$1"
' _ {} \;
```

Or archive to a centralized logging system:

```bash
# Ship to your log aggregator
for f in state/*/events.jsonl; do
  machine=$(basename $(dirname "$f"))
  cat "$f" | jq -c '. + {machine: "'$machine'"}' | \
    curl -X POST -d @- https://logs.example.com/ingest
done
```

## State Recovery

### Rebuilding from Scratch

If state files are lost, `forjar import` reconstructs them from live machine state:

```bash
# Re-import all machines
forjar import -f forjar.yaml --state-dir state/

# Verify reconstruction
forjar drift -f forjar.yaml --state-dir state/
```

Import runs `state_query_script` for each resource and captures the current live state. The resulting lock file reflects what is actually deployed, not what the config says should be deployed.

### Handling Conflicts

When the lock file says a resource is converged but drift detection finds changes:

1. **If the config hasn't changed**: Someone modified the machine out-of-band. Run `forjar apply` to reconverge.
2. **If the config changed**: Run `forjar apply` — the new desired-state hash will trigger a re-apply.
3. **If you want to accept the drift**: Run `forjar import` to capture the current state as the new baseline.

### State File Compatibility

Lock files include a `schema` field for forward compatibility:

```yaml
schema: '1.0'
```

Forjar validates the schema version on load. Future versions may introduce `schema: '2.0'` with migration support.

## Migration Guide

When a new version of forjar changes the lock file format, the `schema` field drives forward compatibility. This section describes what happens during version transitions and how to handle them.

### Schema Version Contract

Every lock file begins with a `schema` field:

```yaml
schema: '1.0'
```

Forjar checks this field when loading a lock file. The behavior depends on the relationship between the lock file's schema version and the running forjar version's expected schema:

| Lock Schema | Forjar Expected | Behavior |
|-------------|-----------------|----------|
| `1.0` | `1.0` | Normal operation, no migration needed |
| `1.0` | `2.0` | Auto-migrate: forjar reads the old format and writes the new format on next save |
| `2.0` | `1.0` | Error: forjar refuses to load a lock file from a newer schema it does not understand |
| Missing | Any | Error: malformed lock file, must be rebuilt |

### What Triggers a Format Change

Lock file format changes are reserved for structural changes to the schema itself. The following do not require a schema version bump:

- Adding new resource types (the `type` field is an enum that extends without breaking existing entries)
- Adding new keys to the `details` map (details is a `HashMap<String, Value>` and tolerates unknown keys)
- Adding new event types to the event log (JSONL consumers ignore unknown event tags)

A schema version bump would be required for:

- Renaming or removing existing top-level fields (`machine`, `hostname`, `resources`, etc.)
- Changing the structure of `ResourceLock` (for example, splitting `hash` into separate fields)
- Changing the serialization format from YAML to another format

### Upgrade Procedure

When upgrading forjar to a version with a new lock schema:

```bash
# 1. Back up current state
cp -r state/ state-backup-$(date +%Y%m%d)/

# 2. Upgrade forjar binary
cargo install forjar

# 3. Run apply — forjar auto-migrates lock files on write
forjar apply -f forjar.yaml

# 4. Verify the migrated state
forjar status -f forjar.yaml
forjar drift -f forjar.yaml
```

The apply command reads the old-format lock, builds the in-memory `StateLock` struct, and writes it back in the new format. No separate migration command is needed because the Rust structs always reflect the current schema, and `serde` handles deserialization of the old format through default values and optional fields.

### Downgrade Considerations

Downgrading forjar to an older version after a schema migration is not supported. The older binary will refuse to load lock files with a schema version it does not recognize. If you need to downgrade:

```bash
# Restore from backup
rm -rf state/
cp -r state-backup-20260225/ state/

# Or rebuild state from scratch with the older version
rm -rf state/
forjar apply -f forjar.yaml --force
```

### Event Log Compatibility

The event log (`events.jsonl`) does not have a schema version. Each line is a self-contained JSON object with an `event` field that identifies its type. Consumers should ignore unknown event types rather than failing. This design means the event log format is forward-compatible indefinitely — new event types can be added without breaking existing log parsers.

## Output Persistence and Cross-Stack Data Flow

Forjar persists resolved output values in the global lock file, enabling cross-stack data flow between independent configurations.

### How It Works

When a config declares `outputs:`, forjar resolves the template expressions and stores them in `forjar.lock.yaml` after every successful apply:

```yaml
# forjar.yaml (producer)
outputs:
  db_host:
    value: "{{machines.db.addr}}"
    description: "Database server address"
  api_port:
    value: "{{params.port}}"
    description: "API server port"
```

After apply, the global lock includes:

```yaml
# state/forjar.lock.yaml
schema: '1.0'
name: database-stack
last_apply: 2026-03-01T14:00:00Z
generator: forjar 1.1.1
machines:
  db:
    resources: 5
    converged: 5
    failed: 0
    last_apply: 2026-03-01T14:00:00Z
outputs:
  db_host: 10.0.0.5
  api_port: "8080"
```

### Consuming Outputs

Another config reads these outputs via the `forjar-state` data source:

```yaml
# forjar.yaml (consumer)
data:
  db:
    type: forjar-state
    state_dir: ../database-stack/state
    outputs:
      - db_host
    max_staleness: 24h    # warn if producer hasn't applied in 24 hours
```

The `max_staleness` field triggers a warning if the producer's `last_apply` timestamp is older than the threshold. Supported units: `s` (seconds), `m` (minutes), `h` (hours), `d` (days).

### Viewing Outputs

```bash
# Show all resolved outputs
forjar output -f forjar.yaml

# Show a specific output
forjar output -f forjar.yaml --key db_host

# JSON output for scripting
forjar output -f forjar.yaml --json
```

## State Integrity Verification

Forjar writes a BLAKE3 sidecar file (`.b3`) alongside every lock file. Before apply, integrity is verified automatically.

### Sidecar Files

After writing `state.lock.yaml`, forjar also writes `state.lock.yaml.b3` containing the BLAKE3 hash of the lock file contents. This enables detection of tampering or corruption.

```
state/
  forjar.lock.yaml        # Global lock
  forjar.lock.yaml.b3     # BLAKE3 hash sidecar
  web/
    state.lock.yaml        # Per-machine lock
    state.lock.yaml.b3     # BLAKE3 hash sidecar
```

### Pre-Apply Verification

Before every apply, forjar checks:

1. All lock files are valid YAML (catches corruption)
2. Lock file content matches its `.b3` sidecar hash (catches tampering)

Missing sidecars produce a warning (backward-compatible with older state). Hash mismatches produce an error and block apply unless `--yes` is used.

### Manual Verification

```bash
# Verify lock file integrity
forjar lock-verify --state-dir state

# Override integrity check
forjar apply -f forjar.yaml --yes
```

## Event-Sourced State Reconstruction

Forjar can reconstruct the state of any machine at any point in time by replaying the event log.

### Usage

```bash
# Reconstruct state at a specific timestamp
forjar state-reconstruct --machine web --at 2026-03-01T14:00:00Z

# JSON output
forjar state-reconstruct --machine web --at 2026-03-01T14:00:00Z --json

# Custom state directory
forjar state-reconstruct --machine web --at 2026-03-01T14:00:00Z --state-dir state
```

### How It Works

The `state-reconstruct` command reads `state/<machine>/events.jsonl` and replays events chronologically up to the specified timestamp:

- `resource_converged` events set the resource to converged with its hash
- `resource_failed` events set the resource to failed with error details
- `drift_detected` events update the resource to drifted status

The result is a `StateLock` representing the machine's state at that moment.

### Use Cases

- **Point-in-time recovery**: Understand what was deployed at a specific moment
- **Audit**: Verify what resources were active during an incident
- **Debugging**: Compare reconstructed state at two timestamps to find what changed

## SQLite Query Engine

Forjar can ingest flat-file state into a SQLite database for sub-second queries across the entire stack (FJ-2001).

### Configuration

```rust
use forjar::core::types::SqliteConfig;

let config = SqliteConfig::default();
// DB at state/forjar.db, WAL mode, FTS5 enabled, 8MB cache
```

The database is configured with WAL mode for concurrent reads, FTS5 for full-text search, and tuned PRAGMAs (busy_timeout, mmap, cache_size).

### Schema

The schema (version 2) creates core tables, derived tables, and FTS5 indexes:

| Table | Purpose |
|-------|---------|
| `machines` | Machine inventory with transport metadata |
| `resources` | Resource state per machine and generation |
| `generations` | Generation metadata with config hashes |
| `events` | Apply/converge/destroy event log |
| `run_logs` | Per-resource execution logs |
| `destroy_log` | Destroyed resource records (ingested from `destroy-log.jsonl`) |
| `drift_findings` | Drift detection results with expected vs actual hashes |
| `ingest_cursor` | Per-machine ingest bookkeeping for incremental updates |
| `resources_fts` | FTS5 full-text search (porter tokenizer, no raw JSON) |
| `events_fts` | FTS5 search over events |
| `run_logs_fts` | FTS5 search over run logs |

Twelve indexes cover the common query patterns (by machine, type, status, generation, run_id, path, drift, destroy).

### Incremental Ingest

The `IngestCursor` tracks which generations have been ingested per machine, enabling incremental ingest without re-processing:

```
Ingest: 120 resources, 8 generations, 45 run logs from 3 machines (1.23s)
```

### Query Enrichments

The `QueryEnrichments` flags control which additional data is joined into query results:

| Flag | Description |
|------|-------------|
| `--history` | Generation history |
| `--drift` | Drift findings |
| `--timing` | Duration statistics |
| `--churn` | Change frequency |
| `--health` | Health summary |
| `--destroy-log` | Destroy history |
| `--reversibility` | Reversibility analysis |
| `-G` | Git history fusion via RRF |

## Generation Diff

Compare resources across generations to see what changed:

```bash
forjar diff --generation 5 8
```

Output:
```
Diff: generation 5 → 8 (intel)
1 added, 1 modified, 1 removed, 2 unchanged
  + monitoring-agent (package)
  ~ bash-aliases (file) — hash changed
  - legacy-cron (service)
```

Each resource is classified as added (`+`), modified (`~`), removed (`-`), or unchanged. Modified resources show old and new BLAKE3 hashes for auditing.

Use `--json` for machine-readable output in CI pipelines.

## Task State Model

Task resources (`type: task`) use extended state tracking beyond simple converged/failed status. Each task mode stores different state:

### Pipeline State

Pipeline tasks track per-stage status in the lock file:

| Field | Description |
|-------|-------------|
| `stages[].name` | Stage name |
| `stages[].status` | `pending`, `running`, `passed`, `failed`, `skipped` |
| `stages[].exit_code` | Command exit code |
| `stages[].duration_ms` | Execution time |
| `stages[].input_hash` | BLAKE3 hash of inputs (for cache skip) |
| `last_completed` | Index of last passed stage |

### Service State

Long-running service tasks track health:

| Field | Description |
|-------|-------------|
| `pid` | Process ID (if running) |
| `healthy` | Last health check result |
| `consecutive_failures` | Consecutive failed health checks |
| `restart_count` | Total restarts since initial start |

### Dispatch State

Parameterized tasks track invocation history:

| Field | Description |
|-------|-------------|
| `invocations[]` | Recent invocations with timestamp, exit_code, duration, caller |
| `total_invocations` | Lifetime invocation count |

### GPU Scheduling

Multi-GPU tasks use `GpuSchedule` for device assignment:

```
CUDA_VISIBLE_DEVICES=0  train-a
CUDA_VISIBLE_DEVICES=1  train-b
CUDA_VISIBLE_DEVICES=0  train-c  (round-robin wraps)
```

### Barrier Tasks

Cross-machine synchronization uses `BarrierTask`:

```
barrier/sync-training: waiting for gpu-1, gpu-2 (33%)
barrier/sync-training: SATISFIED
```

All task state is stored in `state/<machine>/state.lock.yaml` under the task resource's `details` map, maintaining compatibility with the existing lock file schema.

## Destroy Log

When `forjar destroy` removes resources, it writes a destroy log to `state/destroy-log.jsonl` for undo-destroy recovery:

```jsonl
{"timestamp":"2026-03-05T14:30:00Z","machine":"web-01","resource_id":"nginx-config","resource_type":"file","pre_hash":"blake3:abc123","generation":5,"config_fragment":"type: file\npath: /etc/nginx/nginx.conf\n","reliable_recreate":true}
```

Each entry records the pre-destroy state: the resource's hash, config fragment, and whether it can be reliably recreated. Resources with inline `content:` are flagged as `reliable_recreate: true`.

On partial destroy failure, only succeeded resource entries are removed from the lock file. Failed resources keep their lock entries so the next `forjar apply` can re-converge them.

### Undo-Destroy Replay

`forjar undo-destroy` reads the destroy log and replays entries to recreate destroyed resources:

```bash
# Preview what would be recreated
forjar undo-destroy --dry-run

# Replay reliable entries only
forjar undo-destroy --yes

# Include unreliable entries (best-effort)
forjar undo-destroy --force --yes

# Filter to specific machine
forjar undo-destroy --machine web-01 --yes
```

The replay process:
1. Reads `destroy-log.jsonl` entries, filtering by machine if specified
2. Classifies entries as **reliable** (`config_fragment` + `reliable_recreate: true`) or **unreliable**
3. For each entry: deserializes `config_fragment` to a `Resource`, generates a convergence script via `codegen::apply_script()`, and executes via `transport::exec_script()`
4. Reports replayed/failed counts

Without `--force`, unreliable entries are skipped. Entries without `config_fragment` are always skipped (no way to reconstruct).

## Force Re-Apply

`forjar apply --force` bypasses the planner's hash comparison, forcing all resources to be re-applied even if their config hasn't changed. This is essential for:

- **Secret rotation**: When `{{ secrets.PASSWORD }}` resolves to a new value but the template hash is unchanged
- **External state recovery**: When the target machine's state was modified outside forjar
- **Drift remediation**: When you want to re-converge everything regardless of planner state

Without `--force`, the planner uses BLAKE3 hash comparison for O(1) idempotency checks. With `--force`, it treats every resource as needing re-application.

## Active Undo

`forjar undo` reverts to a previous generation by restoring lock files and re-applying. Unlike `rollback` (which reads config from git history), `undo` operates on the generation snapshots in `state/generations/`.

```bash
# Preview changes
forjar undo --dry-run

# Undo last apply
forjar undo --yes

# Undo last 3 generations
forjar undo --generations 3 --yes
```

The undo process:
1. Reads the target generation's lock files and metadata
2. Computes a resource diff (creates, updates, destroys)
3. In dry-run mode, prints the diff and exits
4. Writes `undo-progress.yaml` per machine for resume support
5. Restores lock files from the target generation
6. Re-applies the current config with `--force` to converge
7. Updates progress status to `completed` or `partial`

### Undo Resume

If an undo fails partway (e.g., SSH timeout), use `--resume` to pick up where it left off:

```bash
forjar undo --resume --yes
```

Resume reads `state/<machine>/undo-progress.yaml`, identifies pending and failed resources, and re-applies. Running `--resume` on a completed undo is a no-op.
