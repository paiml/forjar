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

1. **ensure** -- create and start the container if not already running
2. **exec** -- pipe generated scripts to bash inside the container
3. **cleanup** -- remove the container after apply (ephemeral mode only)

Container machines are useful for:
- Local dogfooding and development without polluting the host
- CI integration testing of package, service, and mount resources
- Isolated environments that can be recreated on every run

## Machine Configuration Reference

### Machine Fields

Every machine entry supports the following fields:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `hostname` | string | required | Machine hostname (used in display output and container naming) |
| `addr` | string | required | Network address: IP, DNS name, `127.0.0.1`/`localhost` for local, or `container` for container transport |
| `user` | string | `root` | SSH user for remote connections. Ignored for local and container transport. |
| `arch` | string | `x86_64` | CPU architecture. Used for `arch:` filtering on resources. Must be one of: `x86_64`, `aarch64`, `armv7l`, `riscv64`, `s390x`, `ppc64le`. |
| `ssh_key` | string | -- | Path to SSH private key file. Supports `~` expansion. Ignored for local and container transport. |
| `roles` | [string] | [] | Informational tags for the machine. Not used in execution logic; useful for documentation and filtering. |
| `transport` | string | -- | Explicit transport override. Set to `container` for container execution. When omitted, transport is inferred: `127.0.0.1`/`localhost` uses local, everything else uses SSH. |
| `container` | object | -- | Container configuration block. Required when `transport: container`. See below. |
| `cost` | integer | 0 | Relative cost weight for scheduling order. Lower values are applied first. Useful for prioritizing cheap on-prem machines over expensive cloud instances. |

### Container Transport Fields

The `container:` block configures how forjar manages the container lifecycle. All fields have sensible defaults, so a minimal container machine only needs `image`:

```yaml
machines:
  minimal-container:
    hostname: test
    addr: container
    transport: container
    container:
      image: ubuntu:22.04
```

Full field reference:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `runtime` | string | `docker` | Container runtime executable. Must be `docker` or `podman`. Forjar calls `<runtime> run`, `<runtime> exec`, `<runtime> stop`, and `<runtime> rm` using this value. |
| `image` | string | -- | OCI image to use when creating the container. Required when `ephemeral: true`. For non-ephemeral containers that already exist, this can be omitted. |
| `name` | string | `forjar-<hostname>` | Container name passed to `--name`. When omitted, forjar generates it from the machine key as `forjar-<hostname>`. Must be unique across all container machines. |
| `ephemeral` | bool | `true` | When true, the container is destroyed (`docker rm -f`) after apply completes. When false, the container persists between runs. Ephemeral containers guarantee a clean state on every apply. |
| `privileged` | bool | `false` | When true, passes `--privileged` to `docker run`. Required for resources that need raw device access (e.g., mount resources, certain service configurations). Use sparingly -- it disables most container security isolation. |
| `init` | bool | `true` | When true, passes `--init` to `docker run`, which runs `tini` as PID 1 inside the container. This ensures proper signal forwarding and zombie process reaping. Recommended for all containers that run services. |

### Transport Inference

When `transport` is not explicitly set, forjar infers it from the `addr` field:

| `addr` Value | Inferred Transport | Mechanism |
|-------------|-------------------|-----------|
| `127.0.0.1` | local | Direct shell execution (`bash -c`) |
| `localhost` | local | Direct shell execution (`bash -c`) |
| `container` | container | `docker exec -i <name> bash` |
| anything else | SSH | `ssh -i <key> <user>@<addr> bash` |

You can override this inference by setting `transport` explicitly. For example, to SSH into localhost (useful for testing SSH transport):

```yaml
machines:
  ssh-local:
    hostname: local-via-ssh
    addr: 127.0.0.1
    transport: ssh            # Force SSH even for localhost
    ssh_key: ~/.ssh/id_ed25519
```

### Container Lifecycle Examples

**Ephemeral CI testing** -- container created fresh, destroyed after apply:

```yaml
machines:
  ci-test:
    hostname: ci-test
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      ephemeral: true
      init: true
```

**Persistent development container** -- container survives between runs:

```yaml
machines:
  dev-box:
    hostname: dev-box
    addr: container
    transport: container
    container:
      runtime: podman
      image: fedora:39
      name: forjar-dev
      ephemeral: false
      init: true
```

**Privileged container for mount testing:**

```yaml
machines:
  mount-test:
    hostname: mount-test
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      privileged: true
      init: true
```

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

## Template Resolution

### How Templates Work

Forjar resolves `{{...}}` templates in two passes during the planning phase — before any scripts are generated.

**Template types:**

| Syntax | Source | Example |
|--------|--------|---------|
| `{{params.key}}` | `params:` block | `{{params.domain}}` → `example.com` |
| `{{secrets.key}}` | Environment variable | `{{secrets.db-pass}}` → `FORJAR_SECRET_DB_PASS` |
| `{{machine.name.field}}` | Machine inventory | `{{machine.web.addr}}` → `192.168.1.10` |

### Parameter Types

Params are YAML values coerced to strings during template resolution:

```yaml
params:
  domain: example.com       # String → "example.com"
  port: 8080                 # Integer → "8080"
  debug: true                # Boolean → "true"
  ratio: 0.75                # Float → "0.75"
```

### Secret Resolution

Secrets are never stored in config files. Instead, they reference environment variables:

```yaml
resources:
  db-config:
    type: file
    path: /etc/myapp/db.conf
    content: |
      host=db.internal
      password={{secrets.db-password}}
```

At apply time, forjar looks for `FORJAR_SECRET_DB_PASSWORD` (uppercase, hyphens become underscores). If the variable is not set, the apply fails with a clear error message pointing to the expected variable name.

```bash
# Set secrets before apply
export FORJAR_SECRET_DB_PASSWORD="hunter2"
export FORJAR_SECRET_API_TOKEN="sk-live-abc123"
forjar apply -f production.yaml --state-dir state/
```

### Machine Field References

Access any machine field in templates:

```yaml
machines:
  primary-db:
    hostname: db-01
    addr: 10.0.1.50
    user: postgres
    arch: x86_64

resources:
  db-proxy-config:
    type: file
    machine: web
    path: /etc/pgbouncer/pgbouncer.ini
    content: |
      [databases]
      mydb = host={{machine.primary-db.addr}} port=5432 user={{machine.primary-db.user}}
```

Available machine fields: `hostname`, `addr`, `user`, `arch`, `ssh_key`.

### Template Error Handling

| Error | When | Message |
|-------|------|---------|
| Unknown param | `{{params.missing}}` | `unknown param: missing` |
| Missing secret | `{{secrets.key}}` without env var | `secret 'key' not found (set env var FORJAR_SECRET_KEY ...)` |
| Unknown machine | `{{machine.bogus.addr}}` | `unknown machine: bogus` |
| Invalid field | `{{machine.web.cost}}` | `unknown machine field: cost` |
| Unclosed template | `{{params.name` | `unclosed template at position N` |
| Unknown type | `{{foobar.baz}}` | `unknown template variable type: foobar` |

## Configuration Validation Pipeline

Forjar validates configs through a multi-stage pipeline before any machine is touched. The `forjar validate` command runs the first two stages. The `forjar plan` and `forjar apply` commands run all four.

```
forjar.yaml
    |
    v
[Stage 1: Parser]         YAML deserialization -> ForjarConfig struct
    |                      Fails on malformed YAML, unknown fields, wrong types
    v
[Stage 2: Validator]       Structural + semantic checks (accumulates ALL errors)
    |                      Fails on missing fields, bad references, cycles
    v
[Stage 3: Recipe Expander] Inline recipe resources, validate inputs
    |                      Fails on missing recipe files, type mismatches
    v
[Stage 4: Resolver]        Template resolution + DAG construction
    |                      Fails on missing params/secrets/machines, unclosed templates
    v
[Stage 5: Purifier]        bashrs lint on generated scripts (Invariant I8)
                           Fails on Error-severity shell diagnostics
```

### Stage 1: Parser (`parser::parse_config`)

The parser deserializes YAML into Rust structs using `serde_yaml_ng`. This stage catches:

- Malformed YAML syntax (unclosed quotes, bad indentation, tab characters)
- Unknown or misspelled top-level keys
- Type mismatches (e.g., `version: 1.0` as float instead of string `"1.0"`)
- Missing required fields in the YAML schema

```bash
$ forjar validate -f broken.yaml
YAML parse error: machines.web: missing field `hostname` at line 5 column 3
```

The parser produces a `ForjarConfig` struct containing `machines`, `resources`, `params`, and `policy` -- all strongly typed.

### Stage 2: Validator (`parser::validate_config`)

The validator performs cross-reference and constraint checks on the parsed config. It accumulates ALL errors before reporting, so you see every problem at once rather than fixing them one at a time.

**Machine validation:**

| Check | Error Example |
|-------|--------------|
| Version must be `"1.0"` | `version must be "1.0", got "2.0"` |
| Name must be non-empty | `name must not be empty` |
| Architecture must be recognized | `machine 'gpu' has invalid arch 'arm64' (expected: x86_64, aarch64, ...)` |
| Container transport requires container block | `machine 'test' has transport=container but no container config` |
| Ephemeral containers require an image | `machine 'test' container is ephemeral but has no image` |
| Container runtime must be docker or podman | `machine 'test' container runtime 'lxc' invalid (expected: docker, podman)` |

**Resource validation (per type):**

| Type | Checks |
|------|--------|
| `package` | Must have `provider`. Must have non-empty `packages` list. |
| `file` | Must have `path`. Cannot have both `content` and `source`. State must be `file`/`directory`/`symlink`/`absent`. Symlink requires `target`. |
| `service` | Must have `name`. State must be `running`/`stopped`/`enabled`/`disabled`. |
| `mount` | Must have `source` and `path`. State must be `mounted`/`unmounted`/`absent`. |
| `user` | Must have `name`. State must be `present`/`absent`. |
| `docker` | Must have `name`. Must have `image` (unless state=absent). State must be `running`/`stopped`/`absent`. |
| `cron` | Must have `name`, `schedule`, `command` (unless state=absent). Schedule must have exactly 5 fields. State must be `present`/`absent`. |
| `network` | Must have `port`. Protocol must be `tcp`/`udp`. Action must be `allow`/`deny`/`reject`. State must be `present`/`absent`. |

**Dependency validation:**

| Check | Error |
|-------|-------|
| Resource references a valid machine | `resource 'X' references unknown machine 'Y'` |
| `depends_on` targets exist | `resource 'X' depends on unknown resource 'Y'` |
| No self-dependencies | `resource 'X' depends on itself` |
| No cycles | `dependency cycle detected involving: A, B, C` |

All errors are collected into a list and returned together:

```bash
$ forjar validate -f broken.yaml
validation errors:
  - resource 'web-pkg' (package) has no packages
  - resource 'web-pkg' (package) has no provider
  - resource 'nginx-conf' references unknown machine 'web-server'
  - resource 'backup' (cron) schedule '0 2 *' must have exactly 5 fields
```

### Stage 3: Recipe Expander (`parser::expand_recipes`)

If any resource has `type: recipe`, the expander loads the referenced recipe file, validates the provided inputs against the recipe's input schema, and inlines the expanded resources into the config. This stage catches:

- Missing recipe file on disk
- Required recipe inputs not provided
- Input type mismatches (string where int expected)
- Integer inputs outside declared min/max bounds
- Enum inputs not in the declared choices list
- Path inputs that are not absolute

### Stage 4: Resolver (`resolver::resolve_templates` + `resolver::build_execution_order`)

The resolver performs two tasks:

**Template resolution** -- replaces `{{params.X}}`, `{{secrets.X}}`, and `{{machine.NAME.FIELD}}` with concrete values. Errors at this stage:

| Error | Cause |
|-------|-------|
| `unknown param: X` | `{{params.X}}` references a key not in the `params:` block |
| `secret 'X' not found (set env var FORJAR_SECRET_X)` | `{{secrets.X}}` but the environment variable is not set |
| `unknown machine: X` | `{{machine.X.addr}}` references a machine not in the inventory |
| `unknown machine field: cost` | `{{machine.web.cost}}` references a field not available for templating |
| `unclosed template at position N` | `{{params.name` without closing `}}` |

**DAG construction** -- builds a Directed Acyclic Graph from `depends_on` edges and computes a topological sort using Kahn's algorithm with alphabetical tie-breaking for deterministic execution order.

### Stage 5: Purifier (`purifier::validate_script`)

After codegen produces check, apply, and state_query scripts for each resource, the purifier validates them through the bashrs linter. Scripts must pass with zero Error-severity diagnostics. Warning-level findings (such as the `$SUDO` unquoted variable pattern in package/user/cron/network handlers) are acceptable and do not block execution.

This stage is Invariant I8: no raw shell execution. All generated shell is bashrs-validated before being piped to any transport.

### Running Validation

```bash
# Validate only (stages 1-3, no secrets needed)
forjar validate -f forjar.yaml

# Plan (stages 1-5, secrets needed if templates reference them)
forjar plan -f forjar.yaml --state-dir state/

# Both report errors and exit non-zero on failure
```

On success, `forjar validate` prints:

```
OK: production-stack (2 machines, 12 resources)
```

### Common Validation Errors

```
error: resource 'nginx-conf' depends on unknown 'nginx-package'
  hint: available resources: nginx-pkg, ssl-cert, firewall

error: dependency cycle detected involving: a, b, c
  hint: resource 'a' depends_on 'b', 'b' depends_on 'c', 'c' depends_on 'a'

error: resource 'db-config' references unknown machine 'database'
  hint: available machines: web, db, cache
```

## Multi-File Configuration

### Splitting Large Configs

For large infrastructures, split configs by concern:

```bash
# Apply multiple config files
forjar apply -f machines.yaml -f web-resources.yaml -f db-resources.yaml

# Or use shell glob
forjar apply -f configs/*.yaml
```

Each file is parsed and merged. Machines from all files form a unified inventory, and resources from all files are combined into a single execution plan.

### Config Organization Patterns

**By machine role:**
```
configs/
  machines.yaml          # All machine definitions
  web.yaml               # Web server resources
  database.yaml          # Database resources
  monitoring.yaml        # Monitoring resources
```

**By environment:**
```
configs/
  base.yaml              # Shared resources
  staging.yaml           # Staging overrides
  production.yaml        # Production machines + resources
```

**By team:**
```
configs/
  infra/machines.yaml    # Platform team owns machines
  app/web-deploy.yaml    # App team owns deployments
  security/firewall.yaml # Security team owns rules
```
