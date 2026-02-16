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

## Resources

Resources declare the desired state of infrastructure components. Each resource has a unique ID and a `type`.

```yaml
resources:
  resource-id:
    type: package|file|service|mount
    machine: machine-id       # Single machine or list
    state: ...                # Desired state (type-specific)
    depends_on: [other-id]    # Execution ordering
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

## Policy

```yaml
policy:
  failure: stop_on_first      # stop_on_first | continue_independent
  parallel_machines: false     # Concurrent machine execution (future)
  tripwire: true               # Enable provenance event logging
  lock_file: true              # Persist BLAKE3 state after apply
```

### Failure Policies

- **stop_on_first** (default): Jidoka. Stop immediately on first failure. Partial state preserved.
- **continue_independent**: Continue applying resources that don't depend on the failed one.
