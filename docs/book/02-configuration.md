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
    cost: 10                  # Optional. Cost weight (default: 0, lower = preferred)
```

### Cost-Aware Scheduling

Machines with a lower `cost` value are applied first. This is useful when you have a mix of cheap on-prem machines and expensive cloud instances — forjar will converge cheaper machines first:

```yaml
machines:
  on-prem:
    hostname: rack-01
    addr: 192.168.1.10
    cost: 1               # cheap, runs first
  cloud-gpu:
    hostname: gpu-instance
    addr: 10.0.0.50
    cost: 100             # expensive, runs last
```

If `cost` is omitted, it defaults to 0. Machines with equal cost maintain their original order.

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
    tags: [web, critical]     # Optional. Labels for selective filtering
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

### Resource Tags

Tags allow selective filtering of resources during `plan`, `apply`, and `check`:

```yaml
resources:
  web-config:
    type: file
    machine: web-server
    path: /etc/nginx/nginx.conf
    content: "..."
    tags: [web, critical]

  db-config:
    type: file
    machine: db-server
    path: /etc/postgres/pg.conf
    content: "..."
    tags: [db, critical]
```

Use `--tag` on the CLI to filter:

```bash
# Only apply resources tagged "web"
forjar apply -f forjar.yaml --tag web

# Plan only critical resources
forjar plan -f forjar.yaml --tag critical

# Check only db resources
forjar check -f forjar.yaml --tag db
```

When `--tag` is omitted, all resources are included. When specified, only resources with that tag are planned/applied/checked. Resources without any tags are excluded when `--tag` is used.

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

## Cross-Machine References

Use `{{machine.NAME.FIELD}}` to reference properties of other machines in templates:

```yaml
params:
  app_port: "8080"

machines:
  db:
    hostname: db-primary
    addr: 10.0.0.5
  web:
    hostname: web-frontend
    addr: 10.0.0.10

resources:
  db-config:
    type: file
    machine: web
    path: /etc/app/database.conf
    content: |
      db_host={{machine.db.addr}}
      db_hostname={{machine.db.hostname}}
```

Available machine fields: `addr`, `hostname`, `user`, `arch`.

## Template Syntax Reference

| Syntax | Source | Example | Resolved Value |
|--------|--------|---------|----------------|
| `{{params.X}}` | `params:` block | `{{params.env}}` | `production` |
| `{{secrets.X}}` | `FORJAR_SECRET_*` env vars | `{{secrets.db-pass}}` | env value |
| `{{machine.NAME.FIELD}}` | Machine properties | `{{machine.db.addr}}` | `10.0.0.5` |

Templates are resolved in all string fields: `content`, `path`, `source`, `target`, `owner`, `group`, `mode`, `name`, `options`, `command`, `schedule`, `port`, `protocol`, `action`, `from_addr`, `image`, `shell`, `home`, `restart`, `version`. List fields are also resolved: `ports`, `environment`, `volumes`, `packages`.

Unresolved templates (no matching param/secret/machine) pass through unchanged — they are not treated as errors.

## Validation Rules

`forjar validate` checks your config for errors before apply. These rules are enforced:

### Machine Validation

| Rule | Error If Violated |
|------|-------------------|
| Valid architecture | `arch` must be one of: `x86_64`, `aarch64`, `armv7l`, `riscv64`, `s390x`, `ppc64le` |
| Container transport | `transport: container` requires a `container:` block |
| Ephemeral image | `ephemeral: true` requires `container.image` |
| Container runtime | `container.runtime` must be `docker` or `podman` |

### Resource Validation

| Type | Required Fields | Additional Rules |
|------|----------------|-----------------|
| `package` | `provider`, `packages` (non-empty) | — |
| `file` | `path` | Cannot have both `content` and `source`. State must be `file`/`directory`/`symlink`/`absent`. Symlink requires `target`. |
| `service` | `name` | State must be `running`/`stopped`/`enabled`/`disabled`. |
| `mount` | `source`, `path` | State must be `mounted`/`unmounted`/`absent`. |
| `user` | `name` | State must be `present`/`absent`. |
| `docker` | `name`, `image` (unless absent) | State must be `running`/`stopped`/`absent`. |
| `cron` | `name`, `schedule`, `command` (unless absent) | Schedule must have exactly 5 fields (min hour dom mon dow). State must be `present`/`absent`. |
| `network` | `port` | State must be `present`/`absent`. Protocol must be `tcp`/`udp`. Action must be `allow`/`deny`/`reject`. |

### Dependency Validation

| Rule | Error |
|------|-------|
| Unknown machine | Resource references a machine not in `machines:` |
| Unknown dependency | `depends_on` references a resource that doesn't exist |
| Self-dependency | Resource depends on itself |
| Circular dependency | Cycle detected in dependency graph (e.g., A → B → C → A) |

## Complete Example

A full config exercising multiple features:

```yaml
version: "1.0"
name: production-stack
description: "Web + DB servers with monitoring"

params:
  env: production
  app_port: "8080"

machines:
  web:
    hostname: web-01
    addr: 10.0.0.10
    ssh_key: ~/.ssh/deploy_ed25519
    roles: [frontend]
    cost: 1
  db:
    hostname: db-01
    addr: 10.0.0.20
    ssh_key: ~/.ssh/deploy_ed25519
    roles: [database]
    cost: 1

resources:
  base-packages:
    type: package
    machine: [web, db]
    provider: apt
    packages: [curl, htop, jq]
    tags: [base]

  app-config:
    type: file
    machine: web
    path: /etc/app/config.env
    content: |
      ENVIRONMENT={{params.env}}
      PORT={{params.app_port}}
      DB_HOST={{machine.db.addr}}
      DB_PASSWORD={{secrets.db-password}}
    mode: "0600"
    owner: www-data
    depends_on: [base-packages]
    tags: [web, critical]

  nginx:
    type: service
    machine: web
    name: nginx
    state: running
    restart_on: [app-config]
    depends_on: [app-config]
    tags: [web]

  firewall:
    type: network
    machine: web
    port: "{{params.app_port}}"
    protocol: tcp
    action: allow
    name: app-http
    tags: [web, security]

policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
  pre_apply: "echo 'Deploying {{params.env}}...'"
  post_apply: "echo 'Deploy complete at $(date)'"
```

## Configuration Validation

Forjar validates configs at multiple levels before any machine is touched.

### Structural Validation

These checks run during `forjar validate`:

| Check | Example Error |
|-------|--------------|
| Version must be "1.0" | `version must be "1.0"` |
| Resource references valid machine | `resource 'X' references unknown machine 'Y'` |
| Dependencies reference valid resources | `resource 'X' depends on unknown resource 'Y'` |
| No self-dependencies | `resource 'X' depends on itself` |
| No circular dependencies | `dependency cycle detected involving: A, B, C` |
| Package has provider | `resource 'X' (package) has no provider` |
| Package has packages list | `resource 'X' (package) has no packages` |
| File doesn't have both content and source | `resource 'X' (file) has both content and source` |
| Symlink has target | `resource 'X' (file) state=symlink requires a target` |
| Service has valid state | `resource 'X' (service) state 'Y' invalid` |
| Mount has valid state | `resource 'X' (mount) state 'Y' invalid` |
| Cron has schedule | `resource 'X' (cron) missing schedule` |
| Network has valid protocol | `resource 'X' (network) protocol must be tcp or udp` |

### Error Accumulation

Validation collects ALL errors before reporting — it doesn't stop at the first error:

```bash
$ forjar validate -f broken.yaml
validation errors:
  - resource 'web-pkg' (package) has no packages
  - resource 'web-pkg' (package) has no provider
  - resource 'nginx-conf' references unknown machine 'web-server'
  - resource 'backup' (cron) schedule '0 2 *' must have exactly 5 fields
```

This gives you a complete picture of what needs fixing, rather than a whack-a-mole experience.

### Recipe Validation

When recipes are present, additional validation occurs:

| Check | Example Error |
|-------|--------------|
| Recipe file exists | `recipe file not found: recipes/X.yaml` |
| Required inputs provided | `recipe 'X' input 'Y' is required but not provided` |
| Input types match | `recipe 'X' input 'Y' expected int, got string` |
| Int inputs within bounds | `recipe 'X' input 'Y' value 70000 exceeds max 65535` |
| Enum inputs in choices | `recipe 'X' input 'Y' must be one of [a, b, c]` |
| Path inputs absolute | `recipe 'X' input 'Y' path must be absolute` |

## Minimal Configuration

The smallest valid config:

```yaml
version: "1.0"
name: minimal
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  test:
    type: file
    machine: m
    path: /tmp/test.txt
    content: "hello"
```

Or using the implicit localhost (no machines block needed):

```yaml
version: "1.0"
name: minimal
machines: {}
resources:
  test:
    type: file
    machine: localhost
    path: /tmp/test.txt
    content: "hello"
```

## Configuration Anti-Patterns

### Avoid: Hardcoded Secrets

```yaml
# BAD — secrets in plain text
resources:
  db-config:
    content: "password=hunter2"

# GOOD — use secret references
resources:
  db-config:
    content: "password={{secrets.db-password}}"
```

### Avoid: Implicit Dependencies

```yaml
# BAD — config file needs nginx installed, but no depends_on
resources:
  nginx-pkg:
    type: package
    packages: [nginx]
  nginx-conf:
    type: file
    path: /etc/nginx/nginx.conf
    content: "..."
    # Missing: depends_on: [nginx-pkg]
```

Without `depends_on`, alphabetical tie-breaking determines order — `nginx-conf` runs before `nginx-pkg` because "c" < "p". The config file write would fail if the directory doesn't exist yet.

### Avoid: Overly Broad Multi-Machine Targeting

```yaml
# BAD — all machines get the same config
resources:
  config:
    type: file
    machine: [web, db, cache, monitor]
    content: "..."

# GOOD — use recipes for machine-specific configs
resources:
  web-stack:
    type: recipe
    machine: web
    recipe: web-server
  db-stack:
    type: recipe
    machine: db
    recipe: database
```
