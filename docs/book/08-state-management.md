# State Management

Forjar uses a file-based state system to track what has been applied to each machine. This enables idempotent applies, drift detection, and rollback.

## State Directory Layout

```
state/
  forjar.lock.yaml            # Global lock — summary of all machines
  intel/
    state.lock.yaml            # Per-machine lock — resource hashes and status
    events.jsonl               # Provenance event log — append-only audit trail
  web-server/
    state.lock.yaml
    events.jsonl
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
