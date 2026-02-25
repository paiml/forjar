# Configuration Reference

The `forjar.yaml` file is the single source of truth for your infrastructure.

## Top-Level Schema

```yaml
version: "1.0"           # Required. Must be "1.0"
name: my-infra            # Required. Human-readable name
description: "..."        # Optional

params:                   # Optional. Global parameters for templates
  key: value

machines:                 # Required. Machine inventory
  machine-id:
    hostname: ...
    addr: ...

resources:                # Required. Infrastructure resources
  resource-id:
    type: ...

policy:                   # Optional. Execution policy
  failure: stop_on_first
  tripwire: true
  lock_file: true
```

## Machines

Each machine entry defines a target host:

```yaml
machines:
  gpu-box:
    hostname: lambda          # Required. Machine hostname
    addr: 192.168.50.100      # Required. IP or DNS name
    user: noah                # Optional. Default: root
    arch: x86_64              # Optional. Default: x86_64
    ssh_key: ~/.ssh/id_ed25519  # Optional. SSH private key path
    roles: [gpu-compute]      # Optional. Informational tags
```

### Local Machine

Use `addr: 127.0.0.1` or `addr: localhost` to target the local machine (no SSH):

```yaml
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
```

### Container Machine

Use `transport: container` to execute inside a Docker or Podman container:

```yaml
machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker       # docker | podman (default: docker)
      image: ubuntu:22.04   # Required for ephemeral containers
      name: forjar-test     # Auto-generated from key if omitted
      ephemeral: true       # Destroy after apply (default: true)
      privileged: false     # --privileged flag (default: false)
      init: true            # --init for PID 1 reaping (default: true)
```

Container transport uses the same stdin-pipe mechanism as local and SSH (`docker exec -i <name> bash`). The container lifecycle is managed automatically:

1. **ensure** — create and start the container if not already running
2. **exec** — pipe generated scripts to bash inside the container
3. **cleanup** — remove the container after apply (ephemeral mode only)

Container machines are useful for:
- Local dogfooding and development without polluting the host
- CI integration testing of package, service, and mount resources
- Isolated environments that can be recreated on every run

## Resources

Resources declare the desired state of infrastructure components. Each resource has a unique ID and a `type`.

```yaml
resources:
  resource-id:
    type: package|file|service|mount|user|docker|cron|network
    machine: machine-id       # Single machine or list
    state: ...                # Desired state (type-specific)
    depends_on: [other-id]    # Execution ordering
    arch: [x86_64, aarch64]   # Optional. Only apply to matching architectures
```

### Machine Targeting

Target a single machine or multiple:

```yaml
# Single
machine: gpu-box

# Multiple
machine: [gpu-box, edge-node]
```

### Dependencies

Use `depends_on` to enforce execution order:

```yaml
resources:
  packages:
    type: package
    machine: m1
    provider: apt
    packages: [nginx]

  config:
    type: file
    machine: m1
    path: /etc/nginx/nginx.conf
    content: "..."
    depends_on: [packages]

  service:
    type: service
    machine: m1
    name: nginx
    state: running
    depends_on: [config]
```

Forjar builds a DAG from dependencies and executes in topological order (Kahn's algorithm with alphabetical tie-breaking for determinism).

### Architecture Filtering

Resources can be restricted to specific CPU architectures using the `arch` field:

```yaml
resources:
  gpu-driver:
    type: package
    machine: [x86-box, arm-box]
    provider: apt
    packages: [nvidia-driver]
    arch: [x86_64]          # Only install on x86_64 machines

  arm-firmware:
    type: file
    machine: [x86-box, arm-box]
    path: /etc/arm-firmware.conf
    content: "..."
    arch: [aarch64]         # Only deploy on ARM machines
```

When `arch` is omitted or empty, the resource applies to all architectures. When specified, forjar skips the resource during both `plan` and `apply` if the target machine's `arch` doesn't match.

Recognized architectures: `x86_64`, `aarch64`, `armv7l`, `riscv64`, `s390x`, `ppc64le`.

## Parameters

Global parameters can be referenced in any string field:

```yaml
params:
  env: production
  data_dir: /mnt/data

resources:
  data-dir:
    type: file
    state: directory
    path: "{{params.data_dir}}"
    machine: m1

  config:
    type: file
    machine: m1
    path: /etc/app/env
    content: "ENVIRONMENT={{params.env}}"
```

## Secrets

Secrets are resolved from environment variables at apply time. Use `{{secrets.KEY}}` in any string field:

```yaml
resources:
  db-config:
    type: file
    machine: m1
    path: /etc/app/database.conf
    content: |
      host=db.internal
      password={{secrets.db-password}}
    mode: "0600"
```

The secret key is normalized to an environment variable: `{{secrets.db-password}}` reads from `FORJAR_SECRET_DB_PASSWORD` (uppercase, hyphens become underscores).

```bash
# Set secrets before apply
export FORJAR_SECRET_DB_PASSWORD="hunter2"
export FORJAR_SECRET_API_KEY="sk-live-abc123"

forjar apply -f forjar.yaml --state-dir state/
```

If a secret is missing, forjar exits with a clear error message naming the expected environment variable.

Secrets are never written to forjar.yaml, state files, or git. They exist only in memory during apply.

## Policy

```yaml
policy:
  failure: stop_on_first      # stop_on_first | continue_independent
  parallel_machines: false     # Concurrent machine execution (future)
  tripwire: true               # Enable provenance event logging
  lock_file: true              # Persist BLAKE3 state after apply
  pre_apply: "echo 'validating...' && ./scripts/check-env.sh"
  post_apply: "echo 'done!' && ./scripts/notify-slack.sh"
```

### Failure Policies

- **stop_on_first** (default): Jidoka. Stop immediately on first failure. Partial state preserved.
- **continue_independent**: Continue applying resources that don't depend on the failed one.

### Apply Hooks

Run local shell commands before and after apply:

```yaml
policy:
  pre_apply: "echo 'Pre-flight check' && ./validate.sh"
  post_apply: "echo 'Apply complete' && date"
```

- **pre_apply**: Runs before any resources are applied. If the command exits non-zero, the apply is **aborted**. Use for pre-flight validation, environment checks, or approval gates.
- **post_apply**: Runs after a successful apply. Informational only — a non-zero exit logs a warning but does not change the apply's exit code. Use for notifications, logging, or cleanup.

Hooks are skipped during `--dry-run`.
