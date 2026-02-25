# Forjar — Rust-Native Infrastructure as Code

**Version**: 0.6.0-spec
**Status**: Active
**Author**: Noah Gift / Pragmatic AI Labs
**Date**: 2026-02-25

---

## 1. Vision

Forjar is a Rust-native Infrastructure as Code tool that is faster, more provable, and more sovereign than Terraform, Pulumi, or Ansible. It generates provably safe shell from Rust — not the other way around. Every apply is traced, every file is hashed, every change is auditable back to the git commit that caused it.

### 1.1 Core Thesis

Terraform uses HCL to describe state and shells out to cloud provider APIs. Pulumi wraps cloud SDKs in general-purpose languages. Both treat the machine as a black box behind an API.

Forjar treats the machine as a **knowable system**. It uses Rust to generate provisioning code, bashrs to purify it into provably safe POSIX shell, BLAKE3 to fingerprint everything, and renacer to trace every syscall. State lives in git. There is no remote backend, no cloud SDK, no runtime. Just SSH, purified shell, and cryptographic proof.

### 1.2 Design Principles

| Principle | Meaning |
|-----------|---------|
| **Sovereign** | Zero dependency on external services. State in git. Stack crates only. |
| **Provable** | Every shell command is generated from Rust and purified by bashrs. No raw `sh -c`. |
| **Auditable** | Every apply produces a renacer syscall trace and BLAKE3 state snapshot. Tripwire built in. |
| **Fast** | Rust binary, BLAKE3 diffing in microseconds, parallel SSH, copia delta sync. |
| **Bare-metal first** | Manages real machines over SSH. Containers serve as execution targets (transport) and managed resources (docker type). Pepita kernel isolation is Phase 3. |
| **Ephemeral** | Any machine can be destroyed and rebuilt from the repo alone. |
| **Jidoka** | Stop on first failure. Partial state is preserved. No cascading damage. |

### 1.3 Competitive Position

| Feature | Terraform | Pulumi | Ansible | **Forjar** |
|---------|-----------|--------|---------|-----------|
| Language | HCL | Python/TS/Go | YAML | **YAML + Rust codegen** |
| Runtime | Go binary | Node/Python + Go | Python + SSH | **Single Rust binary** |
| State | S3/Consul/local JSON | S3/Consul/SaaS | None | **Git (BLAKE3 YAML)** |
| Provenance | None | None | None | **renacer syscall trace** |
| Drift detection | `terraform plan` (API calls) | `pulumi preview` (API calls) | None | **BLAKE3 content hash (local, instant)** |
| Shell safety | None | None | None | **bashrs purification** |
| Speed | Seconds-minutes | Seconds-minutes | Minutes | **Milliseconds-seconds** |
| Bare metal | Weak (provisioner hacks) | Weak | Strong | **First-class** |
| External deps | ~200 Go modules | ~500 npm/pip packages | ~50 Python packages | **< 5 crates** |

---

## 2. Architecture

### 2.1 Data Flow

```
forjar.yaml                    (human-authored desired state)
       │
       ▼
┌─────────────┐
│   parser    │                parse + validate YAML
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   recipe    │                load recipes, validate inputs, expand into resources
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  resolver   │                resolve templates, compute dependency DAG
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   planner   │                diff desired vs current state → execution plan
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  codegen    │                generate Rust provisioning AST per resource
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   bashrs    │                transpile Rust AST → purified POSIX shell (FJ-036: pending)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  executor   │                transport dispatch + BLAKE3 snapshot
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   state     │                update lock files, commit to git
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  tripwire   │                append provenance event log
└─────────────┘
```

### 2.2 Module Structure

```
src/
  main.rs               CLI entry point
  lib.rs                Public API
  build.rs              Compile-time contract binding verification
  cli/
    mod.rs              Subcommand dispatch (init, validate, plan, apply, drift, status, history, destroy, import, show, graph, check, diff, fmt, lint, rollback, anomaly)
  core/
    mod.rs              Re-exports
    types.rs            All types (Machine, Resource, State, Lock)
    parser.rs           YAML parsing + validation
    resolver.rs         Template resolution, dependency DAG
    planner.rs          Desired vs current state diffing
    codegen.rs          Script generation — dispatch to resource handlers
    executor.rs         Orchestration loop, Jidoka policy
    state.rs            Lock file management (BLAKE3 content-addressed)
    recipe.rs           Recipe loading, input validation, expansion into resources
  tripwire/
    mod.rs              Provenance tracing orchestration
    hasher.rs           BLAKE3 file/directory/state hashing
    drift.rs            Drift detection (hash current vs lock)
    eventlog.rs         Append-only JSONL provenance log
    tracer.rs           renacer integration — syscall capture per apply (Phase 4)
    snapshot.rs         Pre/post apply filesystem snapshots (Phase 4)
  resources/
    mod.rs              Resource type registry
    package.rs          apt/cargo/uv package management
    service.rs          systemd service management
    file.rs             File/directory/symlink management
    mount.rs            NFS/bind mount management
    user.rs             User/group management
    docker.rs           Container resource management
    network.rs          Firewall rule management (ufw)
    cron.rs             Scheduled task management (crontab)
    pepita.rs           Kernel namespace isolation (Phase 3)
  transport/
    mod.rs              Transport abstraction + dispatch
    local.rs            Local execution (this machine)
    ssh.rs              SSH execution (remote machines)
    container.rs        Container execution (docker/podman exec)
```

### 2.3 Dependency Policy

**Stack crates only. External deps are exceptions requiring justification.**

| Dependency | Source | Justification |
|-----------|--------|---------------|
| `blake3` | External | No stack alternative. Pure Rust, no C deps, audited. **Integrated.** |
| `serde` | External | Foundational serialization. No alternative. **Integrated.** |
| `serde_yaml_ng` | External | YAML parsing (`serde_yaml` successor, actively maintained). **Integrated.** |
| `serde_json` | External | JSON serialization for event logs. **Integrated.** |
| `indexmap` | External | Insertion-ordered maps for deterministic lock file output. **Integrated.** |
| `clap` | External | CLI argument parsing with derive macros. **Integrated.** |
| `base64` | External | Source file transfer encoding (FJ-035). **Integrated.** |
| `provable-contracts` | Stack | Formal invariant verification — compile-time contract enforcement. **Integrated.** |
| `provable-contracts-macros` | Stack | `#[contract]` proc macro for function-level binding. **Integrated.** |
| `bashrs` | Stack | Shell purification — core to the provability thesis. *Phase 2, pending.* |
| `pepita` | Stack | Kernel interfaces. *Phase 3, pending.* |
| `renacer` | Stack | Syscall tracing for provenance. *Phase 4, pending.* |
| `aprender` | Stack | ML-based drift anomaly detection. *Phase 4, pending.* |

**Banned**: tokio (use std threads + ssh binary), reqwest, hyper, tonic, any cloud SDK.

**Deferred**: copia (delta sync — base64 sufficient for now), repartir (parallel dispatch — std::thread::scope sufficient), duende (daemon management — not needed), pmat (compliance gates — external tool).

---

## 3. Desired State Schema (`forjar.yaml`)

### 3.1 Top-Level Structure

```yaml
version: "1.0"
name: sovereign-lab
description: "Noah's sovereign AI lab infrastructure"

# Global parameters (templatable)
params:
  raid_path: /mnt/nvme-raid0
  stack_version: "0.6.5"

# Machine inventory
machines:
  lambda:
    hostname: noah-Lambda-Vector
    addr: 192.168.50.50
    user: noah
    arch: x86_64
    ssh_key: ~/.ssh/id_ed25519
    roles:
      - gpu-compute
      - nfs-server
      - raid-storage

  intel:
    hostname: mac-server
    addr: 192.168.50.100
    user: noah
    arch: x86_64
    ssh_key: ~/.ssh/id_ed25519
    roles:
      - cpu-compute
      - nfs-client

  jetson:
    hostname: jetson
    addr: 192.168.55.1
    user: nvidia
    arch: aarch64
    ssh_key: ~/.ssh/id_ed25519
    roles:
      - edge-inference

  # Container execution target (for testing/CI)
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      name: forjar-test
      ephemeral: true
      privileged: false
      init: true

# Resource declarations
resources:
  # ── Packages ──────────────────────────────────────
  nfs-server-pkg:
    type: package
    machine: lambda
    provider: apt
    packages:
      - nfs-kernel-server
      - nfs-common

  nfs-client-pkg:
    type: package
    machine: intel
    provider: apt
    packages:
      - nfs-common

  stack-tools:
    type: package
    machine: [intel, jetson]
    provider: cargo
    packages:
      - batuta
      - whisper-apr

  # ── Files ─────────────────────────────────────────
  nfs-exports:
    type: file
    machine: lambda
    path: /etc/exports
    content: |
      {{params.raid_path}} 192.168.50.100(ro,sync,no_subtree_check,no_root_squash)
    owner: root
    group: root
    mode: "0644"
    depends_on:
      - nfs-server-pkg

  # ── Services ──────────────────────────────────────
  nfs-server-svc:
    type: service
    machine: lambda
    name: nfs-kernel-server
    state: running
    enabled: true
    depends_on:
      - nfs-exports

  # ── Mounts ────────────────────────────────────────
  raid-mount:
    type: mount
    machine: intel
    source: "192.168.50.50:{{params.raid_path}}"
    path: /mnt/lambda-raid
    fstype: nfs
    options: ro,hard,intr
    depends_on:
      - nfs-server-svc
      - nfs-client-pkg

  # ── Directories ───────────────────────────────────
  transcripts-dir:
    type: file
    machine: intel
    path: /data/transcripts
    state: directory
    owner: noah
    mode: "0755"

# Execution policy
policy:
  failure: stop_on_first       # Jidoka
  parallel_machines: true      # Apply to independent machines concurrently
  tripwire: true               # Enable provenance tracing on every apply
  lock_file: true              # Persist BLAKE3 state after apply
```

### 3.2 Resource Types

#### `package`

```yaml
type: package
machine: <name | [names]>
provider: apt | cargo | uv
packages: [list]
state: present | absent          # default: present
version: <optional constraint>
```

#### `file`

```yaml
type: file
machine: <name>
path: /absolute/path
state: file | directory | absent | symlink
content: <inline string>          # for state=file
source: <local path>              # alternative to content (base64 transfer via FJ-035)
target: <symlink target>          # for state=symlink
owner: <user>
group: <group>
mode: "0644"
```

#### `service`

```yaml
type: service
machine: <name>
name: <systemd unit>
state: running | stopped
enabled: true | false
restart_on:                       # restart when these resources change
  - <resource_id>
```

#### `mount`

```yaml
type: mount
machine: <name>
source: <device or nfs path>
path: /mount/point
fstype: nfs | ext4 | xfs | bind
options: <mount options string>
state: mounted | unmounted | absent
```

#### `user`

```yaml
type: user
machine: <name>
name: <username>
state: present | absent
groups: [list]
shell: /bin/bash
home: /home/<user>
ssh_authorized_keys:
  - <key string>
```

#### `docker`

```yaml
type: docker
machine: <name>
name: <container name>
image: <image:tag>
state: running | stopped | absent
ports: ["8080:80"]
volumes: ["/host:/container"]
environment: ["KEY=value"]
restart: unless-stopped     # Docker restart policy
command: <optional command>
```

#### `cron`

```yaml
type: cron
machine: <name>
name: <job identifier>      # Used as forjar tag in crontab
state: present | absent
schedule: "0 * * * *"       # Standard cron expression
command: /usr/local/bin/backup.sh
owner: root                  # Crontab user (default: root)
```

#### `network`

```yaml
type: network
machine: <name>
name: <rule comment>         # Optional ufw rule comment
state: present | absent
port: "22"                   # Port number or range
protocol: tcp                # tcp | udp (default: tcp)
action: allow                # allow | deny
from: 192.168.1.0/24        # Optional source CIDR
```

#### `pepita` (Phase 3)

```yaml
type: pepita
machine: <name>
name: <namespace name>
state: present | absent
isolation:
  network: isolated | host
  filesystem: overlay | bind
  cgroups:
    memory_mb: 4096
    cpus: 4
```

### 3.3 Recipes

Recipes are reusable, parameterized infrastructure patterns — the forjar equivalent of Nix flakes, Homebrew formulae, or Ansible roles. A recipe bundles resources into a sharable, composable unit with typed inputs.

#### 3.3.1 Design Philosophy

| Decision | Forjar | Nix | Homebrew | Why |
|----------|--------|-----|----------|-----|
| Recipe language | **YAML** | Custom functional lang | Ruby DSL | Zero learning curve |
| Composition | `include` + inputs | Flake inputs + overlays | Taps | Simple, sufficient |
| Sharing | Git repos | Flake registries | Taps (git repos) | Git-native, sovereign |
| Parameterization | Typed YAML inputs | Module options | Formula DSL | Declarative, no code |
| Isolation | None (bare-metal) | `/nix/store/<hash>` | `/usr/local/Cellar` | Trust the OS |
| Rollback | `git revert` | Atomic generations | None | State lives in git |

#### 3.3.2 Recipe File Format

A recipe is a standalone YAML file declaring inputs and resources:

```yaml
# recipes/nfs-server.yaml
recipe:
  name: nfs-server
  version: "1.0"
  description: "NFS server with exports and firewall"
  author: "Pragmatic AI Labs"
  license: "MIT"

  inputs:
    export_path:
      type: string
      description: "Directory to export via NFS"
    allowed_network:
      type: string
      default: "192.168.50.0/24"
      description: "Network CIDR allowed to mount"
    options:
      type: string
      default: "rw,sync,no_subtree_check"
      description: "NFS export options"
    read_only:
      type: bool
      default: false

  # Recipes this recipe depends on (resolved first)
  requires: []

resources:
  nfs-packages:
    type: package
    provider: apt
    packages: [nfs-kernel-server, nfs-common]

  export-dir:
    type: file
    state: directory
    path: "{{inputs.export_path}}"
    owner: root
    mode: "0755"

  exports-file:
    type: file
    path: /etc/exports
    content: |
      {{inputs.export_path}} {{inputs.allowed_network}}({{inputs.options}})
    depends_on: [export-dir]

  nfs-service:
    type: service
    name: nfs-kernel-server
    state: running
    enabled: true
    restart_on: [exports-file]
    depends_on: [nfs-packages, exports-file]
```

#### 3.3.3 Recipe Input Types

| Type | YAML Value | Validation |
|------|------------|------------|
| `string` | Any string | Non-empty if required |
| `int` | Integer | Optional min/max |
| `bool` | true/false | Must be boolean |
| `list` | YAML sequence | Optional min/max length |
| `path` | String | Must start with `/` |
| `enum` | String | Must be one of `choices` |

Example with constraints:

```yaml
inputs:
  port:
    type: int
    default: 8080
    min: 1024
    max: 65535
  protocol:
    type: enum
    choices: [tcp, udp]
    default: tcp
  mount_points:
    type: list
    min_length: 1
    description: "Paths to mount"
```

#### 3.3.4 Recipe Sources

Recipes can be loaded from three sources:

```yaml
# In forjar.yaml
recipes:
  # 1. Local path (relative to forjar.yaml)
  - path: recipes/nfs-server.yaml

  # 2. Local directory (all .yaml files in directory)
  - path: recipes/

  # 3. Git repository (cloned to .forjar/recipes/<name>/)
  - git: github.com/paiml/forjar-recipes
    ref: v1.0.0                    # tag, branch, or commit SHA
    path: recipes/                 # subdirectory within the repo
```

Git-based recipe sources are analogous to Homebrew taps:
- `brew tap paiml/tools` → `git: github.com/paiml/forjar-recipes`
- `brew install batuta` → `recipe: nfs-server` with inputs
- Recipes are cached locally in `.forjar/recipes/` and pinned by SHA in the lock file

#### 3.3.5 Using Recipes in forjar.yaml

A recipe is instantiated as a resource of `type: recipe`:

```yaml
resources:
  raid-nfs:
    type: recipe
    recipe: nfs-server
    machine: lambda
    inputs:
      export_path: /mnt/nvme-raid0
      allowed_network: "192.168.50.0/24"
    depends_on: [some-other-resource]

  # Regular resources alongside recipes
  tools:
    type: package
    machine: lambda
    provider: cargo
    packages: [batuta]
    depends_on: [raid-nfs]
```

When the config is loaded, recipe resources are **expanded** into their constituent resources with namespaced IDs:

```
raid-nfs                    →  raid-nfs/nfs-packages
                               raid-nfs/export-dir
                               raid-nfs/exports-file
                               raid-nfs/nfs-service
```

The `machine` target from the recipe resource propagates to all inner resources. External `depends_on` references to the recipe ID (`raid-nfs`) become dependencies on the recipe's **last** resource in topo order.

#### 3.3.6 Recipe Composition

Recipes can require other recipes:

```yaml
# recipes/nfs-client-mount.yaml
recipe:
  name: nfs-client-mount
  version: "1.0"
  inputs:
    server_addr:
      type: string
    remote_path:
      type: path
    local_path:
      type: path
    mount_options:
      type: string
      default: "ro,hard,intr"
  requires:
    - recipe: nfs-client-pkg

resources:
  mount-point:
    type: file
    state: directory
    path: "{{inputs.local_path}}"

  nfs-mount:
    type: mount
    source: "{{inputs.server_addr}}:{{inputs.remote_path}}"
    path: "{{inputs.local_path}}"
    fstype: nfs
    options: "{{inputs.mount_options}}"
    depends_on: [mount-point]
```

#### 3.3.7 Recipe Registry Structure

A recipe registry is a git repo with this structure:

```
forjar-recipes/
  registry.yaml              # metadata index
  recipes/
    nfs-server.yaml
    nfs-client.yaml
    docker-host.yaml
    rust-toolchain.yaml
    gpu-drivers.yaml
    ...
```

`registry.yaml`:

```yaml
name: paiml-recipes
version: "1.0"
description: "Pragmatic AI Labs infrastructure recipes"
recipes:
  - name: nfs-server
    path: recipes/nfs-server.yaml
    tags: [networking, storage]
  - name: docker-host
    path: recipes/docker-host.yaml
    tags: [containers]
  - name: gpu-drivers
    path: recipes/gpu-drivers.yaml
    tags: [gpu, nvidia]
    arch: [x86_64]
```

#### 3.3.8 Comparison to Alternatives

| Capability | Nix Flakes | Homebrew Taps | Ansible Roles | **Forjar Recipes** |
|------------|-----------|---------------|---------------|-------------------|
| Format | Nix lang | Ruby DSL | YAML + Jinja2 | **Pure YAML** |
| Inputs | Module options (typed) | Formula DSL | `defaults/main.yml` | **Typed YAML inputs** |
| Sharing | FlakeHub, git | GitHub taps | Ansible Galaxy | **Git repos** |
| Lock pinning | `flake.lock` (SHA) | None | `requirements.yml` | **forjar.lock.yaml (SHA)** |
| Composition | `inputs.follows` | None | `meta/dependencies` | **`requires` + `depends_on`** |
| Namespacing | Flake outputs | Formula names | Role name prefix | **`recipe-id/resource-id`** |
| Customization | Overlays | None | Variables | **Input overrides** |
| Learning curve | Very steep | Low | Medium | **Minimal (YAML only)** |

### 3.4 Machine Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `hostname` | string | required | Machine hostname |
| `addr` | string | required | Network address — IP, DNS, or `container` sentinel |
| `user` | string | `root` | SSH user |
| `arch` | string | `x86_64` | CPU architecture |
| `ssh_key` | string | — | Path to SSH private key |
| `roles` | [string] | `[]` | Informational role tags |
| `transport` | string | — | Explicit transport: `container`. If omitted, inferred from `addr`. |
| `container` | object | — | Container config (required when `transport: container`) |
| `container.runtime` | string | `docker` | `docker` or `podman` |
| `container.image` | string | — | OCI image (required for ephemeral containers) |
| `container.name` | string | `forjar-{key}` | Container name |
| `container.ephemeral` | bool | `true` | Destroy container after apply |
| `container.privileged` | bool | `false` | Run with `--privileged` |
| `container.init` | bool | `true` | Run with `--init` for PID 1 reaping |

### 3.5 Template Variables

| Pattern | Resolves To |
|---------|-------------|
| `{{params.key}}` | Global parameter value |
| `{{machine.name.addr}}` | Machine address |
| `{{machine.name.hostname}}` | Machine hostname |
| `{{resource.id.path}}` | Resource path (for file/mount) |

---

## 4. State Management

### 4.1 State Directory

All state lives in the repo under `state/`:

```
state/
  forjar.lock.yaml              Global lock (schema version, last apply)
  lambda/
    state.lock.yaml             Per-machine BLAKE3 state
    events.jsonl                Per-machine provenance log
    snapshots/
      2026-02-16T14-00-00Z.yaml   Point-in-time snapshot
  intel/
    state.lock.yaml
    events.jsonl
    snapshots/
      ...
  jetson/
    state.lock.yaml
    events.jsonl
```

### 4.2 Lock File Format

```yaml
schema: "1.0"
machine: lambda
hostname: noah-Lambda-Vector
generated_at: "2026-02-16T14:00:00Z"
generator: "forjar 0.1.0"
blake3_version: "1.8"

resources:
  nfs-server-pkg:
    type: package
    status: converged
    applied_at: "2026-02-16T14:00:01Z"
    duration_seconds: 3.2
    hash: "blake3:a1b2c3d4..."
    details:
      packages:
        nfs-kernel-server: "1:2.6.4-3ubuntu1"
        nfs-common: "1:2.6.4-3ubuntu1"

  nfs-exports:
    type: file
    status: converged
    applied_at: "2026-02-16T14:00:02Z"
    duration_seconds: 0.01
    hash: "blake3:e5f6g7h8..."
    details:
      content_hash: "blake3:content..."
      owner: root
      group: root
      mode: "0644"
      size_bytes: 82

  nfs-server-svc:
    type: service
    status: converged
    applied_at: "2026-02-16T14:00:03Z"
    duration_seconds: 0.5
    hash: "blake3:i9j0k1l2..."
    details:
      active: true
      enabled: true
      pid: 12345
```

### 4.3 State Hashing

Every resource gets a composite BLAKE3 hash computed from its **observable state** on the machine:

| Resource Type | Hash Inputs |
|--------------|-------------|
| `package` | Package name + installed version |
| `file` | Content hash + owner + group + mode |
| `service` | Active state + enabled state |
| `mount` | Source + path + fstype + options + mounted state |
| `user` | UID + GID + groups + shell + home |

The machine-level hash is a BLAKE3 of all resource hashes (sorted by resource ID).

---

## 5. Tripwire & Provenance

### 5.1 Provenance Chain

Every `forjar apply` produces three artifacts:

1. **State lock** (`state/<machine>/state.lock.yaml`) — BLAKE3 hashes of all managed resources
2. **Syscall trace** — renacer captures every syscall during apply (file writes, chmod, chown, mount, etc.)
3. **Event log** (`state/<machine>/events.jsonl`) — append-only log of what happened

All three are committed to git. The provenance chain is:

```
git log state/lambda/state.lock.yaml
  → commit abc123 "forjar apply: converge lambda (3 resources)"
    → state.lock.yaml shows BLAKE3 hashes
    → events.jsonl shows exact operations + timestamps
    → git diff shows what changed
```

### 5.2 Event Log Format

```jsonl
{"ts":"2026-02-16T14:00:00Z","event":"apply_started","machine":"lambda","run_id":"r-abc123","forjar_version":"0.1.0"}
{"ts":"2026-02-16T14:00:01Z","event":"resource_started","machine":"lambda","resource":"nfs-server-pkg","action":"install"}
{"ts":"2026-02-16T14:00:04Z","event":"resource_converged","machine":"lambda","resource":"nfs-server-pkg","duration_seconds":3.2,"hash":"blake3:a1b2c3d4"}
{"ts":"2026-02-16T14:00:04Z","event":"resource_started","machine":"lambda","resource":"nfs-exports","action":"create"}
{"ts":"2026-02-16T14:00:04Z","event":"resource_converged","machine":"lambda","resource":"nfs-exports","duration_seconds":0.01,"hash":"blake3:e5f6g7h8"}
{"ts":"2026-02-16T14:00:05Z","event":"apply_completed","machine":"lambda","run_id":"r-abc123","resources_converged":3,"resources_unchanged":0,"resources_failed":0,"total_seconds":5.0}
```

### 5.3 Drift Detection

`forjar drift` re-reads every managed resource on every machine and compares to the lock:

```
$ forjar drift
Checking lambda (3 resources)...
  nfs-server-pkg   OK
  nfs-exports      DRIFTED  (content_hash changed: blake3:e5f6... → blake3:x9y8...)
  nfs-server-svc   OK

Checking intel (2 resources)...
  nfs-client-pkg   OK
  raid-mount       OK

Drift detected on lambda: 1 resource
  nfs-exports: /etc/exports was modified outside forjar
    Expected: blake3:e5f6g7h8...
    Actual:   blake3:x9y8z7w6...
    Last forjar apply: 2026-02-16T14:00:02Z (commit abc123)
```

### 5.4 Tripwire Mode

When `policy.tripwire: true`, forjar can run as a periodic check (via cron or systemd timer):

```bash
forjar drift --tripwire --alert-cmd "notify-send 'forjar: drift on {{machine}}'"
```

This hashes all managed files and compares to lock state. Any unauthorized change triggers the alert. Because BLAKE3 hashes are microsecond-fast, this can run every minute on thousands of files with negligible overhead.

---

## 6. Execution Model

### 6.1 Plan Phase

`forjar plan` reads the YAML, connects to each machine (SSH or local), reads current state, and produces a diff:

```
$ forjar plan
Planning: sovereign-lab (3 machines, 7 resources)

lambda:
  + nfs-server-pkg      INSTALL  nfs-kernel-server, nfs-common
  + nfs-exports          CREATE   /etc/exports
  + nfs-server-svc       START    nfs-kernel-server

intel:
  + nfs-client-pkg      INSTALL  nfs-common
  + stack-tools          INSTALL  batuta, whisper-apr (cargo)
  + raid-mount           MOUNT    192.168.50.50:/mnt/nvme-raid0 → /mnt/lambda-raid
  + transcripts-dir      CREATE   /data/transcripts

Plan: 7 to add, 0 to change, 0 to destroy.
```

### 6.2 Apply Phase

`forjar apply` executes the plan:

1. Build resource dependency DAG (Kahn's toposort, deterministic)
2. Group resources by machine
3. For each machine (in parallel if `parallel_machines: true`):
   a. Open SSH connection (or local exec)
   b. Start renacer syscall trace
   c. For each resource in topo order:
      - Generate Rust provisioning code
      - Transpile via bashrs to purified POSIX shell
      - Execute on target
      - Hash resulting state
      - Update lock
      - Append event
   d. Stop renacer trace
   e. Save trace artifact
4. Write all lock files
5. Commit to git (if `--auto-commit`)

### 6.3 Shell Generation Pipeline

This is what makes forjar unique. No resource handler writes raw shell. Instead:

```
Resource declaration (YAML)
       │
       ▼
Rust codegen (generates typed provisioning AST)
       │
       ▼
bashrs transpile (Rust AST → purified POSIX shell)
       │
       ▼
bashrs verify (proves: no injection, idempotent, no unquoted vars)
       │
       ▼
Execute via SSH
```

Example for a `package` resource:

```rust
// codegen output (never seen by user, internal AST)
fn provision_nfs_server_pkg() {
    let packages = ["nfs-kernel-server", "nfs-common"];
    if !all_installed(&packages) {
        apt_update();
        apt_install(&packages);
    }
    assert_installed(&packages);
}
```

bashrs transpiles this to:

```sh
#!/bin/sh
set -euo pipefail
if ! dpkg -l 'nfs-kernel-server' >/dev/null 2>&1 || ! dpkg -l 'nfs-common' >/dev/null 2>&1; then
  apt-get update -qq
  DEBIAN_FRONTEND=noninteractive apt-get install -y -qq 'nfs-kernel-server' 'nfs-common'
fi
dpkg -l 'nfs-kernel-server' >/dev/null 2>&1
dpkg -l 'nfs-common' >/dev/null 2>&1
```

The shell is:
- **Quoted** — no injection possible
- **Idempotent** — check-before-act pattern
- **Verified** — postcondition asserts at the end
- **Deterministic** — same input always produces same shell

### 6.4 Transport

Forjar supports three execution transports. Transport selection follows a priority chain:

| Priority | Condition | Transport | Dispatch |
|----------|-----------|-----------|----------|
| 1 | `transport: container` or `addr: container` | Container | `docker exec -i <name> bash` |
| 2 | `addr` is `127.0.0.1`, `localhost`, or local hostname | Local | `bash` (stdin pipe) |
| 3 | All other addresses | SSH | `ssh user@addr bash` (stdin pipe) |

All three transports share the same mechanism: **pipe a shell script to bash stdin, capture stdout/stderr/exit_code**.

#### Container Transport

```rust
fn exec_container(machine: &Machine, script: &str) -> Result<ExecOutput> {
    Command::new(&config.runtime)  // "docker" or "podman"
        .args(["exec", "-i", &container_name, "bash"])
        .stdin(Stdio::piped())     // pipe purified shell to stdin
        .output()
}
```

Container lifecycle:
1. **Ensure** — `docker run -d --name <name> --init <image> sleep infinity`
2. **Exec** — `docker exec -i <name> bash` (one per resource apply)
3. **Cleanup** — `docker rm -f <name>` (ephemeral only, even on failure)

#### Ephemeral vs Attached Containers

| Mode | `ephemeral` | Behavior |
|------|-------------|----------|
| Ephemeral | `true` (default) | Container created before apply, destroyed after |
| Attached | `false` | Container must already exist; forjar only execs into it |

#### Local Transport

```rust
fn exec_local(script: &str) -> Result<ExecOutput> {
    Command::new("bash")
        .stdin(Stdio::piped())
        .output()
}
```

#### SSH Transport

```rust
fn exec_ssh(machine: &Machine, script: &str) -> Result<ExecOutput> {
    Command::new("ssh")
        .args(["-o", "BatchMode=yes"])
        .args(["-o", "ConnectTimeout=5"])
        .args(["-i", &machine.ssh_key])
        .arg(format!("{}@{}", machine.user, machine.addr))
        .arg("bash")
        .stdin(Stdio::piped())
        .output()
}
```

Script is piped to stdin, never passed as an argument (prevents arg-length limits and injection).

---

## 7. CLI Reference

### 7.1 Commands

```
forjar <COMMAND> [OPTIONS]

Commands:
  init        Initialize a new forjar project
  validate    Parse and validate forjar.yaml without connecting
  plan        Show execution plan (diff desired vs current)
  apply       Converge infrastructure to desired state
  drift       Detect unauthorized changes (tripwire)
  status      Show current state from lock files
  history     Show apply history from event logs
  destroy     Remove all managed resources (reverse order)
  import      Import existing infrastructure from a machine
  show        Show fully resolved config (recipes, templates, secrets)
  graph       Show resource dependency graph (Mermaid or DOT)
  check       Run check scripts to verify pre-conditions
  diff        Compare two state snapshots
  fmt         Format (normalize) a forjar.yaml config file
  lint        Lint config for best practices beyond validation
  rollback    Rollback to a previous config revision from git history
  anomaly     Detect anomalous resource behavior from event history
```

### 7.2 Global Options

```
-v, --verbose           Enable verbose output (diagnostic info to stderr)
    --no-color          Disable colored output (also honors NO_COLOR env)
-h, --help              Print help
-V, --version           Print version
```

### 7.3 `forjar plan`

```
forjar plan [OPTIONS]

Options:
  -f, --file <PATH>      Config file path (default: forjar.yaml)
  -m, --machine <NAME>   Plan for specific machine only
  -r, --resource <ID>    Plan for specific resource only
  -t, --tag <TAG>        Filter to resources with this tag
  --state-dir <PATH>     State directory (default: state)
  --json                 Output plan as JSON
  --output-dir <DIR>     Write generated scripts to directory for auditing
```

### 7.4 `forjar apply`

```
forjar apply [OPTIONS]

Options:
  -f, --file <PATH>      Config file path (default: forjar.yaml)
  -m, --machine <NAME>   Apply to specific machine only
  -r, --resource <ID>    Apply specific resource only
  -t, --tag <TAG>        Filter to resources with this tag
  --force                Force re-apply all resources (ignore cache)
  --dry-run              Show what would be executed without running
  --auto-commit          Git commit state after successful apply
  --no-tripwire          Skip provenance tracing (faster, less safe)
  -p, --param KEY=VALUE  Override a parameter
  --timeout <SECS>       Timeout per transport operation (seconds)
  --state-dir <PATH>     State directory (default: state)
```

### 7.5 `forjar drift`

```
forjar drift [OPTIONS]

Options:
  -f, --file <PATH>          Config file path (default: forjar.yaml)
  -m, --machine <NAME>       Check specific machine only
  --state-dir <PATH>         State directory (default: state)
  --tripwire                 Exit non-zero on any drift (for cron/CI)
  --alert-cmd <CMD>          Run command on drift detection (sets $FORJAR_DRIFT_COUNT)
  --auto-remediate           Auto-fix drift: force re-apply all drifted resources
  --dry-run                  List resources that would be checked without connecting
  --json                     Output drift report as JSON
```

### 7.6 `forjar history`

```
forjar history [OPTIONS]

Options:
  --state-dir <PATH>     State directory (default: state)
  -m, --machine <NAME>   Show history for specific machine
  -n, --limit <N>        Show last N applies (default: 10)
  --json                 Output as JSON
```

### 7.7 `forjar check`

```
forjar check [OPTIONS]

Options:
  -f, --file <PATH>      Config file path (default: forjar.yaml)
  -m, --machine <NAME>   Filter to specific machine
  -r, --resource <ID>    Filter to specific resource
  --tag <TAG>            Filter to resources with this tag
  --json                 Output as JSON
```

Runs check scripts against live machines to verify pre-conditions without applying. Exits non-zero if any check fails.

### 7.8 `forjar show`

```
forjar show [OPTIONS]

Options:
  -f, --file <PATH>      Config file path (default: forjar.yaml)
  -r, --resource <ID>    Show specific resource only
  --json                 Output as JSON instead of YAML
```

Shows the fully resolved config (recipes expanded, templates resolved, secrets injected). Useful for debugging.

### 7.9 `forjar import`

```
forjar import [OPTIONS]

Options:
  --addr <HOST>          Machine address (IP, hostname, or localhost)
  --user <USER>          SSH user (default: root)
  --name <NAME>          Machine name in config (derived from addr if omitted)
  --output <FILE>        Output file path (default: forjar.yaml)
  --scan <TYPES>         Comma-separated scan types (default: packages,files,services)
```

Scans installed packages (dpkg), enabled services (systemctl), and config files (/etc/*.conf). The generated config should be reviewed and customized before applying.

### 7.10 `forjar diff`

```
forjar diff <FROM> <TO> [OPTIONS]

Options:
  -m, --machine <NAME>   Filter to specific machine
  --json                 Output as JSON
```

Compares two state snapshots to show what changed between applies. Output symbols: `+` added, `-` removed, `~` changed.

### 7.11 `forjar fmt`

```
forjar fmt [OPTIONS]

Options:
  -f, --file <PATH>      Config file path (default: forjar.yaml)
  --check                Check formatting without writing (exit non-zero if unformatted)
```

Parses YAML, validates, and re-serializes in canonical format. Idempotent.

### 7.12 `forjar lint`

```
forjar lint [OPTIONS]

Options:
  -f, --file <PATH>      Config file path (default: forjar.yaml)
  --json                 Output as JSON
```

Detects:
- Unused machines (defined but not referenced)
- Resources without tags (when config has many resources)
- Duplicate content across file resources
- Dependencies on non-existent resources
- Cross-machine dependencies (resource depends on resource targeting different machines)
- Package resources with empty package lists

### 7.13 `forjar rollback`

```
forjar rollback [OPTIONS]

Options:
  -f, --file <PATH>      Config file path (default: forjar.yaml)
  -n, --revision <N>     Git revisions back to rollback to (default: 1)
  -m, --machine <NAME>   Filter to specific machine
  --dry-run              Show what would change without applying
  --state-dir <PATH>     State directory (default: state)
```

Reads the previous config from `git show HEAD~N:<file>`, compares against current, and re-applies with `--force`.

### 7.14 `forjar anomaly`

```
forjar anomaly [OPTIONS]

Options:
  --state-dir <PATH>     State directory (default: state)
  -m, --machine <NAME>   Filter to specific machine
  --min-events <N>       Minimum events to consider (default: 3)
  --json                 Output as JSON
```

Statistical anomaly detection from event history. Analyzes per-resource metrics:

- **High churn** (z-score > 1.5): Resources converging far more often than average
- **High failure rate** (>20%): Resources failing more than 1 in 5 applies
- **Drift events**: Any drift detected in history

---

## 8. Phased Implementation

### Phase 1: Foundation (v0.1) — Immediate Need

**Goal**: `forjar apply` works on lambda + intel + jetson with packages, files, services, mounts.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-001 | `core/types.rs` — all types from this spec | **Done** |
| FJ-002 | `core/parser.rs` — YAML parse + validate | **Done** |
| FJ-003 | `core/resolver.rs` — template resolution + DAG | **Done** |
| FJ-004 | `core/planner.rs` — desired vs current state diff | **Done** |
| FJ-005 | `core/codegen.rs` — script generation for resources | **Done** |
| FJ-006 | `resources/package.rs` — apt + cargo + uv providers | **Done** |
| FJ-007 | `resources/file.rs` — file/directory/symlink | **Done** |
| FJ-008 | `resources/service.rs` — systemd management | **Done** |
| FJ-009 | `resources/mount.rs` — NFS + bind mounts | **Done** |
| FJ-010 | `transport/local.rs` — local execution | **Done** |
| FJ-011 | `transport/ssh.rs` — SSH execution | **Done** |
| FJ-012 | `core/executor.rs` — orchestration loop | **Done** |
| FJ-013 | `core/state.rs` — lock file management | **Done** |
| FJ-014 | `tripwire/hasher.rs` — BLAKE3 state hashing | **Done** |
| FJ-015 | `tripwire/eventlog.rs` — JSONL event log | **Done** |
| FJ-016 | `tripwire/drift.rs` — drift detection (file + non-file via state_query_script) | **Done** |
| FJ-017 | `cli/` — all subcommands (validate, plan, apply, drift, status, graph, destroy, history) | **Done** |
| FJ-018 | Integration test: lambda + intel NFS setup | **Done** |
| FJ-019 | `core/recipe.rs` — recipe loading, input validation, expansion + CLI pipeline wiring | **Done** |
| FJ-020 | Provable contracts integration — YAML contracts, binding.yaml, `#[contract]` annotations, falsification tests | **Done** |
| FJ-021 | `transport/container.rs` — container exec + ephemeral lifecycle | **Done** |
| FJ-022 | Dogfood configs + end-to-end container verification workflow | **Done** |

### Phase 2: Containers + Parallel (v0.2)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-030 | `resources/docker.rs` — container lifecycle | **Done** |
| FJ-031 | `resources/user.rs` — user/group management | **Done** |
| FJ-032 | `resources/network.rs` — firewall rules (ufw) | **Done** |
| FJ-033 | `resources/cron.rs` — scheduled tasks (crontab) | **Done** |
| FJ-034 | Parallel multi-machine apply via `std::thread::scope` | **Done** |
| FJ-035 | Source file transfer via base64 transport | **Done** |
| FJ-036 | bashrs integration — full shell purification pipeline | |

> **Note**: FJ-030 (`resources/docker.rs`) manages containers *as resources* (deploying containers on machines). This is distinct from FJ-021 which uses containers *as transport targets* (running forjar scripts inside containers).

### Phase 3: Kernel Isolation (v0.3)

| Ticket | Description |
|--------|-------------|
| FJ-040 | `resources/pepita.rs` — kernel namespace isolation |
| FJ-041 | pepita cgroup management (memory, CPU, GPU) |
| FJ-042 | Overlay filesystem via pepita |
| FJ-043 | Network namespace isolation |
| FJ-044 | Migration path: Docker → pepita |

### Phase 4: Intelligence (v0.4)

| Ticket | Description |
|--------|-------------|
| FJ-050 | `tripwire/tracer.rs` — full renacer syscall tracing |
| FJ-051 | ML drift anomaly detection via aprender |
| FJ-052 | Cost-aware scheduling — `cost` field on machines, sorted execution order | **Done** |
| FJ-053 | Auto-remediation (`--auto-remediate` on drift → force re-apply) | **Done** |
| FJ-054 | Pre/post apply hooks (`policy.pre_apply` / `policy.post_apply`) | **Done** |

### Phase 5: Polish (v0.5)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-060 | `forjar graph` — Mermaid/DOT visualization | **Done** |
| FJ-061 | `forjar destroy` — teardown all resources | **Done** |
| FJ-062 | Secrets management — `{{secrets.KEY}}` templates resolved from `FORJAR_SECRET_*` env vars | **Done** |
| FJ-063 | MCP integration via paiml-mcp-agent-toolkit | |
| FJ-064 | Cross-architecture support — `arch` field on resources + machines, validation, plan/apply filtering | **Done** |
| FJ-065 | `forjar import` — scan machine and generate forjar.yaml | **Done** |

### Phase 6: Developer Experience (v0.6)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-070 | `forjar show` — resolved config viewer (recipes, templates, secrets) | **Done** |
| FJ-071 | `forjar check` — pre-condition verification without apply | **Done** |
| FJ-072 | `forjar diff` — compare two state snapshots | **Done** |
| FJ-073 | `forjar fmt` — canonical YAML normalization with `--check` mode | **Done** |
| FJ-074 | `forjar lint` — best practice warnings (unused machines, untagged resources, duplicate content, broken deps, cross-machine deps, empty packages) | **Done** |
| FJ-075 | Resource tags — `tags: [web, critical]` field + `--tag` filtering on plan/apply/check | **Done** |
| FJ-076 | Transport timeout — `--timeout` flag for per-resource script execution limits | **Done** |
| FJ-077 | Drift dry-run — `--dry-run` flag to list checks without connecting | **Done** |
| FJ-078 | Drift auto-remediate — `--auto-remediate` force re-applies drifted resources | **Done** |
| FJ-079 | Plan output-dir — `--output-dir` writes generated scripts for audit | **Done** |
| FJ-080 | `forjar rollback` — restore previous config from git history + force re-apply | **Done** |
| FJ-081 | Systemd detection guard — graceful skip for service resources in non-systemd environments | **Done** |
| FJ-082 | `forjar anomaly` — statistical drift anomaly detection from event history (z-score churn, failure rate, drift events) | **Done** |
| FJ-083 | Parser validation hardening — state enum validation for file/service/mount/docker, protocol/action validation for network, symlink target requirement | **Done** |
| FJ-084 | `forjar import` — add `users` and `cron` scan types for user/crontab discovery | **Done** |
| FJ-085 | SSH transport — refactor `build_ssh_args()` + 10 unit tests for command construction, key expansion, argument ordering | **Done** |
| FJ-086 | Cron schedule validation — 5-field check, state validation (present/absent), absent skips required fields | **Done** |
| FJ-087 | BLAKE3 hasher edge case tests — empty files, large streaming, file-vs-string consistency, directory change detection | **Done** |

---

## 9. Performance Targets

| Operation | Target | Rationale |
|-----------|--------|-----------|
| `forjar validate` | < 10ms | Pure YAML parse, no I/O |
| `forjar plan` (3 machines, 20 resources) | < 2s | Parallel SSH + BLAKE3 hash |
| `forjar drift` (3 machines, 100 files) | < 1s | BLAKE3 is 4GB/s on modern CPUs |
| `forjar apply` (no changes) | < 500ms | Hash compare only, no shell exec |
| Binary size | < 10MB | Single static binary, no runtime |
| Memory usage | < 50MB | No GC, no runtime, no VM |
| Cold start | < 5ms | Rust binary, no interpreter |

---

## 10. Testing Strategy

### 10.1 Unit Tests

Every module has inline tests. Minimum 95% line coverage.

- `types.rs`: serde roundtrip, defaults, validation
- `parser.rs`: valid/invalid YAML, missing fields, bad refs
- `resolver.rs`: template substitution, DAG cycle detection
- `planner.rs`: diff computation for each resource type
- `codegen.rs`: Rust AST → expected shell output
- `state.rs`: lock roundtrip, atomic writes, hash verification
- `tripwire/`: drift detection, event log formatting

### 10.2 Integration Tests

End-to-end tests using local execution (no SSH needed):

1. Parse YAML → plan → apply locally → verify state lock
2. Re-apply → verify no changes (idempotent)
3. Modify file outside forjar → drift detection catches it
4. Dependency ordering: service depends on package depends on file

### 10.3 Live Tests

Against real machines (gated behind `--features live-test`):

1. SSH to intel → install package → verify
2. Create NFS export on lambda → mount on intel → verify
3. Full 3-machine convergence

### 10.4 Formal Verification

Provable-contracts integration provides three verification layers:

1. **Compile-time**: `build.rs` calls `verify_bindings()` — build fails if binding gaps exist
2. **Falsification**: proptest-based tests derived from contract proof obligations
3. **Bounded model checking**: Kani harnesses for pure-functional invariants (Phase 2)

Contract YAML files live in `../provable-contracts/contracts/forjar/`.

### 10.5 Container Integration Tests

Container transport tests verify all resource types work inside Docker/Podman containers:

**Test target**: `tests/Dockerfile.test-target` (Ubuntu 22.04 + bash + coreutils + sudo)

**Test matrix** (feature-gated: `--features container-test`):

| Test | Description |
|------|-------------|
| Container lifecycle | ensure → exec → cleanup |
| File resource in container | Create file, verify content |
| Transport dispatch | `exec_script` routes to container |
| Idempotent ensure | Second ensure is a no-op |

**Running**:
```bash
docker build -t forjar-test-target -f tests/Dockerfile.test-target .
cargo test --features container-test
```

### 10.6 Dogfood Workflow

Container transport enables end-to-end dogfooding of all Phase 1 resource types without root or host pollution. Three dogfood configs exercise progressively deeper code paths:

| Config | Resources | What it proves |
|--------|-----------|----------------|
| `examples/dogfood-container.yaml` | file, directory | File codegen, state hashing, lock persistence |
| `examples/dogfood-packages.yaml` | package (apt), file, dependency DAG | Package codegen against real dpkg, cross-resource dependencies, idempotency |
| `examples/dogfood-phase2.yaml` | user, file (source), cron, dependency DAG | Phase 2 resource types, base64 source transfer, user management, cron |

**Dogfood verification workflow** (run after any codegen, transport, or executor change):

```bash
# 1. Build test target
docker build -t forjar-test-target -f tests/Dockerfile.test-target .

# 2. First apply — all resources converge
cargo run -- apply -f examples/dogfood-phase2.yaml --state-dir /tmp/dogfood-state

# 3. Idempotency proof — second apply, zero changes
cargo run -- apply -f examples/dogfood-phase2.yaml --state-dir /tmp/dogfood-state

# 4. Drift detection — verify lock state matches live state
cargo run -- drift -f examples/dogfood-phase2.yaml --state-dir /tmp/dogfood-state

# 5. Destroy — reverse teardown and state cleanup
cargo run -- destroy -f examples/dogfood-phase2.yaml --state-dir /tmp/dogfood-state --yes
```

**What each dogfood config exercises**:

**`dogfood-container.yaml`** (file resources only):
- File creation with content, owner, group, mode
- Directory creation with permissions
- BLAKE3 content hashing against real files
- State lock persistence and idempotency

**`dogfood-packages.yaml`** (packages + files + dependency DAG):
- `apt-get install` codegen against real dpkg inside container
- `dpkg -l` check-before-act idempotency pattern
- Cross-resource dependency ordering (package → file)
- Package state query for hash computation
- Full executor loop: codegen → transport → hash → state → events

---

## 11. Project Bootstrap

```toml
# Cargo.toml
[package]
name = "forjar"
version = "0.1.0"
edition = "2021"
rust-version = "1.85.0"
authors = ["Pragmatic AI Labs"]
description = "Rust-native Infrastructure as Code — bare-metal first, BLAKE3 state, provenance tracing"
license = "MIT OR Apache-2.0"
repository = "https://github.com/paiml/forjar"
keywords = ["iac", "infrastructure", "devops", "provisioning", "bare-metal"]
categories = ["command-line-utilities", "development-tools"]

[dependencies]
blake3 = "1.8"
serde = { version = "1.0", features = ["derive"] }
serde_yaml_ng = "0.10"
serde_json = "1.0"
clap = { version = "4", features = ["derive"] }
indexmap = { version = "2.7", features = ["serde"] }
base64 = "0.22.1"
provable-contracts-macros = { path = "../provable-contracts/crates/provable-contracts-macros" }

[build-dependencies]
provable-contracts = { path = "../provable-contracts/crates/provable-contracts" }

[dev-dependencies]
tempfile = "3"
criterion = { version = "0.5", features = ["html_reports"] }
proptest = "1"

[features]
container-test = []
```

---

## 12. Invariants

| ID | Invariant | Enforced By | Contract |
|----|-----------|-------------|----------|
| I1 | Every apply produces a state lock committed to git | executor + state module | — |
| I2 | Every shell command is generated from Rust, never hand-written | codegen + bashrs | `codegen-dispatch-v1.yaml` |
| I3 | State hashes are BLAKE3 content-addressed | hasher module | `blake3-state-v1.yaml` |
| I4 | Lock files are written atomically (temp + rename) | state module | `execution-safety-v1.yaml` |
| I5 | Resource ordering respects dependency DAG | resolver + Kahn's toposort | `dag-ordering-v1.yaml` |
| I6 | Drift detection compares live state to lock hashes | tripwire/drift | — |
| I7 | Jidoka: stop on first failure, preserve partial state | executor | `execution-safety-v1.yaml` |
| I8 | No raw shell execution — all shell is bashrs-purified | codegen pipeline | — |
| I9 | State never leaves the git repo — no remote backends | state module | — |
| I10 | Every apply is traceable to a git commit | tripwire/eventlog | — |
| I11 | Recipes expand deterministically — same inputs always produce same resources | recipe module | `recipe-determinism-v1.yaml` |
| I12 | Recipe inputs are validated against declared types before expansion | recipe module | `recipe-determinism-v1.yaml` |
| I13 | Git-pinned recipes are locked by SHA for reproducibility | state module | — |

---

## 13. Provable Contracts

### 13.1 Overview

Forjar integrates with the `provable-contracts` framework to provide formal verification of core invariants. Every critical function is annotated with a `#[contract]` attribute that binds it to a YAML contract equation. The build system verifies binding completeness at compile time.

### 13.2 Contract Architecture

```
provable-contracts/contracts/forjar/
├── blake3-state-v1.yaml          I3: BLAKE3 content-addressed hashing
├── dag-ordering-v1.yaml          I5: Topological sort correctness
├── execution-safety-v1.yaml      I4, I7: Atomic writes + jidoka policy
├── recipe-determinism-v1.yaml    I11, I12: Deterministic expansion + input validation
├── codegen-dispatch-v1.yaml      I2: Script generation dispatch completeness
└── binding.yaml                  13 bindings mapping equations → forjar functions
```

### 13.3 Verification Layers

| Layer | Mechanism | When |
|-------|-----------|------|
| L1 | `build.rs` binding verification | Every `cargo build` |
| L2 | Proptest falsification tests | Every `cargo test` |
| L3 | Kani bounded model checking | Phase 2 (`cargo kani`) |

### 13.4 Annotated Functions

| Module | Function | Contract | Equation |
|--------|----------|----------|----------|
| `tripwire::hasher` | `hash_string` | `blake3-state-v1` | `hash_string` |
| `tripwire::hasher` | `hash_file` | `blake3-state-v1` | `hash_file` |
| `tripwire::hasher` | `composite_hash` | `blake3-state-v1` | `composite_hash` |
| `core::resolver` | `build_execution_order` | `dag-ordering-v1` | `topological_sort` |
| `core::state` | `save_lock` | `execution-safety-v1` | `atomic_write` |
| `core::recipe` | `validate_inputs` | `recipe-determinism-v1` | `validate_inputs` |
| `core::recipe` | `expand_recipe` | `recipe-determinism-v1` | `expand_recipe` |
| `core::codegen` | `check_script` | `codegen-dispatch-v1` | `check_script` |
| `core::codegen` | `apply_script` | `codegen-dispatch-v1` | `apply_script` |
| `core::codegen` | `state_query_script` | `codegen-dispatch-v1` | `state_query_script` |

### 13.5 Build Integration

`build.rs` conditionally loads `binding.yaml` when the sibling `provable-contracts` repo is present:

```rust
fn main() {
    let binding_path = "../provable-contracts/contracts/forjar/binding.yaml";
    if std::path::Path::new(binding_path).exists() {
        provable_contracts::build_helper::verify_bindings(
            binding_path,
            provable_contracts::build_helper::BindingPolicy::WarnOnGaps,
        );
    }
}
```

When bindings are verified, the build emits `CONTRACT_*` environment variables consumed by `#[contract]` proc macros at compile time. Missing bindings produce compile warnings (WarnOnGaps policy).

### 13.6 Falsification Tests

15 proptest-based falsification tests validate contract proof obligations:

| Module | Tests | Obligations Tested |
|--------|-------|--------------------|
| `hasher.rs` | 3 | Prefix format, determinism, composite order-sensitivity |
| `resolver.rs` | 3 | Topological ordering, cycle detection, determinism |
| `state.rs` | 1 | Atomic write leaves no temp file |
| `executor.rs` | 2 | Jidoka stop/continue policy dispatch |
| `recipe.rs` | 4 | Expansion determinism, int bounds, path validation, external deps |
| `codegen.rs` | 2 | Dispatch completeness, dispatch symmetry |

---

## 14. Open Questions

1. ~~**Secrets**: Encrypt in git (age/sops-style) or external vault?~~ **Resolved**: Environment variable-based secrets (`FORJAR_SECRET_*` → `{{secrets.KEY}}`). Age encryption deferred to future.
2. ~~**Rollback**: Should `forjar rollback` replay the previous state, or just show the diff?~~ **Resolved**: `forjar rollback -n N` reads previous `forjar.yaml` from `git show HEAD~N`, compares changes, and re-applies with `--force`. Supports `--dry-run` for safe preview (FJ-080).
3. ~~**Import**: Should `forjar import` be able to adopt existing infrastructure?~~ **Resolved**: `forjar import --addr <host>` scans packages, services, and config files, generates forjar.yaml (FJ-065).
4. ~~**Multi-repo**: Should machines be able to be managed by multiple forjar repos?~~ **Resolved**: No — one repo per fleet, sovereignty principle. Enforced by convention; lint warns on cross-machine dependencies.
5. ~~**Systemd in containers**: Service resources require systemd. Should forjar detect when running inside a container without systemd and skip/warn?~~ **Resolved**: All service scripts now include a systemd guard (`command -v systemctl`) that gracefully exits 0 with a `FORJAR_WARN` message when systemd is unavailable (FJ-081).
