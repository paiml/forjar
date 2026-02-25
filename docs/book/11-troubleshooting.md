# Troubleshooting

Common problems, error messages, and their solutions.

## Validation Errors

### "version must be \"1.0\""

Your config uses an unsupported version.

```yaml
# Wrong
version: "2.0"

# Correct
version: "1.0"
```

### "resource 'X' references unknown machine 'Y'"

A resource targets a machine not defined in the `machines:` block.

```bash
# Show which machines are defined
forjar show -f forjar.yaml --json | jq '.machines | keys'
```

Fix: add the machine to `machines:` or correct the `machine:` field on the resource.

Exception: `machine: localhost` is always valid — it refers to the local machine and does not need a definition.

### "resource 'X' depends on unknown resource 'Y'"

A `depends_on` entry references a resource that doesn't exist. Common causes:

- Typo in the dependency name
- The dependency was renamed or removed
- Recipe expansion changed resource IDs (recipe resources get prefixed)

```bash
# List all resource IDs
forjar show -f forjar.yaml --json | jq '.resources | keys'
```

### "resource 'X' depends on itself"

Self-dependency is caught at parse time. Remove the resource's own ID from its `depends_on` list.

### "resource 'X' (file) has both content and source (pick one)"

A file resource can use inline `content:` or a `source:` file path, but not both. Choose one.

### "resource 'X' (file) state=symlink requires a target"

Symlink files need a `target:` field specifying where the link points.

```yaml
my-link:
  type: file
  machine: m1
  path: /usr/local/bin/myapp
  state: symlink
  target: /opt/myapp/bin/myapp
```

## SSH Connection Issues

### "ssh: connect to host X port 22: Connection refused"

The SSH daemon isn't running or the port is wrong.

```bash
# Test connectivity manually
ssh -o BatchMode=yes -o ConnectTimeout=5 user@host echo ok

# Check if sshd is listening
ssh user@host 'ss -tlnp | grep :22'
```

### "Permission denied (publickey)"

Forjar uses `BatchMode=yes` (no interactive password prompts). Ensure:

1. Your SSH key is added: `ssh-add -l`
2. The key is in `~/.ssh/authorized_keys` on the target
3. If using `ssh_key:` in your machine config, the path is correct and the file has `0600` permissions

```yaml
machines:
  web:
    hostname: web.example.com
    addr: 203.0.113.10
    user: deploy
    ssh_key: ~/.ssh/deploy_ed25519
```

### "Host key verification failed"

Forjar uses `StrictHostKeyChecking=accept-new`, which accepts new hosts but rejects changed fingerprints. If a host was rebuilt:

```bash
# Remove the old fingerprint
ssh-keygen -R hostname-or-ip

# Then re-run forjar — the new key will be accepted
forjar apply -f forjar.yaml
```

## Container Transport Issues

### "machine 'X' uses container transport but has no 'container' block"

When `transport: container` is set, a `container:` block is required:

```yaml
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
```

### "machine 'X' is ephemeral but has no container image"

Ephemeral containers (the default) need an `image:` to create from:

```yaml
container:
  runtime: docker
  image: ubuntu:22.04    # Required for ephemeral: true
  ephemeral: true
```

For non-ephemeral containers (attaching to an existing one), set `ephemeral: false` and provide a `name:`.

### "container runtime must be 'docker' or 'podman'"

Only `docker` and `podman` are supported runtimes.

### Container command not found

If resource scripts fail inside containers with "command not found", the container image may be too minimal. Ensure the image has:

- `bash` (forjar pipes scripts to `bash`)
- `coreutils` (for `cat`, `chmod`, `chown`, `mkdir`)
- `sudo` (if resources need privilege escalation)

```dockerfile
# Minimal test target
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y bash coreutils sudo
```

## State and Drift Issues

### "failed to read state/X/state.lock.yaml"

The state directory doesn't exist or isn't writable. Create it:

```bash
forjar init -f forjar.yaml --state-dir ./state
```

Or specify a writable path:

```bash
forjar apply -f forjar.yaml --state-dir /tmp/forjar-state
```

### False drift detection

If `forjar drift` reports changes that haven't actually changed, possible causes:

1. **Non-deterministic output**: State query scripts produce different output each run (e.g., timestamps, process IDs). The BLAKE3 hash will differ.
2. **Content hash vs. file hash**: Ensure `content_hash` uses file content hashing, not metadata.
3. **Missing state lock**: First run has no baseline — everything appears as drift.

```bash
# Reset state by re-applying
forjar apply -f forjar.yaml --state-dir ./state

# Then check drift
forjar drift -f forjar.yaml --state-dir ./state
```

### "corrupted state lock" / YAML parse errors

If the state lock file is corrupted (e.g., from an interrupted write), delete it and re-apply:

```bash
rm state/machine-name/state.lock.yaml
forjar apply -f forjar.yaml --state-dir ./state
```

Forjar uses atomic writes (write to temp file, then rename), so corruption is rare but possible if the filesystem doesn't support atomic rename.

## DAG and Dependency Issues

### Cycle detected in dependency graph

Forjar uses topological sort (Kahn's algorithm) for execution ordering. Cycles are detected at plan time:

```bash
# Visualize the dependency graph
forjar graph -f forjar.yaml
```

Example cycle: A → B → C → A. Fix by restructuring dependencies or breaking the cycle with an intermediate resource.

### Unexpected execution order

Resources with no dependencies execute in alphabetical order (deterministic tie-breaking). To control order:

```yaml
resources:
  setup-dirs:
    type: file
    # ...

  deploy-app:
    type: file
    depends_on: [setup-dirs]   # Runs after setup-dirs
```

```bash
# Preview execution order
forjar plan -f forjar.yaml
```

## Resource-Specific Issues

### Package: "E: Could not get lock /var/lib/dpkg/lock"

Another apt process is running. Wait for it to finish or kill it:

```bash
# On the target machine
sudo kill $(cat /var/lib/dpkg/lock-frontend 2>/dev/null) 2>/dev/null
sudo rm -f /var/lib/dpkg/lock-frontend /var/lib/dpkg/lock
```

### Service: "Failed to connect to bus"

Service management requires systemd. In containers without systemd, service resources will fail. Use container transport with a systemd-enabled image, or skip service resources in container tests.

### Mount: "mount: only root can do that"

Mount operations require root. Forjar generates `$SUDO` detection:

```bash
SUDO=""
[ "$(id -u)" -ne 0 ] && SUDO="sudo"
```

Ensure the target user has passwordless sudo for mount commands, or run as root.

### User: "useradd: cannot lock /etc/passwd"

Another user management command is running, or `/etc/passwd` is locked. Retry after the lock clears.

### Cron: schedule field count

Cron schedules must have exactly 5 fields: `minute hour day-of-month month day-of-week`.

```yaml
# Wrong (6 fields)
schedule: "0 2 * * * *"

# Correct (5 fields)
schedule: "0 2 * * *"
```

## Architecture Filtering

### Resource skipped on wrong architecture

Resources with an `arch:` filter only apply to machines with a matching architecture:

```yaml
resources:
  arm-pkg:
    type: package
    machine: m1
    packages: [libraspberrypi-bin]
    provider: apt
    arch: [aarch64]   # Only applies on ARM64 machines
```

If the machine arch doesn't match, the resource is silently skipped during apply. Check with:

```bash
forjar show -f forjar.yaml --json | jq '.machines | to_entries[] | {key, arch: .value.arch}'
```

Valid architectures: `x86_64`, `aarch64`, `armv7l`, `riscv64`, `s390x`, `ppc64le`.

## CLI Issues

### "failed to read X: No such file or directory"

The config file path is wrong. Forjar defaults to `./forjar.yaml`:

```bash
# Explicit path
forjar validate -f /path/to/forjar.yaml

# Or from the project directory
cd my-infra && forjar validate
```

### Import scanning finds nothing

`forjar import` scans live system state. Ensure:

1. You're running on the target machine (or via SSH)
2. The scan type is correct: `--scan services`, `--scan users`, `--scan files`, `--scan cron`
3. You have read permissions for the scanned resources

```bash
# Scan everything
forjar import --scan services --scan users --scan cron --name my-server
```

## Drift Detection Issues

### "no findings" when drift should exist

Drift detection only checks resources with `status: converged` in the lock file. If a resource failed during apply, it won't be drift-checked.

```bash
# Check lock file status
cat state/web-server/state.lock.yaml | grep status

# Force re-apply, then check drift
forjar apply -f forjar.yaml --force
forjar drift -f forjar.yaml
```

### Drift detected immediately after apply

This usually means the apply script doesn't produce the exact content expected. Common causes:
- File content has a trailing newline that wasn't in the YAML `content` field
- Package auto-generates config files that modify managed files
- Systemd restarts change service state between apply and drift check

### Anomaly detection shows high churn

Resources that converge repeatedly (z-score > 1.5) indicate external modification:

```bash
# Identify high-churn resources
forjar anomaly --state-dir state --json | jq '.anomalies[] | select(.type == "high_churn")'

# Common fixes:
# 1. Add "Managed by forjar — do not edit" comments to files
# 2. Disable unattended-upgrades for managed packages
# 3. Coordinate with monitoring tools that restart services
```

## Performance Issues

### Apply takes too long

```bash
# Profile which resources take the most time
cat state/web-server/events.jsonl | jq 'select(.event == "resource_converged") | {resource, duration_seconds}' | sort -t: -k2 -n

# Common bottlenecks:
# - Package installs (apt update is slow)
# - Large file transfers
# - Service restarts with health checks
```

### SSH connection timeouts

Forjar uses `ConnectTimeout=5` by default. For slow networks:

```bash
# Increase timeout per-command
forjar apply -f forjar.yaml --timeout 30

# Or check SSH connectivity directly
ssh -o BatchMode=yes -o ConnectTimeout=5 user@host echo ok
```

## Debugging Checklist

When something goes wrong, work through this checklist:

1. **Validate first**: `forjar validate -f forjar.yaml` — catches 90% of config errors
2. **Check the plan**: `forjar plan -f forjar.yaml` — shows what would change
3. **Dry run**: `forjar apply -f forjar.yaml --dry-run` — previews without executing
4. **Check events**: `cat state/<machine>/events.jsonl | jq .` — see what happened
5. **Check lock**: `cat state/<machine>/state.lock.yaml` — see stored state
6. **Test locally**: Add a `localhost` machine and test resources locally first
7. **Use containers**: Add a container machine for safe, isolated testing

## Container Transport Issues

### "docker: command not found"

Container transport requires Docker (or Podman) installed on the host:

```bash
# Check Docker is available
docker --version

# For Podman, set runtime:
#   container:
#     runtime: podman
```

### Container Fails to Start

If `forjar apply` fails during container creation:

```bash
# Manual container test
docker run -d --name forjar-test --init ubuntu:22.04 sleep infinity
docker exec forjar-test bash -c "echo ok"
docker rm -f forjar-test
```

Common causes:
- Image not available locally (`docker pull ubuntu:22.04`)
- Port conflicts from previous runs (`docker rm -f forjar-<machine-key>`)
- Insufficient disk space for container images

### systemd Inside Containers

Some resources (services) need systemd, which requires `privileged: true`:

```yaml
machines:
  test:
    hostname: test
    addr: container
    transport: container
    container:
      image: ubuntu:22.04
      privileged: true    # Needed for systemctl
```

Without `privileged`, service resources will fail with `systemctl` errors.

## Recipe Errors

### "recipe file not found: recipes/X.yaml"

Recipe files must be at `recipes/{name}.yaml` relative to your `forjar.yaml`:

```
my-project/
  forjar.yaml           # References recipe: web-server
  recipes/
    web-server.yaml     # Must exist here
```

### "input 'Y' is required but not provided"

A recipe input has no default and wasn't provided:

```yaml
# Recipe definition
recipe:
  inputs:
    domain:
      type: string      # No default — required

# Usage — must provide the input
resources:
  web:
    type: recipe
    recipe: web-server
    inputs:
      domain: example.com   # Required
```

### "input 'Y' value Z exceeds max"

An integer input is out of its declared range:

```yaml
# Recipe declares min/max
recipe:
  inputs:
    port:
      type: int
      min: 1
      max: 65535

# Usage — value must be within range
inputs:
  port: 70000    # Error: exceeds max 65535
```

### Recipe Expansion Debug

To see how recipes expand:

```bash
# Validate shows expanded resource count
forjar validate -f forjar.yaml

# Graph shows namespaced resources
forjar graph -f forjar.yaml

# Plan shows all expanded resources with actions
forjar plan -f forjar.yaml --state-dir state/
```

## Drift Detection Issues

### False Drift on Package Resources

Package drift can be triggered by automatic updates (unattended-upgrades):

```bash
# Check if drift is real
forjar drift -f forjar.yaml --json | jq '.findings[]'

# Pin package versions to prevent drift
resources:
  my-pkg:
    type: package
    provider: apt
    packages: [nginx=1.24.0-1]    # Pin version
```

### Drift After Manual Changes

If someone edits a managed file manually:

```bash
# See what drifted
forjar drift -f forjar.yaml --state-dir state/

# Option 1: Restore to desired state
forjar apply -f forjar.yaml --state-dir state/ --force

# Option 2: Accept the change — update your config to match
# Edit forjar.yaml, then apply
```

### No Drift Detected (But Expected)

Drift detection only works for resources with `status: converged` in the lock file:
- Failed resources are skipped (no baseline to compare)
- Resources never applied are skipped (no hash recorded)
- Resources with missing `content_hash` or `live_hash` are skipped

```bash
# Check lock file for resource status
cat state/<machine>/state.lock.yaml | grep -A2 "resource-name"
```

## Performance Issues

### Slow Apply on Many Resources

Forjar applies resources sequentially per machine (to respect dependency order). For configs with many independent resources:

```bash
# Use parallel machine execution
forjar apply -f forjar.yaml --parallel

# Or target specific resources
forjar apply -f forjar.yaml -r slow-resource
```

### Large File Hashing

BLAKE3 hashes files using a 64KB streaming buffer, so memory usage is constant regardless of file size. If hashing seems slow, check:

```bash
# File size
ls -lh /path/to/large/file

# Disk I/O
iostat -x 1 3
```

### Event Log Size

Event logs grow unbounded. For long-running systems:

```bash
# Check sizes
du -sh state/*/events.jsonl

# Rotate (keep last 1000 entries)
for f in state/*/events.jsonl; do
  tail -1000 "$f" > "$f.tmp" && mv "$f.tmp" "$f"
done
```

## Error Exit Codes

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success |
| 1 | General error (validation, transport, config) |
| 1 | Drift detected (with `--tripwire` flag) |
| 2 | CLI argument error |

## Common Patterns That Cause Issues

### Circular Dependencies

```yaml
# This causes a cycle error:
resources:
  a:
    depends_on: [b]
  b:
    depends_on: [a]
```

Fix: Remove one direction of the dependency, or introduce a shared dependency.

### Missing Machine for Multi-Machine Resources

```yaml
resources:
  pkg:
    type: package
    machine: [web, db, missing-machine]   # Error if missing-machine undefined
```

Fix: Define all machines in the `machines:` block, or remove the unknown reference.

### Template Resolution Failures

```yaml
params:
  env: production

resources:
  config:
    type: file
    content: "env={{params.evn}}"   # Typo: 'evn' not 'env'
```

Unresolved templates pass through unchanged — the file will literally contain `{{params.evn}}`. Check for typos in template references.

## Getting Help

```bash
# Command help
forjar --help
forjar apply --help

# Validate before applying
forjar validate -f forjar.yaml && forjar plan -f forjar.yaml

# Dry run
forjar apply -f forjar.yaml --dry-run

# Verbose output
forjar apply -f forjar.yaml --verbose

# Inspect generated scripts
forjar plan -f forjar.yaml --output-dir /tmp/audit/
```
