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

## Container Transport Issues

### Docker Not Found

```
transport error: failed to execute docker exec: No such file or directory
```

The container runtime binary isn't in PATH. Verify installation:

```bash
which docker    # Should return /usr/bin/docker or similar
docker info     # Should show Docker daemon info
```

For rootless Docker, ensure the socket is accessible. For Podman, set `runtime: podman` in the container config.

### Container Not Running

```
container 'forjar-test' is not running
```

For ephemeral containers, forjar auto-starts them with `docker run -d --init --name <name> <image> sleep infinity`. If this fails:

```bash
# Check if container exists but is stopped
docker ps -a --filter name=forjar-test

# Check Docker daemon logs
journalctl -u docker --since "5 minutes ago"

# Manually test container creation
docker run -d --init --name test-container ubuntu:22.04 sleep infinity
docker exec -i test-container bash -c 'echo OK'
docker rm -f test-container
```

### Privileged Operations in Containers

Some resource types need elevated permissions:

| Resource | Requires | Solution |
|----------|----------|----------|
| Package (apt) | Root | Use root user (default) |
| Service (systemctl) | systemd | Use `--privileged --init` or skip in container |
| Mount | CAP_SYS_ADMIN | Use `privileged: true` in container config |
| File | Write permission | Use root user (default) |

For systemd-dependent services in containers, use an image with systemd pre-configured:

```yaml
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: jrei/systemd-ubuntu:22.04
      privileged: true
      init: false   # systemd replaces init
```

## Performance Issues

### Slow Applies

If applies take longer than expected:

1. **Check script execution time**: Use `--verbose` to see per-resource timing
2. **Package manager mirrors**: Slow apt/yum mirrors dominate apply time
3. **Large file transfers**: Base64-encoded files in shell scripts are slower for large files
4. **Sequential execution**: Resources without dependencies run in dependency order, not parallel

```bash
# Time individual resources
forjar apply -f forjar.yaml --state-dir state/ --verbose 2>&1 | \
  grep "duration"
```

### State Directory Growth

Event logs (`events.jsonl`) grow unbounded. For long-running deployments:

```bash
# Check state directory size
du -sh state/

# Count events per machine
for f in state/*/events.jsonl; do
  echo "$(wc -l < "$f") events in $f"
done

# Trim to last 1000 events per machine
for f in state/*/events.jsonl; do
  tail -n 1000 "$f" > "$f.tmp" && mv "$f.tmp" "$f"
done
```

## Debugging Techniques

### Script Inspection

Export generated scripts for manual review:

```bash
# Export all scripts to a directory
forjar plan -f forjar.yaml --output-dir /tmp/audit/

# Review what would be executed
ls /tmp/audit/
# check_nginx-conf.sh  apply_nginx-conf.sh  state_query_nginx-conf.sh

# Run a check script manually
bash /tmp/audit/check_nginx-conf.sh
echo "exit code: $?"
```

### Dry Run Analysis

```bash
# See what would change without touching anything
forjar apply -f forjar.yaml --state-dir state/ --dry-run

# Output shows:
# ✓ nginx-pkg: unchanged (hash match)
# → nginx-conf: would apply (hash changed)
# ✓ nginx-svc: unchanged
```

### Event Log Queries

```bash
# Find all failures in the last 24 hours
jq 'select(.event == "resource_failed")' state/web/events.jsonl

# Count events by type
jq -r '.event' state/web/events.jsonl | sort | uniq -c | sort -rn

# Find longest-running resources
jq -r 'select(.duration_seconds != null) | "\(.duration_seconds)s \(.resource_id)"' \
  state/web/events.jsonl | sort -rn | head -5
```

### Diff Between Applies

Compare lock files to see what changed between applies:

```bash
# Save before-state
cp state/web/state.lock.yaml /tmp/before.yaml

# Run apply
forjar apply -f forjar.yaml --state-dir state/

# Diff
diff /tmp/before.yaml state/web/state.lock.yaml
```

## bashrs Validation Failures

Forjar integrates the `bashrs` shell analysis library to lint every generated script (check, apply, state_query) before execution. When `forjar lint` or `forjar validate` reports bashrs diagnostics, use this section to diagnose and resolve them.

### Understanding Diagnostic Codes

bashrs diagnostics use a prefix convention that indicates the category of the finding:

| Prefix | Meaning | Typical Cause |
|--------|---------|---------------|
| **SEC** | Security violation | Unquoted variable expansion, injection risk |
| **DET** | Non-determinism | Use of `date`, `$$`, `$RANDOM` in scripts |
| **IDEM** | Idempotency violation | Creates resources without checking existence first |
| **SC** | ShellCheck-equivalent | Common shell scripting mistakes (e.g., `read` without `-r`) |

### SEC002: Unquoted Variable ($SUDO Pattern)

The most common finding in forjar-generated scripts is SEC002 on the `$SUDO` privilege escalation pattern:

```
warn: bashrs: web-packages/apply [SEC002] unquoted variable: $SUDO
```

This is expected and safe. Package, user, cron, and network resource handlers generate the following pattern:

```bash
SUDO=""
if [ "$(id -u)" -ne 0 ]; then SUDO="sudo"; fi
$SUDO apt-get install -y curl
```

When running as root, `$SUDO` is empty and disappears cleanly from the command line. bashrs classifies this as a warning (not an error) because the pattern is recognized as safe. Handlers that produce zero-diagnostic scripts include file, directory, symlink, service, and mount -- these use only static, single-quoted arguments with no dynamic variable expansion.

No action is required for SEC002 warnings on `$SUDO`. They will not block validation or apply.

### DET Warnings: Non-Deterministic Commands

DET-prefixed diagnostics indicate commands whose output varies between runs. In infrastructure scripts, non-determinism can cause false drift detection.

```
warn: bashrs: my-resource/apply [DET001] non-deterministic command: date
```

If you see DET warnings on your own custom scripts (via `command:` fields), audit whether the non-determinism affects state hashing. For forjar-generated scripts, DET warnings should not appear -- if they do, file a bug.

### Syntax Errors in Custom Content

If a file resource uses `content:` with embedded shell fragments that happen to be parseable, bashrs may flag syntax issues:

```
error: bashrs: config-file/apply [SC1009] parse error near unexpected token
```

This typically means the heredoc content triggered a parser edge case. Check that heredoc delimiters use single quotes (`'FORJAR_EOF'`) to prevent variable expansion inside the content block.

### Using `forjar lint --json` for Machine-Readable Diagnostics

For CI pipelines and automated processing, use the `--json` flag:

```bash
forjar lint -f forjar.yaml --json
```

Output:

```json
{
  "warnings": 3,
  "findings": [
    "bashrs: web-packages/apply [SEC002] unquoted variable: $SUDO",
    "bashrs: deploy-user/apply [SEC002] unquoted variable: $SUDO",
    "bashrs script lint: 0 error(s), 4 warning(s) across 6 resources"
  ]
}
```

In CI, you can parse this output to enforce policies:

```bash
# Fail CI if bashrs reports any errors (not warnings)
ERRORS=$(forjar lint -f forjar.yaml --json | jq '.findings[] | select(contains("error(s)"))')
if echo "$ERRORS" | grep -q "[1-9].*error(s)"; then
    echo "FAIL: bashrs reported lint errors"
    exit 1
fi
```

The JSON output includes all lint findings (unused machines, untagged resources, duplicate content) alongside bashrs diagnostics, so a single `forjar lint --json` call covers the full lint surface.

### Debugging a Failing Purification

If `forjar apply` fails with a `bashrs purify:` or `bashrs parse:` error, the generated script could not pass through the full purification pipeline (parse, AST purification, reformat). To debug:

```bash
# Export scripts for inspection
forjar plan -f forjar.yaml --output-dir /tmp/debug-scripts/

# Manually inspect the failing script
cat /tmp/debug-scripts/<machine>/<resource>.sh

# Check for common issues:
# - Unbalanced quotes
# - Unclosed heredocs
# - Bash syntax not supported by bashrs parser
```

## Common Container Transport Issues

### Container Exec Failures

When `forjar apply` reports an error during container script execution, the issue is usually in the container runtime layer, not the script itself.

**"failed to exec in container 'forjar-X': No such file or directory"**

The container runtime binary (`docker` or `podman`) is not in PATH, or the container does not exist:

```bash
# Verify the runtime is installed
which docker && docker --version
# or
which podman && podman --version

# Check if the container is running
docker ps --filter name=forjar-X

# If missing, forjar may not have created it yet. Run ensure manually:
docker run -d --init --name forjar-X ubuntu:22.04 sleep infinity
```

**"failed to exec in container 'forjar-X': <stderr output>"**

The container exists but the command inside failed. Common causes:

1. **No bash in the image**: Forjar pipes scripts to `bash` inside the container. Alpine and distroless images lack bash.

   ```bash
   # Test bash availability
   docker exec forjar-X bash -c 'echo ok'
   # If "bash: not found", use a different base image
   ```

2. **Missing coreutils**: Scripts use `cat`, `chmod`, `chown`, `mkdir`. Minimal images may lack these.

3. **Package manager not available**: Package resources assume `apt-get`, `yum`, or `dnf` depending on `provider:`. Verify the image includes the package manager.

### Ephemeral Container Cleanup

Ephemeral containers (`ephemeral: true`, the default) are created before apply and destroyed afterward. Issues with cleanup:

**Stale containers from interrupted runs:**

```bash
# List forjar containers
docker ps -a --filter name=forjar-

# Remove stale containers manually
docker rm -f forjar-test-box forjar-web-server

# Then re-run
forjar apply -f forjar.yaml --state-dir state/
```

**Cleanup fails but apply succeeded:**

Cleanup errors are non-fatal. If the container was created with a different runtime than configured (e.g., switched from docker to podman), cleanup will fail because the wrong runtime is called:

```bash
# Check which runtime the container was created with
docker inspect forjar-X 2>/dev/null && echo "docker" || podman inspect forjar-X 2>/dev/null && echo "podman"

# Remove with the correct runtime
docker rm -f forjar-X
```

**Non-ephemeral containers are never cleaned up:**

When `ephemeral: false`, forjar attaches to an existing container and never removes it. If you see "container not found" errors, ensure the named container is running before apply:

```bash
docker ps --filter name=my-persistent-container
# If not running:
docker start my-persistent-container
```

### Runtime Not Found

**"failed to start container 'forjar-X': executable file not found in $PATH"**

The `runtime:` field in your container config references a binary that does not exist:

```yaml
container:
  runtime: docker    # Must be 'docker' or 'podman'
  image: ubuntu:22.04
```

Verify:

```bash
# Check if docker is available
command -v docker

# For rootless docker
export DOCKER_HOST=unix:///run/user/$(id -u)/docker.sock
docker info

# For podman, ensure it's installed
command -v podman
```

If using a non-standard path, provide the full path:

```yaml
container:
  runtime: /usr/local/bin/podman
```

**Runtime is installed but the daemon is not running:**

```bash
# Docker
sudo systemctl start docker
sudo systemctl status docker

# Podman (daemonless — usually just works)
podman info
```

### Container Image Pull Failures

If `ensure_container` fails because the image is not available locally:

```bash
# Pull the image manually
docker pull ubuntu:22.04

# For private registries, authenticate first
docker login registry.example.com
docker pull registry.example.com/my-image:latest
```

Then re-run `forjar apply`. Forjar does not pull images automatically -- the image must be available locally before container creation.

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
