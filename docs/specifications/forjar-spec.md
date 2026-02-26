# Forjar — Rust-Native Infrastructure as Code

**Version**: 0.9.0-spec
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
| **Bare-metal first** | Manages real machines over SSH. Containers serve as execution targets (transport) and managed resources (docker type). Pepita kernel isolation provides cgroups v2, overlayfs, netns, chroot, and seccomp (FJ-040). |
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
| External deps | ~200 Go modules | ~500 npm/pip packages | ~50 Python packages | **15 crates** |
| Secrets | Vault/sensitive/KMS | ESC/BYOK | Ansible Vault | **age encryption (v0.7)** |
| Conditionals | `count`/`for_each`/`dynamic` | Native loops | `when:` | **`when:`/`for_each:` (v0.7)** |
| Multi-env | Workspaces/Stacks | Stacks/ESC | Inventory groups | **Workspaces (v0.8)** |
| State surgery | `state mv`/`rm`/`import` | `state delete`/`rename` | N/A | **`state mv`/`rm`/`list` (v0.8)** |
| Policy-as-code | Sentinel/OPA | CrossGuard | ansible-lint | **YAML policies (v0.9)** |
| Rolling deploy | Stacks orchestrate | N/A | `serial:` | **`policy.serial:` (v0.9)** |

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
│   bashrs    │                validate + purify shell scripts (FJ-036: done)
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
    mod.rs              Subcommand dispatch (init, validate, plan, apply, drift, status, history, destroy, import, show, graph, check, diff, fmt, lint, rollback, anomaly, trace, migrate, mcp, bench)
  mcp/
    mod.rs              MCP server via pforge — 9 tool handlers, registry, ForgeConfig
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
    purifier.rs         Shell script validation via bashrs (FJ-036)
    migrate.rs          Docker-to-pepita resource migration (FJ-044)
  tripwire/
    mod.rs              Provenance tracing orchestration
    hasher.rs           BLAKE3 file/directory/state hashing
    drift.rs            Drift detection (hash current vs lock)
    eventlog.rs         Append-only JSONL provenance log
    tracer.rs           W3C-compatible trace provenance (FJ-050)
    anomaly.rs          ML-inspired drift anomaly detection (FJ-051)
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
    pepita.rs           Kernel namespace isolation (FJ-040)
  transport/
    mod.rs              Transport abstraction + dispatch
    local.rs            Local execution (this machine)
    ssh.rs              SSH execution (remote machines)
    container.rs        Container execution (docker/podman exec)
    pepita.rs           Kernel namespace execution (FJ-230, planned)
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
| `bashrs` | Stack | Shell purification — core to the provability thesis. **Integrated.** |
| `pforge-runtime` | Stack | MCP server framework — O(1) handler dispatch, protocol handling. **Integrated.** |
| `pforge-config` | Stack | MCP config types — ForgeConfig, ToolDef, ParamSchema. **Integrated.** |
| `tokio` | External | Async runtime for MCP server (FJ-063). Required by pforge. **Integrated.** |
| `async-trait` | External | Async trait support for MCP handlers. **Integrated.** |
| `schemars` | External | JSON Schema generation for MCP tool introspection. **Integrated.** |
| `rustc-hash` | External | Fast FxHash for pforge handler registry. **Integrated.** |
| `pepita` | Stack | Kernel interfaces. *Implemented inline in `resources/pepita.rs` (FJ-040).* |
| `renacer` | Stack | Syscall tracing for provenance. *Implemented inline in `tripwire/tracer.rs` (FJ-050).* |
| `aprender` | Stack | ML-based drift anomaly detection. *Implemented inline in `tripwire/anomaly.rs` (FJ-051).* |

**Banned**: reqwest, hyper, tonic, any cloud SDK. tokio allowed only for MCP server (FJ-063).

**Deferred**: repartir (parallel dispatch — std::thread::scope sufficient), duende (daemon management — not needed), pmat (compliance gates — external tool).

**Planned (Phase 10)**: copia (delta sync — replaces base64 for files > 1MB, critical for multi-GB model deployment, FJ-242).

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

  # Pepita kernel namespace target (zero Docker dependency, planned FJ-230)
  isolated-box:
    hostname: isolated-box
    transport: pepita
    pepita:
      rootfs: debootstrap:jammy
      cgroups:
        memory_mb: 2048
        cpus: 2
      network: isolated
      filesystem: overlay
      ephemeral: true

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

#### `pepita` (FJ-040)

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

#### `model` (Planned — FJ-240)

```yaml
type: model
machine: <name>
name: <model identifier>
source: "hf://meta-llama/Llama-3-8B-GGUF"   # HuggingFace repo, URL, or local path
format: gguf                                   # gguf | safetensors | apr
quantization: q4_k_m                           # q4_k_m | q5_k_m | q8_0 | f16 | none
path: /data/models/llama-3-8b.gguf            # destination on machine
checksum: "blake3:abc123..."                   # optional pin (drift detection)
cache_dir: ~/.cache/apr                        # model cache directory
state: present | absent
```

#### `gpu` (Planned — FJ-241)

```yaml
type: gpu
machine: <name>
driver_version: "535"                    # NVIDIA driver version
cuda_version: "12.3"                     # CUDA toolkit version
devices: [0, 1]                          # GPU indices (default: all)
persistence_mode: true                   # nvidia-persistenced (default: true)
compute_mode: default                    # default | exclusive_process | prohibited
memory_limit_mb: 8192                    # optional cgroup GPU memory limit
state: present | absent
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
| `transport` | string | — | Explicit transport: `container` or `pepita`. If omitted, inferred from `addr`. |
| `container` | object | — | Container config (required when `transport: container`) |
| `container.runtime` | string | `docker` | `docker` or `podman` |
| `container.image` | string | — | OCI image (required for ephemeral containers) |
| `container.name` | string | `forjar-{key}` | Container name |
| `container.ephemeral` | bool | `true` | Destroy container after apply |
| `container.privileged` | bool | `false` | Run with `--privileged` |
| `container.init` | bool | `true` | Run with `--init` for PID 1 reaping |
| `pepita` | object | — | Pepita namespace config (required when `transport: pepita`). Planned: FJ-230. |
| `pepita.rootfs` | string | — | Base rootfs path or `debootstrap:<suite>` (e.g., `debootstrap:jammy`) |
| `pepita.cgroups.memory_mb` | int | `2048` | Memory limit in MB (cgroup v2 `memory.max`) |
| `pepita.cgroups.cpus` | int | `2` | CPU limit (cgroup v2 `cpuset.cpus`) |
| `pepita.network` | string | `isolated` | `isolated` (new netns) or `host` (share host network) |
| `pepita.filesystem` | string | `overlay` | `overlay` (copy-on-write overlayfs) or `bind` (bind-mount rootfs) |
| `pepita.ephemeral` | bool | `true` | Destroy namespace after apply |

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
    trace.jsonl                 W3C trace spans from apply (FJ-050)
  intel/
    state.lock.yaml
    events.jsonl
    trace.jsonl
  jetson/
    state.lock.yaml
    events.jsonl
    trace.jsonl
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

Forjar supports four execution transports. Transport selection follows a priority chain:

| Priority | Condition | Transport | Dispatch | Status |
|----------|-----------|-----------|----------|--------|
| 1 | `transport: pepita` | Pepita | `nsenter --target <pid> -- bash` (stdin pipe) | Planned (FJ-230) |
| 2 | `transport: container` or `addr: container` | Container | `docker exec -i <name> bash` | Done (FJ-021) |
| 3 | `addr` is `127.0.0.1`, `localhost`, or local hostname | Local | `bash` (stdin pipe) | Done (FJ-010) |
| 4 | All other addresses | SSH | `ssh user@addr bash` (stdin pipe) | Done (FJ-011) |

All four transports share the same mechanism: **pipe a shell script to bash stdin, capture stdout/stderr/exit_code**.

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

#### Pepita Transport (Planned — FJ-230)

Pepita transport uses Linux kernel namespaces directly — no Docker daemon, no container runtime, no image registry. The execution target is a `unshare(2)` / `clone(2)` namespace with an overlayfs rootfs.

```rust
fn exec_pepita(machine: &Machine, script: &str) -> Result<ExecOutput> {
    Command::new("nsenter")
        .args(["--target", &namespace_pid, "--mount", "--pid", "--net", "--"])
        .arg("bash")
        .stdin(Stdio::piped())     // pipe purified shell to stdin
        .output()
}
```

Pepita namespace lifecycle:
1. **Create** — `unshare --mount --pid --net --fork` + mount overlayfs rootfs from debootstrap base
2. **Cgroups** — write `memory.max` and `cpuset.cpus` to cgroup v2 hierarchy
3. **Exec** — `nsenter --target <pid> --mount --pid --net -- bash` (one per resource apply)
4. **Cleanup** — tear down namespace, remove cgroup, unmount overlayfs (ephemeral only)

**Interface model** — mirrors the wos microkernel `VmManager` pattern:

| wos concept | Pepita transport equivalent |
|---|---|
| `VmConfig` (memory, vcpus, devices) | `PepitaConfig` (rootfs, cgroups, network, filesystem) |
| `VmState` (Created→Running→Stopped) | Namespace state (Created→Running→Destroyed) |
| `VirtioDeviceConfig::Console` | `/dev/pts` bind-mount for stdin/stdout |
| `VirtioDeviceConfig::Block` | overlayfs lower/upper/work layers |
| `VirtioDeviceConfig::Net` | veth pair into network namespace |
| `MAX_VMS` jidoka guard | `MAX_NAMESPACES` jidoka guard |

**Why not Docker**: Docker requires a daemon (`dockerd`), an image registry, an overlay storage driver, and `docker` CLI. Pepita uses 4 syscalls (`clone`, `mount`, `pivot_root`, `nsenter`) and a debootstrap'd rootfs. Zero runtime dependency. True sovereign isolation.

**Requires**: `CAP_SYS_ADMIN` or root. `debootstrap` for rootfs creation (one-time setup).

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
  trace       View trace provenance data from apply runs
  migrate     Migrate Docker resources to pepita kernel isolation
  mcp         Start MCP server (pforge integration)
  bench       Run performance benchmarks (spec §9 targets)
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

### 7.15 `forjar trace`

```
forjar trace [OPTIONS]

Options:
  --state-dir <PATH>     State directory (default: state)
  -m, --machine <NAME>   Filter to specific machine
  --json                 Output as JSON
```

Reads W3C-compatible trace provenance data from `state/<machine>/trace.jsonl`. Traces are produced by `forjar apply` when `policy.tripwire: true`. Output is grouped by `trace_id` and sorted by Lamport logical clock. Each span shows resource name, operation, duration, and parent span.

### 7.16 `forjar migrate`

```
forjar migrate [OPTIONS]

Options:
  -f, --file <PATH>      Config file path (default: forjar.yaml)
  -o, --output <FILE>    Write migrated config to file (default: stdout)
```

Converts `docker` resources to `pepita` kernel namespace resources. Translates Docker-specific fields (image, ports, environment, volumes, restart policy) into pepita equivalents (cgroups, netns, overlayfs, seccomp). Emits warnings for Docker features that have no direct pepita equivalent (e.g., `restart: unless-stopped` → manual systemd unit).

### 7.17 `forjar mcp`

```
forjar mcp [OPTIONS]

Options:
  --schema               Export tool schemas as JSON instead of starting server
```

Starts a Model Context Protocol server via pforge. Exposes 9 tools: `forjar_validate`, `forjar_plan`, `forjar_drift`, `forjar_lint`, `forjar_graph`, `forjar_show`, `forjar_status`, `forjar_trace`, `forjar_anomaly`. AI assistants (Claude, Copilot, Cursor) connect via MCP to inspect and manage infrastructure. `--schema` exports JSON schemas for all tools without starting the server.

### 7.18 `forjar bench`

```
forjar bench [OPTIONS]

Options:
  --iterations <N>       Number of iterations per benchmark (default: 1000)
  --json                 Output as JSON
```

Runs inline performance benchmarks against spec §9 targets: validate (< 10ms), plan (< 2s), drift (< 1s), BLAKE3 hashing. Reports mean time, standard deviation, and margin vs target. Use `cargo bench` for Criterion-based benchmarks with statistical rigor; `forjar bench` is for quick verification.

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
| FJ-036 | bashrs integration — shell purification pipeline | **Done** |

> **Note**: FJ-030 (`resources/docker.rs`) manages containers *as resources* (deploying containers on machines). This is distinct from FJ-021 which uses containers *as transport targets* (running forjar scripts inside containers).

### Phase 3: Kernel Isolation (v0.3)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-040 | `resources/pepita.rs` — kernel namespace isolation (cgroups v2, overlayfs, netns, chroot, seccomp), 24 tests | **Done** |
| FJ-041 | pepita cgroup management (memory, CPU, GPU) — cgroups v2 memory.max + cpuset.cpus in FJ-040 | **Done** |
| FJ-042 | Overlay filesystem via pepita — overlayfs mount/unmount in FJ-040 | **Done** |
| FJ-043 | Network namespace isolation — ip netns add/del in FJ-040 | **Done** |
| FJ-044 | Migration path: Docker → pepita — `forjar migrate` CLI, `core/migrate.rs` docker_to_pepita() conversion, 16 tests | **Done** |

### Phase 4: Intelligence (v0.4)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-050 | `tripwire/tracer.rs` — renacer-compatible trace provenance (TraceSession, TraceSpan, W3C trace/span IDs, Lamport clock, JSONL output) | **Done** |
| FJ-051 | ML drift anomaly detection — ADWIN adaptive windowing, isolation scoring, EWMA z-score, detect_anomalies() bulk analysis | **Done** |
| FJ-052 | Cost-aware scheduling — `cost` field on machines, sorted execution order | **Done** |
| FJ-053 | Auto-remediation (`--auto-remediate` on drift → force re-apply) | **Done** |
| FJ-054 | Pre/post apply hooks (`policy.pre_apply` / `policy.post_apply`) | **Done** |

### Phase 5: Polish (v0.5)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-060 | `forjar graph` — Mermaid/DOT visualization | **Done** |
| FJ-061 | `forjar destroy` — teardown all resources | **Done** |
| FJ-062 | Secrets management — `{{secrets.KEY}}` templates resolved from `FORJAR_SECRET_*` env vars | **Done** |
| FJ-063 | MCP integration via pforge — 9 tool handlers (validate, plan, drift, lint, graph, show, status, trace, anomaly), `forjar mcp` CLI, pforge-runtime HandlerRegistry + McpServer, 33 tests | **Done** |
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
| FJ-088 | Network resource tests — reject action, UDP protocol, CIDR ranges, absent+source, default protocol/port, pipefail | **Done** |
| FJ-089 | Mount resource tests — bind mount, default options/fstype, fstab format/idempotency, mkdir ordering, absent cleanup | **Done** |
| FJ-090 | Codegen dispatch tests — user/docker/cron/network dispatch + pipefail safety verification for all apply scripts | **Done** |
| FJ-091 | Resolver DAG tests — self-dependency cycle, 3-level transitive chain, empty vs missing depends_on, single resource | **Done** |
| FJ-092 | User management example — user creation, SSH keys, groups, system users, user removal (state: absent) | **Done** |
| FJ-093 | Multi-machine example — 3-machine NFS+app+monitor with cross-machine deps, cost scheduling, template resolution | **Done** |
| FJ-094 | Drift & tripwire book chapter — Ch. 9 covering BLAKE3 drift, auto-remediation, anomaly detection, event logs | **Done** |
| FJ-095 | Service resource tests — pipefail safety, idempotent start/stop/enable guards, stopped+disabled combination | **Done** |
| FJ-096 | Cron resource tests — entry preservation, absent cleanup, default schedule/command, custom user in check/query | **Done** |
| FJ-097 | Event logging example — provenance event lifecycle, JSONL reading, run ID generation | **Done** |
| FJ-098 | Eventlog tests — run ID uniqueness, directory creation, JSON validity, timestamp format | **Done** |
| FJ-099 | Testing & CI/CD book chapter — Ch. 10 covering validation pyramid, container testing, GitHub Actions, monitoring | **Done** |
| FJ-100 | Container transport tests — exec error messages, fake runtime, name derivation, podman runtime, init/privileged flags, ensure/cleanup lifecycle with /bin/echo runtime, precise error message assertions (13 tests) | **Done** |
| FJ-101 | SSH transport tests — arg counting (with/without key), BatchMode verification, ConnectTimeout value, StrictHostKeyChecking, DNS hostname, nonstandard user (7 tests) | **Done** |
| FJ-102 | Transport dispatch tests — container priority over local, timeout hostname in error, IPv6 loopback, remote addr detection, stream capture, multiline scripts, exit codes (6 tests) | **Done** |
| FJ-103 | Architecture book — container lifecycle diagram (ensure→exec→cleanup), ephemeral vs attached modes, naming, runtime selection, event type reference table | **Done** |
| FJ-104 | Drift detection tests — DriftFinding fields, detail messages, detect_drift_with_machine local path, multiple files, directory drift, missing content_hash, non-string live_hash (9 tests) | **Done** |
| FJ-105 | State management tests + example — idempotent update, machine addition, overwrite, version, corrupted global lock, atomic temp cleanup (6 tests + runnable example) | **Done** |
| FJ-106 | File resource tests — parent dir creation ordering, pipefail, no content/source, no mode/owner, symlink default, state_query fallback (7 tests) | **Done** |
| FJ-107 | Package resource tests — default provider/state, idempotent check, postcondition verify, uv absent tolerance, cargo absent (6 tests) | **Done** |
| FJ-108 | Docker resource tests — stop-before-run ordering, tolerant absent/stopped, default state, all-options combined (5 tests) | **Done** |
| FJ-109 | User resource tests — custom SSH home, absent idempotent, userdel fallback, SSH permissions, multiple keys, group-before-user ordering (6 tests) | **Done** |
| FJ-110 | CLI module tests — import scan types, show JSON, fmt check, check filters, rollback error, apply param override, lint, init template (20 tests) | **Done** |
| FJ-111 | Parser validation edge cases — deep cycle, diamond pattern, compound errors, docker absent/running, mount double error, network reject/protocol, recipe missing name, arch filters, container transport, self-dep, unknown dep, content+source, symlink target, localhost, invalid states (24 tests) | **Done** |
| FJ-112 | Troubleshooting book chapter (Ch. 11) — validation errors, SSH connection, container transport, state/drift, DAG ordering, resource-specific issues, arch filtering, CLI | **Done** |
| FJ-113 | Runnable examples — validation.rs (multi-error, container transport), arch_filtering.rs (cross-arch planning), resource_scripts.rs (all 9 types × 3 scripts) | **Done** |
| FJ-114 | Executor edge case tests — record_success without tripwire, service details, record_failure tripwire off, build_resource_details variants, collect_machines dedup/order, dry-run+filter, tag-filtered apply, log_tripwire toggle, apply duration (14 tests) | **Done** |
| FJ-115 | Transport edge case tests — local: empty script, env vars, heredoc, exit code range, large output, set -euo. SSH: -o pairing, key ordering, bash-last, success method, relative path. Dispatch: timeout seconds, empty script, Debug/Clone, query alias (16 tests) | **Done** |
| FJ-116 | Drift detection edge cases — transport-based drift (local, drift, missing, directory), mixed resources, failed resource skip, empty file hash, wrong hash format, Debug/Clone (8 tests) | **Done** |
| FJ-117 | Codegen dispatch tests — unsupported type error quality, recipe non-dispatchable, all Phase 1 check scripts nonempty, all Phase 1 state_query scripts nonempty (4 tests) | **Done** |
| FJ-118 | Resolver + recipe edge case tests — unclosed template, passthrough, mixed types, unknown machine, numeric/bool params, group/mode resolution, fan-out/fan-in DAGs, unknown dep, whitespace templates, consecutive templates. Recipe: no-inputs, unclosed template, unknown input ref, terminal ID on empty, nonexistent file, invalid YAML, multiple external deps, all-defaults expansion, content field, RecipeSource derive, optional metadata, requires parsing (27 tests) | **Done** |
| FJ-119 | Expand Getting Started book chapter (Ch. 1, 113→374 lines) — core concepts table, validate-first workflow, multi-resource example with dependencies, graph visualization, filtering, dry-run, state inspection, script auditing, parameters, secrets, cross-arch support | **Done** |
| FJ-120 | Planner edge case tests — arch filter skip, arch filter with lock, multi-machine partial lock, empty execution order, nonexistent resource skipped, describe action fallbacks, content hash sensitivity, combined arch+tag filter (11 tests) | **Done** |
| FJ-121 | Eventlog/hasher edge cases — drift_detected event, nested dir creation, run ID hex format, path special chars. Hasher: directory not found, exact buffer boundary, deep nesting, composite determinism, single-char diff (9 tests) | **Done** |
| FJ-122 | Expand Cookbook chapter (Ch. 7, 548→742 lines) — partial failure recovery, lock file management, auditing and compliance, script auditing workflow, multi-environment promotion, cross-architecture fleet | **Done** |
| FJ-123 | Resource module edge case tests — service.rs (+4: invalid state no-op, restart_on+disabled, no name default, multiple restart_on), mount.rs (+4: all defaults, unknown state, absent no path, state query no path), cron.rs (+4: no name default, no owner default, absent ignores schedule, cmd tag idempotency), network.rs (+4: absent with from_addr, all defaults, no comment without name, ufw force enable always). 16 tests, 700→716. | **Done** |
| FJ-124 | User/docker edge case tests — user.rs (+4: no name default, system_user+home, ssh chown with primary group, modify branch carries all fields), docker.rs (+4: no name default, no image default, multiple ports/env/volumes, absent no pull/run). 8 tests, 716→724. | **Done** |
| FJ-125 | Expand State Management chapter (Ch. 8, 243→392 lines) — composite hashing, hash stability, hashing by resource type, state inspection commands, state comparison, selective force apply, monorepo patterns, state cleanup. Expand Testing & CI chapter (Ch. 10, 284→462 lines) — script auditing, template review, canary deploys, idempotency testing, drift testing, GitOps workflow, post-merge CI job. | **Done** |
| FJ-126 | Extend template resolution to all resource string fields — command, schedule, port, protocol, action, from_addr, image, shell, home, restart, version, plus list fields (ports, environment, volumes, packages). 7 tests, full_stack_deploy example. 724→731. | **Done** |
| FJ-127 | Fix hash_desired_state to include Phase 2 fields — image, command, schedule, restart, port, protocol, action, from_addr, shell, home, target, version, enabled, ports, environment, volumes, restart_on. Without this fix, changing a Docker image, cron schedule, or firewall port would NOT trigger an update. 7 tests, 731→738. | **Done** |
| FJ-128 | Expand Architecture + Drift book chapters past 300-line threshold. Drift detection integration tests — detect_drift_full with matching/mismatched live_hash, codegen error handling, mixed file+service. Executor edge cases — empty details, path-only, timeout threading, arch filter skip, force re-apply, lock_file=false, tripwire=false, empty config. Eventlog — timestamp field ranges, all ProvenanceEvent variants roundtrip, run ID consistency, leap year boundaries. 20 tests, 738→758. | **Done** |
| FJ-129 | Integration tests for apply→drift→re-apply lifecycle. apply_then_drift_no_change, apply_then_drift_after_modification (tamper detection), full 5-step apply-drift-reapply cycle, multi_resource_dependency_order (directory→file with depends_on), config_change_triggers_update (content A→B), event_log_full_lifecycle (4 event types in order). Uses tempdir for isolation. 6 tests, 758→764. | **Done** |
| FJ-130 | Expand Configuration chapter (Ch. 2, 305→455 lines) — cross-machine references, template syntax table, validation rules for machines/resources/dependencies, complete production example. Expand Recipes chapter (Ch. 4, 315→380 lines) — input validation rules, debugging, common error table. Expand Troubleshooting chapter (Ch. 11, 342→416 lines) — drift issues, performance debugging, 7-step checklist. Book total: 4,471→4,760 lines. | **Done** |
| FJ-131 | Comprehensive edge case tests across all modules. Types: MachineTarget default, ResourceType/FailurePolicy/ResourceStatus serde, ContainerConfig variants, Policy hooks, yaml_value_to_string (26 tests). Resolver: all template fields + machine refs + error paths (16 tests). Drift: directory hashing, DriftFinding derives, detect_drift skip paths (10 tests). Executor: localhost implicit machine, empty resources, machine_filter no-match, record_failure types, build_details group-only, continue_independent, dry-run unchanged, ResourceOutcome variants (13 tests). State: global lock overwrite, generator format, deep dirs, name preservation, status roundtrip (6 tests). Parser: parse_and_validate integration (happy path, error formatting, recipe expansion), compound validation errors (package/cron/network), state error message content (11 tests). CLI: cmd_graph (mermaid/dot/unknown format), cmd_diff (empty/same/added/JSON/filter), cmd_anomaly (empty/events/JSON/filter/nonexistent) (15 tests). Book: Resources Ch. 3 (383→545), Architecture Ch. 5 (348→450). Total: 764→861 (97 tests), book 4,760→5,024 lines. | **Done** |
| FJ-132 | Comprehensive coverage push: 861→1062 (201 tests), book 5,333→8,900+ (3,567+ lines). **Tests**: CLI (20 command handler tests), Planner (14 hash + 23 edge cases), Drift (15), Resolver (8 secret/DAG/template), Parser (8 validation), Eventlog (10), Executor (18 integration), State (8), Transport (8), Codegen (8), Types (17 machine/policy/display), Service (4), Mount (5), Docker (5), File (6), Recipe (6), Hasher (6), Container (3). **Milestone: 1000 tests**. Fixed 3 flaky executor tests (/tmp → tempdir). **Book**: All 11 chapters 745+ lines. Configuration: templates, secrets, validation. State: atomic writes, hashing, recovery. Getting Started: lifecycle, concepts, FAQ, comparisons. Troubleshooting: containers, performance, debugging. Architecture: transport, concurrency, contracts. CLI: pipelines, exit codes. Testing: property-based, coverage. Cookbook: logrotate, SSH hardening, staged rollout. Drift: internals, hash architecture. **Dogfood**: All 8 configs validate, all 15 examples pass. | **Done** |

| FJ-036 | bashrs integration — shell purification pipeline. Added `bashrs = "6.64.0"` dependency, created `core/purifier.rs` with three safety levels (validate/lint/purify), integrated bashrs script lint into `forjar lint` CLI, upgraded Rust 1.85→1.87. 1062→1118 tests (56 new), book 8,957→10,530 lines. Architecture chapter + spec updated, I8 invariant enforced. All 19 examples pass, 13 dogfood configs lint cleanly. | **Done** |
| FJ-133 | Wire FJ-050 tracer into executor — TraceSession in apply_machine(), record_span per resource, write_trace to state_dir, gated by tripwire policy. 3 integration tests (trace written, trace not written, span fields). Book: trace provenance + anomaly architecture sections. Example: `trace_provenance.rs`. | **Done** |
| FJ-134 | Wire FJ-051 anomaly module into cmd_anomaly CLI — replace inline z-score with detect_anomalies(), isolation_score(), DriftStatus. 1 integration test. Book: forjar migrate CLI docs. Example: `anomaly_detection.rs`. All 19 examples pass, 13 dogfood configs validate. 1297→1301 tests. | **Done** |
| FJ-135 | `forjar trace` CLI command — view trace provenance data (text + JSON), machine filtering, grouped by trace_id, sorted by logical clock. Removes dead `_total_mean` variable. 6 tests, book updates. 1301→1307 tests. | **Done** |
| FJ-136 | MCP trace+anomaly handlers — `forjar_trace` and `forjar_anomaly` MCP tool handlers, TraceHandler reads trace.jsonl + AnomalyHandler reads events.jsonl with ML detection, 7 new tests, book updates. 1307→1314 tests. | **Done** |
| FJ-137 | Documentation sync — CLI command list (15→20), README resource table (4→9 types), spec §11 Cargo.toml sync (8→15 deps, rust-version 1.85→1.87), spec §1.3 dep count correction. | **Done** |
| FJ-138 | Performance benchmarks — Criterion benchmarks for spec §9 targets (validate 62µs, plan 84µs, drift 356µs), validate scaling (5/20/50/100 resources), binary 13MB, cold start 1.8ms. Book Ch. 10 benchmark docs with regression detection workflow. | **Done** |
| FJ-139 | `forjar bench` CLI command — inline performance benchmarks (validate, plan, drift, BLAKE3), `--iterations` and `--json` flags, CleanupGuard tempdir. Stale Phase labels fixed in spec deps/design principles/module tree. Book Ch. 6 bench docs + MCP tool count 7→9. 2 tests, 1314→1316. | **Done** |
| FJ-140 | Dogfood coverage — 3 new configs (dogfood-service, dogfood-mount, dogfood-network) covering all 9 resource types. Spec §10.6 rewritten (3→13 configs). README test count 254→1316. All 13 dogfood configs validate, all 19 examples pass. | **Done** |
| FJ-141 | Documentation polish — `examples/README.md` index (19 examples + 13 dogfood configs). Cookbook chapter expanded (897→1098 lines): disaster recovery, secret management, performance monitoring, trace auditing, resource tagging patterns. Book testing chapter: 8→13 dogfood configs, 15→19 examples. | **Done** |
| FJ-142 | Type system polish + MCP schema export. Display impls for MachineTarget ("web1", "[web1, web2]") and FailurePolicy ("stop_on_first", "continue_independent"), PartialEq/Eq for MachineTarget. `forjar mcp --schema` exports 9 tool JSON schemas. `docs/mcp-schema.json` generated. 11 new tests, 1316→1327. | **Done** |
| FJ-143 | Spec/README sync — README dep count 6→14, spec §2.2 CLI list 18→21 commands (add trace, migrate, bench), core module tree add purifier.rs + migrate.rs. | **Done** |
| FJ-144 | Getting-started tutorials for trace/migrate/bench (980→1061 lines). CI workflow: add dogfood validation (13 configs), example runner (19 examples), MCP schema check, bench compilation. 5 CI jobs (test, container-test, fmt, dogfood, bench). | **Done** |
| FJ-145 | Add missing `bench_spec9_apply_no_changes` Criterion benchmark (194µs, < 500ms target, 2577x margin). All 5 spec §9 targets now have benchmarks. CI jobs documented in book Ch. 10. README test count 1316→1327. | **Done** |
| FJ-146 | Fix 8→9 resource types in parser test + spec FJ-113 ticket description. Rustfmt cleanup on bench and CLI. | **Done** |
| FJ-147 | Fix dep count 14→15 in spec §1.3 + README. Update README C9 falsifiable claim (10→20 threshold). Expand §1.3 competitive table with 6 new feature rows (secrets, conditionals, multi-env, state surgery, policy, rolling deploy) showing roadmap. Add Phases 7-9 roadmap to spec (FJ-200 through FJ-226). | **Done** |
| FJ-148 | Spec §7.1 commands block: add trace/migrate/mcp/bench (17→21 commands). Add §7.15-7.18 CLI subsections. Fix §4.1 state tree: remove unimplemented `snapshots/`, add `trace.jsonl`. Fix Phase 4 table missing `Status` column. Fix Kani "Phase 2" → "Deferred". | **Done** |
| FJ-149 | Book Ch. 5 module map: add mcp/mod.rs, migrate.rs, pepita.rs (3 missing modules). CLI description 6→21 subcommands. MCP diagram: add TraceHandler + AnomalyHandler (7→9 handlers). Ch. 8 state directory: add trace.jsonl. | **Done** |
| FJ-150 | Examples README: add supporting assets section documenting files/ and recipes/ subdirs. | **Done** |
| FJ-151 | Fix 6 stale counts across book + spec: pepita in book/README, examples 15→19, tests ~700→~1200, Kani Deferred, trace.jsonl in state hierarchy, spec exclude list sync. | **Done** |

### Phase 7: Secrets & Conditionals (v0.7)

**Goal**: Close the two largest feature gaps vs. Terraform/Ansible. Encrypted secrets make forjar usable in real teams. Conditional resources eliminate config duplication.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-200 | `core/secrets.rs` — age-encrypted secret values. `forjar secrets encrypt/decrypt/edit/rekey` CLI. Secrets stored as `ENC[age,...]` markers in forjar.yaml, decrypted at resolve time. Identity from `FORJAR_AGE_KEY` env var or `--identity` flag. Replaces env-var-only `{{secrets.*}}` with encrypted-at-rest values committed to git. | Planned |
| FJ-201 | Secret rotation helpers — `forjar secrets rotate --re-encrypt` re-encrypts all values with a new key. `--recipients` for multi-recipient (team) encryption. Audit log of secret access in events.jsonl. | Planned |
| FJ-202 | Conditional resources — `when:` field on resources. Expression language: `{{machine.arch}} == "x86_64"`, `{{params.env}} != "production"`, `{{machine.roles contains "gpu"}}`. Evaluated at resolve time, false resources excluded from DAG. | Planned |
| FJ-203 | `for_each:` on resources — instantiate a resource template per item. `for_each: {{params.users}}` expands `resource-{item}` per list entry. Works with `when:` for filtered iteration. | Planned |
| FJ-204 | `count:` on resources — numeric multiplier. `count: 3` creates `resource-0`, `resource-1`, `resource-2`. `{{index}}` template variable available inside counted resources. | Planned |
| FJ-205 | `--json` output for plan/apply/drift/status — structured machine-readable JSON on stdout. Plan JSON includes resource diffs, action types, dependency order. Apply JSON includes per-resource timing, exit codes, hashes. Drift JSON includes expected vs actual hashes. | Planned |

### Phase 8: Multi-Environment & State Surgery (v0.8)

**Goal**: Support real-world multi-environment workflows (dev/staging/prod) and state manipulation without re-applying.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-210 | Workspaces — `forjar workspace new/list/select/delete <name>`. Per-workspace state directory (`state/<workspace>/<machine>/`). `{{workspace}}` template variable. Config-level `environments:` block with per-env param overrides. `-w <name>` flag on plan/apply/drift. | Planned |
| FJ-211 | Environment variable files — `env_file: envs/production.yaml` field on workspace. Loads param overrides from external YAML. Supports `--env-file` CLI override. | Planned |
| FJ-212 | `forjar state mv <old-id> <new-id>` — rename a resource in state without re-applying. Updates lock file resource key, preserves hash and metadata. Validates new ID doesn't conflict. | Planned |
| FJ-213 | `forjar state rm <resource-id>` — remove a resource from state without destroying it on the machine. Warns if other resources depend on it. `--force` to skip dependency check. | Planned |
| FJ-214 | `forjar state list` — tabular view of all resources in state with type, status, hash prefix, last applied timestamp. `--machine` filter. `--json` output. | Planned |
| FJ-215 | Output values — `outputs:` top-level block in forjar.yaml. `forjar output <key>` CLI. Cross-config references via `forjar output --config other.yaml <key>`. Outputs written to `state/outputs.yaml`. | Planned |
| FJ-216 | Parallel intra-machine execution — resources within the same machine that have no dependency relationship execute concurrently via `std::thread::scope`. Respects DAG: only independent siblings run in parallel. `policy.parallel_resources: true` (default: false). | Planned |

### Phase 9: Policy & Fleet Operations (v0.9)

**Goal**: Policy-as-code enforcement at plan time. Rolling deploys across machine fleets. External data lookups.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-220 | Policy-as-code — `policies:` top-level block in forjar.yaml. YAML-native rules evaluated at plan time before apply. Rule types: `require` (resource must have field), `deny` (block if condition true), `warn` (advisory). `forjar policy check` CLI. Blocks apply on `deny` violations. | Planned |
| FJ-221 | Built-in policy rules — `no_root_owner` (files must not be owned by root unless tagged `system`), `require_tags` (all resources must have tags), `no_privileged_containers`, `require_ssh_key` (machines must have ssh_key). Shipped as `forjar lint --strict`. | Planned |
| FJ-222 | Rolling deploys — `policy.serial: N` applies to N machines at a time, waiting for convergence before advancing. `policy.max_fail_percentage: 20` aborts the rollout if failure rate exceeds threshold. Compatible with `parallel_machines: true` (serial controls batch size). | Planned |
| FJ-223 | Data sources — `data:` top-level block. `type: file` reads local file content. `type: command` runs shell command, captures stdout. `type: dns` resolves hostname. Available as `{{data.key}}` in templates. Evaluated once at resolve time, cached for the run. | Planned |
| FJ-224 | General-purpose triggers — `triggers:` field on any resource (not just `restart_on` on services). When a dependency resource changes, triggers force re-apply of the dependent. `triggers: [config-file]` on a docker resource restarts the container when the config changes. | Planned |
| FJ-225 | Notification hooks — `policy.notify:` block. `on_success`, `on_failure`, `on_drift` keys. Value is a shell command template with `{{machine}}`, `{{resource}}`, `{{status}}` variables. Runs after apply/drift completes. Supports webhook via `curl` in the command. | Planned |
| FJ-226 | `--check` mode parity — all 9 resource type codegen handlers emit check-mode scripts (report what would change without applying). `forjar apply --check` runs check scripts instead of apply scripts. Exit code 2 = changes needed, 0 = converged. | Planned |
| FJ-230 | Pepita transport — kernel namespace execution target. `transport: pepita` on machines. Uses `unshare(2)` / `clone(2)` with `CLONE_NEWPID \| CLONE_NEWNET \| CLONE_NEWNS` to create isolated execution namespace, pipes purified shell to `nsenter ... bash` stdin (same mechanism as container/SSH/local). **Lifecycle**: (1) create namespace with cgroup v2 limits from `pepita:` config, (2) mount overlayfs rootfs from debootstrap base or OCI image, (3) exec scripts via `nsenter --target <pid> -- bash`, (4) teardown namespace. **Config**: `pepita.rootfs` (path to base rootfs or `debootstrap:jammy`), `pepita.cgroups.memory_mb`, `pepita.cgroups.cpus`, `pepita.network` (`isolated` \| `host`), `pepita.filesystem` (`overlay` \| `bind`), `pepita.ephemeral` (destroy after apply, default true). **Interface model**: mirrors wos `VmConfig`/`VmManager` patterns — `VmState` lifecycle (Created→Running→Stopped), `VirtioDeviceConfig`-style device enum for console/block/net, jidoka guard (`MAX_NAMESPACES`). Zero Docker dependency — uses kernel primitives directly. Requires `CAP_SYS_ADMIN` or root. `transport/pepita.rs` new file. Extends transport dispatch table to 4 transports (pepita > container > local > SSH). Dogfood: `dogfood-pepita-transport.yaml` exercises all resource types inside a kernel namespace. | Planned |

### Phase 10: Sovereign AI Stack (v1.0)

**Goal**: First-class provisioning of the sovereign AI stack (aprender, repartir, pepita, renacer, copia, wos) on bare-metal GPU machines. New resource types for ML models and GPU hardware. Delta sync for large file transfer. Recipes that bundle the full inference-server and distributed-training workflows into reusable patterns.

**Context**: The sovereign AI stack is a set of pure-Rust crates that run ML inference and training on bare metal without cloud APIs:

```
apr-cookbook (deployment patterns)
  ├── aprender / apr-cli    ML framework + inference server (apr serve)
  ├── trueno                SIMD/GPU tensor compute (AVX-512, NEON, wgpu)
  ├── realizar              GPU kernels (FlashAttention, Q4K/Q5K quantization)
  ├── whisper-apr           WASM-first speech recognition
  ├── repartir              Distributed execution (work-stealing, TCP/TLS, MicroVM)
  │   └── pepita            Kernel interfaces (KVM, io_uring, cgroups, zram, SIMD)
  ├── renacer               Syscall tracing (ptrace + DWARF + OTLP + ML anomaly)
  ├── copia                 Delta sync (rsync algorithm, BLAKE3 + Adler-32)
  └── wos                   WASM microkernel (educational, browser-based)
```

Forjar provisions the machines these crates run on. Phase 10 makes that provisioning first-class.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-240 | `resources/model.rs` — ML model resource type. Downloads models via `apr pull`, verifies integrity with BLAKE3, manages model cache directory. **Schema**: `type: model`, `name`, `source` (HuggingFace repo ID, URL, or local path), `format` (gguf \| safetensors \| apr), `quantization` (q4_k_m \| q5_k_m \| q8_0 \| f16 \| none), `path` (destination on machine), `checksum` (optional BLAKE3 hash for pinning), `cache_dir` (default: `~/.cache/apr/`). **Codegen**: check if model exists + hash matches → skip; else `apr pull` or `curl` + verify. **State query**: file exists + BLAKE3 hash match. Enables drift detection on model files — unauthorized model swaps are caught. | Planned |
| FJ-241 | `resources/gpu.rs` — GPU hardware resource type. Manages NVIDIA driver installation, CUDA toolkit, nvidia-persistenced service, GPU cgroup limits, and device verification. **Schema**: `type: gpu`, `driver_version` (e.g., `535`), `cuda_version` (e.g., `12.3`), `devices` (list of GPU indices, default: all), `persistence_mode` (bool, default: true), `compute_mode` (default \| exclusive_process \| prohibited), `memory_limit_mb` (optional cgroup GPU memory limit). **Codegen**: apt install `nvidia-driver-{version}` + `cuda-toolkit-{cuda_version}`, enable `nvidia-persistenced`, verify with `nvidia-smi`. **State query**: `nvidia-smi --query-gpu=driver_version,compute_mode,memory.total --format=csv,noheader`. Extends resource type count from 9 to 11 (with model). | Planned |
| FJ-242 | Copia delta sync integration — replace base64 file transfer for `source:` fields with copia rsync-algorithm delta sync. Add `copia` dependency to Cargo.toml. **Mechanism**: (1) generate remote file signature via transport (`copia signature` on target), (2) compute delta locally (`copia delta`), (3) transfer delta (much smaller than full file), (4) apply patch on target (`copia patch`). Falls back to base64 for new files (no remote signature to diff against). **Threshold**: files > 1MB use copia; files <= 1MB use base64 (overhead not worth it). Critical for deploying 4-7GB GGUF model files — base64 doubles transfer size and requires full re-transfer on any change. | Planned |
| FJ-243 | `recipes/apr-inference-server.yaml` — reusable recipe for deploying an aprender inference server on a GPU machine. **Inputs**: `model_source` (HF repo ID), `model_format` (gguf \| safetensors), `quantization` (q4_k_m default), `port` (8080 default), `workers` (1 default), `gpu_device` (0 default), `user` (service account). **Resources**: gpu (driver + CUDA), model (download + verify), file (systemd unit for `apr serve`), service (running + enabled), network (firewall allow port), cron (daily `apr qa` health check). Depends on FJ-240 + FJ-241. Dogfood: `examples/dogfood-apr-serve.yaml`. | Planned |
| FJ-244 | `recipes/repartir-worker.yaml` — reusable recipe for deploying a repartir remote TCP/TLS executor as a systemd service on worker nodes. **Inputs**: `listen_port` (9000 default), `tls_cert` (path to TLS cert), `tls_key` (path to TLS key), `max_tasks` (concurrent task limit), `backends` (cpu \| gpu \| microvm). **Resources**: package (cargo install repartir-worker), file (systemd unit, TLS certs, config), service (running + enabled), network (firewall allow listen_port). For distributed inference: deploy this recipe to N worker machines, then deploy apr-inference-server with `--distributed` flag pointing to the workers. | Planned |
| FJ-245 | `recipes/renacer-observability.yaml` — reusable recipe for deploying renacer syscall tracing + OTLP export + Grafana stack on a monitoring machine. **Inputs**: `otlp_endpoint`, `grafana_port` (3000 default), `jaeger_port` (16686 default), `retention_days` (7 default). **Resources**: package (cargo install renacer), docker (jaeger all-in-one), docker (grafana with tempo datasource), network (firewall allow grafana + jaeger + OTLP ports), file (grafana provisioning config). Enables: `renacer --otlp-endpoint http://monitor:4317 -- forjar apply` for traced infrastructure operations. `forjar trace` output can also be forwarded to the same OTLP endpoint. | Planned |
| FJ-246 | `recipes/sovereign-ai-stack.yaml` — meta-recipe that composes FJ-243 + FJ-244 + FJ-245 into a complete sovereign AI lab deployment. **Inputs**: `gpu_machines` (list), `worker_machines` (list), `monitor_machine`, `model_source`, `model_format`. **Composition**: (1) deploy renacer observability to monitor machine, (2) deploy repartir workers to worker machines, (3) deploy apr inference server to GPU machines with distributed flag pointing to workers. Uses recipe composition (`requires:`) to chain dependencies. The forjar.yaml example from §3.1 (lambda + intel + jetson) becomes a concrete instance of this meta-recipe. | Planned |
| FJ-247 | Copia-accelerated model deployment benchmarks — measure and document delta sync performance for ML model updates. **Targets**: model weight update (fine-tuned 7B, ~2% delta) transfers < 100MB over SSH (vs 4GB full re-transfer). Quantization format change (f16→q4_k_m) requires full transfer (incompressible delta). Benchmark: copia vs base64 vs rsync for GGUF/SafeTensors files at 1GB, 4GB, 7GB sizes. Results documented in spec §9 performance targets and book Ch. 10. | Planned |
| FJ-248 | Dogfood: full sovereign AI stack on lambda + intel + jetson. **Validation**: (1) `forjar validate` all stack recipes, (2) `forjar plan` shows GPU driver + CUDA + model + apr-serve + repartir-worker + renacer, (3) container transport apply for codegen verification, (4) drift detection catches model file tampering (BLAKE3 mismatch), (5) anomaly detection flags GPU driver version skew across machines. CI: add `sovereign-stack` job to `.github/workflows/ci.yml`. | Planned |

---

## 9. Performance Targets

| Operation | Target | Rationale |
|-----------|--------|-----------|
| `forjar validate` | < 10ms | Pure YAML parse, no I/O |
| `forjar plan` (3 machines, 20 resources) | < 2s | Parallel SSH + BLAKE3 hash |
| `forjar drift` (3 machines, 100 files) | < 1s | BLAKE3 is 4GB/s on modern CPUs |
| `forjar apply` (no changes) | < 500ms | Hash compare only, no shell exec |
| Binary size | < 15MB | Single static binary (MCP/tokio adds ~3MB over core) |
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
3. **Bounded model checking**: Kani harnesses for pure-functional invariants (deferred — not yet scheduled)

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

13 dogfood configs exercise all 9 resource types and cross-cutting features. Container transport configs enable end-to-end testing without root or host pollution; localhost configs validate codegen and planning.

| Config | Resource types | What it proves |
|--------|---------------|----------------|
| `dogfood-container.yaml` | file | File codegen, state hashing, lock persistence, container transport |
| `dogfood-packages.yaml` | package, file | Package codegen, cross-resource dependencies, idempotency |
| `dogfood-phase2.yaml` | user, file, cron | User management, cron scheduling, dependency DAG, base64 source transfer |
| `dogfood-service.yaml` | service, file | Systemd service lifecycle, restart_on triggers, enabled/disabled states |
| `dogfood-mount.yaml` | mount | NFS, bind, tmpfs mounts; fstab management; absent state cleanup |
| `dogfood-network.yaml` | network | UFW firewall rules; allow/deny/absent; source CIDR filtering |
| `dogfood-pepita.yaml` | pepita | Kernel namespace isolation: cgroups, netns, overlay, seccomp, chroot |
| `dogfood-migrate.yaml` | docker, package | Docker container resources, migration to pepita |
| `dogfood-recipe.yaml` | package, recipe | Recipe expansion, composite resource composition |
| `dogfood-crossarch.yaml` | file | Multi-machine targeting, architecture filtering (x86_64/aarch64) |
| `dogfood-tags.yaml` | file | Resource tagging, tag-filtered operations |
| `dogfood-secrets.yaml` | file | Template interpolation with `{{params.*}}` secrets |
| `dogfood-hooks.yaml` | file | Pre/post apply hooks, lifecycle callbacks |

**Dogfood verification workflow** (run after any codegen, transport, or executor change):

```bash
# 1. Build test target (for container transport configs)
docker build -t forjar-test-target -f tests/Dockerfile.test-target .

# 2. Validate all configs
for f in examples/dogfood-*.yaml; do cargo run -- validate -f "$f"; done

# 3. First apply — all resources converge
cargo run -- apply -f examples/dogfood-phase2.yaml --state-dir /tmp/dogfood-state

# 4. Idempotency proof — second apply, zero changes
cargo run -- apply -f examples/dogfood-phase2.yaml --state-dir /tmp/dogfood-state

# 5. Drift detection — verify lock state matches live state
cargo run -- drift -f examples/dogfood-phase2.yaml --state-dir /tmp/dogfood-state

# 6. Destroy — reverse teardown and state cleanup
cargo run -- destroy -f examples/dogfood-phase2.yaml --state-dir /tmp/dogfood-state --yes
```

**Resource type coverage**:

All 9 resource types (`file`, `package`, `service`, `mount`, `user`, `docker`, `cron`, `network`, `pepita`) plus the `recipe` composite type have dedicated dogfood configs validating their codegen, state queries, and edge cases.

---

## 11. Project Bootstrap

```toml
# Cargo.toml
[package]
name = "forjar"
version = "0.1.0"
edition = "2021"
rust-version = "1.87.0"
authors = ["Pragmatic AI Labs"]
description = "Rust-native Infrastructure as Code — bare-metal first, BLAKE3 state, provenance tracing"
license = "MIT OR Apache-2.0"
repository = "https://github.com/paiml/forjar"
homepage = "https://paiml.com"
keywords = ["iac", "infrastructure", "devops", "provisioning", "bare-metal"]
categories = ["command-line-utilities", "development-tools"]
exclude = ["benches/", ".pmat/", "state/", "docs/", "examples/", "target/", ".vscode/", ".idea/", "proptest-regressions/", "*.profraw", "*.profdata"]

[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
all = { level = "warn", priority = -1 }

[dependencies]
blake3 = "1.8"
serde = { version = "1.0", features = ["derive"] }
serde_yaml_ng = "0.10"
serde_json = "1.0"
clap = { version = "4", features = ["derive"] }
indexmap = { version = "2.7", features = ["serde"] }
base64 = "0.22.1"
bashrs = "6.64.0"                   # Shell script validation + purification (FJ-036)
pforge-runtime = "0.1.4"            # MCP server runtime (FJ-063)
pforge-config = "0.1.4"             # MCP configuration types (FJ-063)
tokio = { version = "1.35", features = ["rt-multi-thread", "macros"] }  # Async runtime for MCP
async-trait = "0.1"                 # Async trait support for MCP handlers
schemars = { version = "0.8", features = ["derive"] }  # JSON schema for MCP tool inputs
rustc-hash = "2"                    # Fast hashing for MCP config maps
provable-contracts-macros = { path = "../provable-contracts/crates/provable-contracts-macros" }

[build-dependencies]
provable-contracts = { path = "../provable-contracts/crates/provable-contracts" }

[dev-dependencies]
tempfile = "3"
criterion = { version = "0.5", features = ["html_reports"] }
proptest = "1"

[[bench]]
name = "core_bench"
harness = false

[features]
container-test = []

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
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
| I8 | All shell is bashrs-validated; `forjar lint` reports bashrs diagnostics | codegen + purifier | — |
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
| L3 | Kani bounded model checking | Deferred (`cargo kani`) |

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
