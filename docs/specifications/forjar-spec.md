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
| External deps | ~200 Go modules | ~500 npm/pip packages | ~50 Python packages | **16 crates** |
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
    mod.rs              Subcommand dispatch (init, validate, plan, apply, drift, status, history, destroy, import, show, graph, check, diff, fmt, lint, rollback, anomaly, trace, migrate, mcp, bench, state-list, state-mv, state-rm, output, schema)
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
    conditions.rs       When-expression evaluation for conditional resources (FJ-202)
    secrets.rs          Age-encrypted secret values, ENC[age,...] markers (FJ-200)
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
    model.rs            ML model download/verification (FJ-240)
    gpu.rs              GPU hardware management (FJ-241)
  copia/
    mod.rs              Delta sync — BLAKE3 block hashing, delta computation, patch scripts (FJ-242)
  transport/
    mod.rs              Transport abstraction + dispatch
    local.rs            Local execution (this machine)
    ssh.rs              SSH execution (remote machines)
    container.rs        Container execution (docker/podman exec)
    pepita.rs           Kernel namespace execution (FJ-230)
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

**Done (Phase 10)**: copia (delta sync — replaces base64 for files > 1MB via BLAKE3 per-block hashing, FJ-242).

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

  # NVIDIA GPU container (CUDA workloads — model QA, inference testing)
  gpu-cuda:
    hostname: gpu-cuda
    addr: container
    transport: container
    container:
      runtime: docker
      image: nvidia/cuda:12.4.1-runtime-ubuntu22.04
      name: forjar-cuda
      gpus: all                          # --gpus flag (NVIDIA Container Toolkit)
      env:
        CUDA_VISIBLE_DEVICES: "0,1"

  # AMD ROCm GPU container (HIP/ROCm workloads)
  gpu-rocm:
    hostname: gpu-rocm
    addr: container
    transport: container
    container:
      runtime: docker
      image: rocm/dev-ubuntu-22.04:6.1
      name: forjar-rocm
      devices:                           # --device passthrough
        - /dev/kfd
        - /dev/dri
      group_add:                         # --group-add for device access
        - video
        - render
      env:
        ROCR_VISIBLE_DEVICES: "0"

  # Pepita kernel namespace target (zero Docker dependency, FJ-230)
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

#### `model` (Done — FJ-240)

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

#### `gpu` (Done — FJ-241)

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

#### Common Resource Fields

All resource types support the following optional fields in addition to their type-specific fields:

| Field | Type | Description |
|-------|------|-------------|
| `depends_on` | list | Resources that must be applied before this one |
| `tags` | list | Arbitrary tags for filtering (`--tag` on plan/apply/check) |
| `when` | string | Conditional expression — resource is skipped if false |
| `for_each` | list | Expand resource per item (`{{item}}` template) |
| `count` | integer | Expand resource N times (`{{index}}` template) |
| `arch` | list | Architecture filter — resource only applies on matching machines |
| `pre_apply` | string | Shell command run on the target machine **before** the main apply script. If it exits non-zero, the resource is skipped (apply does not run). Use case: backup a config file before overwrite. |
| `post_apply` | string | Shell command run on the target machine **after** a successful apply script. If it exits non-zero, the resource is marked as failed. Use case: restart a service after deploying its config. |

```yaml
# Lifecycle hooks example
resources:
  nginx-config:
    type: file
    machine: web1
    path: /etc/nginx/sites-enabled/app
    content: |
      server { listen 80; }
    pre_apply: "cp /etc/nginx/sites-enabled/app /etc/nginx/sites-enabled/app.bak"
    post_apply: "systemctl reload nginx"
    depends_on: [nginx-pkg]
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
| `container.gpus` | string | — | NVIDIA GPU passthrough via `--gpus` (e.g., `"all"`, `"device=0"`) |
| `container.devices` | [string] | `[]` | Device passthrough via `--device` (e.g., `["/dev/kfd", "/dev/dri"]` for AMD ROCm) |
| `container.group_add` | [string] | `[]` | Additional groups via `--group-add` (e.g., `["video", "render"]` for GPU device access) |
| `container.env` | map | `{}` | Environment variables via `--env` (e.g., `CUDA_VISIBLE_DEVICES`, `ROCR_VISIBLE_DEVICES`) |
| `pepita` | object | — | Pepita namespace config (required when `transport: pepita`). Done: FJ-230. |
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
| 1 | `transport: pepita` | Pepita | `nsenter --target <pid> -- bash` (stdin pipe) | Done (FJ-230) |
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

#### Pepita Transport (Done — FJ-230)

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
  schema      Export JSON Schema for forjar.yaml
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
  --json                 Output apply results as JSON
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

### 7.19 `forjar status`

```
forjar status [OPTIONS]

Options:
  --state-dir <PATH>     State directory (default: state)
  -m, --machine <NAME>   Show specific machine only
  --json                 Output status as JSON
```

Shows current state from lock files: project name, last apply, per-machine resource status, types, durations. With `--json`, outputs all state as structured JSON for scripting/CI integration.

### 7.20 `forjar schema`

```
forjar schema
```

Exports a JSON Schema for `forjar.yaml` to stdout. The schema is generated from the `ForjarConfig` struct via code (not serde derive). Validates machine, resource, and policy schemas. No arguments required.

Use cases:
- **IDE autocomplete**: Point the VS Code YAML extension at the schema for inline validation and completion
- **External validation**: Pipe config through `jsonschema` or similar tools in CI
- **Documentation**: Machine-readable description of every field, type, and constraint

```bash
# Write schema to file
forjar schema > forjar-schema.json

# Use with VS Code YAML extension (add to .vscode/settings.json):
# "yaml.schemas": { "./forjar-schema.json": "forjar.yaml" }
```

---

## 8. Phased Implementation

### Priority 0: Multi-Vendor GPU Container Transport (v0.x) — Highest Priority

**Goal**: First-class NVIDIA CUDA, AMD ROCm, and Intel GPU passthrough in container transport — enabling `apr-model-qa-playbook` multi-GPU model QA workflows via forjar.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-738 | Multi-vendor GPU container transport — `container.devices` (--device), `container.group_add` (--group-add), `container.env` (--env) fields on ContainerConfig. NVIDIA via `--gpus`, AMD ROCm via `/dev/kfd` + `/dev/dri` + `video`/`render` groups, Intel via `/dev/dri`. Dogfood: `dogfood-multi-gpu.yaml` (2 machines, 5 resources). 2 new transport tests. | **Done** |
| FJ-739 | GPU container integration tests — feature-gated `--features gpu-container-test`. NVIDIA: verify `nvidia-smi` in container. AMD: verify `/dev/kfd` + `/dev/dri` accessible. Cross-vendor: same model config deployed to both. 7 tests. | **Done** |
| FJ-740 | `apr-model-qa-playbook` integration — `apr-model-qa` recipe (5 resources: workspace, model dir, playbook config, test runner, results dir). Dogfood: `dogfood-apr-qa.yaml` (2 machines × 5 resources = 10 resources). Recipe inputs: model_repo, format, quantization, gpu_vendor, backends, modalities, scenario_count. | **Done** |

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
| FJ-152 | Fix stale "Phase 3+" → "future version" in book FAQ. Add FJ-151 ticket to spec. | **Done** |
| FJ-153 | Coverage recovery: 45 new tests across 9 files (docker, mount, service, user, purifier, local, cron, network, file). Tests 1327→1372. Line coverage 96.09% (exceeds 95% target). | **Done** |

### Phase 7: Secrets & Conditionals (v0.7)

**Goal**: Close the two largest feature gaps vs. Terraform/Ansible. Encrypted secrets make forjar usable in real teams. Conditional resources eliminate config duplication.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-200 | `core/secrets.rs` — age-encrypted secret values. `forjar secrets encrypt/decrypt/keygen/view/rekey` CLI. Secrets stored as `ENC[age,...]` markers in forjar.yaml, decrypted at resolve time. Identity from `FORJAR_AGE_KEY` env var or `--identity` flag. Coexists with `{{secrets.*}}` env-var approach. 29 tests, dogfood-age-secrets.yaml. | **Done** |
| FJ-201 | Secret rotation helpers — `forjar secrets rotate --re-encrypt` re-encrypts all markers with new recipients. Multi-recipient encryption via `-r` flags. `SecretAccessed` + `SecretRotated` audit events in events.jsonl. `--re-encrypt` safety flag prevents accidental rotation. 7 tests. | **Done** |
| FJ-202 | Conditional resources — `when:` field on resources. Expression language: `{{machine.arch}} == "x86_64"`, `{{params.env}} != "production"`, `{{machine.roles contains "gpu"}}`. Evaluated at plan time, false resources excluded from plan + execution. New `core/conditions.rs` module. 28 tests, dogfood-conditions.yaml (14th config). | **Done** |
| FJ-203 | `for_each:` on resources — instantiate a resource template per item. `for_each: [alice, bob]` expands `resource-alice`, `resource-bob`. `{{item}}` template resolved in all string fields. Deps referencing expanded resources rewritten to last copy. 20 tests (parser + planner integration), dogfood-iteration.yaml (15th config). | **Done** |
| FJ-204 | `count:` on resources — numeric multiplier. `count: 3` creates `resource-0`, `resource-1`, `resource-2`. `{{index}}` template resolved in all string fields. Validation rejects count: 0 and count + for_each on same resource. expand_resources() in parser.rs runs after expand_recipes(). 20 tests, dogfood-iteration.yaml. | **Done** |
| FJ-205 | `--json` output for plan/apply/drift/status — structured machine-readable JSON on stdout. Plan JSON includes resource diffs, action types, dependency order. Apply JSON includes per-resource timing, exit codes, hashes. Drift JSON includes expected vs actual hashes. | **Done** |

### Phase 8: Multi-Environment & State Surgery (v0.8)

**Goal**: Support real-world multi-environment workflows (dev/staging/prod) and state manipulation without re-applying.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-210 | Workspaces — `forjar workspace new/list/select/delete/current`. Per-workspace state directory (`state/<workspace>/<machine>/`). `{{params.workspace}}` template variable injected automatically. `-w <name>` flag on plan/apply/drift. Active workspace stored in `.forjar/workspace`. Resolve chain: `-w` flag > `.forjar/workspace` > "default". 13 tests. | **Done** |
| FJ-211 | Environment variable files — `--env-file <path>` flag on plan/apply/drift. Loads param overrides from external YAML file. Env file params override config defaults; `--param` CLI flags override env file. `load_env_params()` helper. 8 tests. | **Done** |
| FJ-212 | `forjar state-mv <old-id> <new-id>` — rename a resource in state without re-applying. Updates lock file resource key, preserves hash and metadata. Validates new ID doesn't conflict. `--machine` filter. 6 tests. | **Done** |
| FJ-213 | `forjar state-rm <resource-id>` — remove a resource from state without destroying it on the machine. Warns if other resources reference it via details. `--force` to skip dependency check. `--machine` filter. 5 tests. | **Done** |
| FJ-214 | `forjar state-list` — tabular view of all resources in state with type, status, hash prefix, last applied timestamp. `--machine` filter. `--json` output. 6 tests. | **Done** |
| FJ-215 | Output values — `outputs:` top-level block in forjar.yaml with `value:` (template) and `description:` fields. `forjar output` CLI shows all resolved outputs; `forjar output <key>` shows one value. Template resolution via `{{params.*}}` and `{{machine.NAME.FIELD}}`. `--json` output. 7 tests, dogfood-outputs.yaml (16th config). | **Done** |
| FJ-216 | Parallel intra-machine execution — resources grouped into DAG-level waves via `compute_parallel_waves()`. All resources in a wave have no inter-dependencies. Wave-based execution in `apply_machine` with jidoka break support. `policy.parallel_resources: true` (default: false). `compute_resource_waves()` handles per-machine subsets. 9 tests (4 resolver + 5 executor). | **Done** |

### Phase 9: Policy & Fleet Operations (v0.9)

**Goal**: Policy-as-code enforcement at plan time. Rolling deploys across machine fleets. External data lookups.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-220 | Policy-as-code — `policies:` top-level block in forjar.yaml. YAML-native rules evaluated at plan time before apply. Rule types: `require` (resource must have field), `deny` (block if condition true), `warn` (advisory). `forjar policy` CLI with `--json`. Blocks apply on deny/require violations. Filters by `resource_type` and `tag`. 9 parser tests + 4 CLI tests = 13 total. | **Done** |
| FJ-221 | Built-in policy rules — `forjar lint --strict` adds 4 hardcoded rules: `no_root_owner` (files owned by root must be tagged `system`), `require_tags` (all resources must have tags), `no_privileged_containers` (container machines must not use `--privileged`), `require_ssh_key` (non-local machines must have `ssh_key`). Violations reported as warnings (same format as user-defined policies). `--json` output supported. 6 CLI tests. | **Done** |
| FJ-222 | Rolling deploys — `policy.serial: N` applies to N machines at a time, waiting for convergence before advancing to the next batch. `policy.max_fail_percentage: P` aborts the rollout if cumulative failure rate exceeds threshold (checked after each batch). Compatible with `parallel_machines: true` (serial controls batch size, parallel controls intra-batch concurrency). `apply_machines_rolling()` in executor.rs dispatches to parallel or sequential per batch. 5 executor tests. | **Done** |
| FJ-223 | Data sources — `data:` top-level block. `type: file` reads local file content. `type: command` runs shell command, captures stdout. `type: dns` resolves hostname. Available as `{{data.key}}` in templates. Evaluated once at resolve time, cached for the run. Data values stored in params with `__data__` prefix for template resolution. 8 resolver tests. Dogfood: `dogfood-data.yaml`. | **Done** |
| FJ-224 | General-purpose triggers — `triggers:` field on any resource (not just `restart_on` on services). When a dependency resource converges during apply, triggers force re-apply of the dependent even if unchanged. Validation: triggers must reference existing resources, no self-reference. Executor tracks `converged_resources: HashSet<String>` per machine, checks triggers before NoOp. 4 parser tests + 4 executor tests = 8 total. | **Done** |
| FJ-225 | Notification hooks — `policy.notify:` block with `on_success`, `on_failure`, `on_drift` keys. Shell command templates with `{{machine}}`, `{{converged}}`, `{{unchanged}}`, `{{failed}}`, `{{drift_count}}` variables. `run_notify()` helper with template expansion. Runs after apply/drift completes, per machine. Failures are warnings (don't block). 6 CLI tests. | **Done** |
| FJ-226 | `--check` mode parity — `forjar apply --check` delegates to `cmd_check` which runs check scripts for all 9 resource types (already implemented). Check scripts exist for package, file, service, mount, user, docker, pepita, cron, network. `--check` flag on Apply command struct. 2 CLI tests. | **Done** |
| FJ-230 | Pepita transport — kernel namespace execution target. `transport: pepita` on machines. `PepitaTransportConfig` struct: `rootfs`, `memory_mb`, `cpus`, `network` (isolated\|host), `filesystem` (overlay\|bind), `ephemeral`. Uses `unshare(1)` + `nsenter(1)` with PID+mount+net namespaces. `transport/pepita.rs`: `exec_pepita()` (nsenter + bash stdin), `ensure_namespace()` (unshare + cgroup limits), `cleanup_namespace()` (kill + pidfile cleanup). Transport dispatch priority: pepita > container > local > SSH. `Machine.is_pepita_transport()` + `pepita_name()`. Zero Docker dependency — uses kernel primitives directly. Requires `CAP_SYS_ADMIN` or root. 10 transport tests. | **Done** |

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
| FJ-240 | `resources/model.rs` — ML model resource type. Downloads via `apr pull`, `huggingface-cli`, `curl` (URL), or `cp` (local path). BLAKE3 checksum verification for integrity pinning. **Schema**: `type: model`, `name`, `source`, `format` (gguf\|safetensors\|apr), `quantization`, `path`, `checksum`, `cache_dir`. **Codegen**: check (file exists + b3sum match), apply (download + verify + chown), state_query (size + blake3 hash). Parser validation: requires name, valid state (present\|absent). Resource type count: 9 → 10. 10 tests. | **Done** |
| FJ-241 | `resources/gpu.rs` — GPU hardware resource type. NVIDIA driver install (`nvidia-driver-{version}`), CUDA toolkit (`cuda-toolkit-{version}`), nvidia-persistenced enablement, compute mode setting via `nvidia-smi -c`. **Schema**: `type: gpu`, `driver_version`, `cuda_version`, `devices`, `persistence_mode` (default true), `compute_mode` (default\|exclusive_process\|prohibited), `gpu_memory_limit_mb`. State query: `nvidia-smi --query-gpu=driver_version,compute_mode,memory.total`. Parser validation: requires driver_version, valid state. Resource type count: 10 → 11. 10 tests. | **Done** |
| FJ-242 | Copia delta sync integration — rsync-style block-level delta sync for `source:` file transfers > 1MB. **Module**: `src/copia/mod.rs` — BLAKE3 per-block hashing (4KB blocks), delta computation (Copy/Literal ops), signature/patch shell script generation. **Executor integration**: two-phase protocol in `executor.rs` — (1) execute signature script on remote to get per-block BLAKE3 hashes, (2) compute delta locally comparing block hashes, (3) transfer only changed blocks as base64 literals + dd-copy unchanged blocks, (4) atomic file replacement via temp+mv. Falls back to full base64 for new files (no remote signatures) or files ≤ 1MB. Critical for deploying 4-7GB GGUF model files — only transfers changed blocks after fine-tuning (~2% delta). `parse_signatures()` handles both b3sum and sha256sum fallback from remote. 26 tests. | **Done** |
| FJ-243 | `recipes/apr-inference-server.yaml` — reusable recipe for deploying an aprender inference server on a GPU machine. **Inputs**: `model_source` (HF repo ID), `model_format` (gguf\|safetensors\|apr), `quantization` (q4_k_m default), `port` (8080 default), `workers` (1 default), `gpu_device` (0 default), `user` (apr default). **8 resources**: gpu-driver (NVIDIA 550 + CUDA 12.4), model-download (source + verify + cache), apr-service-user (system account), apr-data-dir (/opt/apr), apr-systemd-unit (ExecStart with all input params), apr-serve (running + enabled + restart triggers), apr-firewall (tcp allow), apr-health-check (curl /health every 6h). Dogfood: `examples/dogfood-apr-serve.yaml` — validates OK (1 machine, 8 resources), plan shows correct DAG order. | **Done** |
| FJ-244 | `recipes/repartir-worker.yaml` — reusable recipe for deploying repartir TCP/TLS executor on worker nodes. **Inputs**: `listen_port` (9000), `tls_cert`, `tls_key`, `max_tasks` (4), `backends` (cpu\|gpu\|microvm), `user` (repartir). **10 resources**: worker-user (system account), worker-dirs (/opt/repartir), worker-tls-dir (mode 0700), worker-pkg (cargo install), worker-config (YAML with listen/TLS/backends), worker-tls-cert (source deploy), worker-tls-key (source deploy, mode 0600), worker-systemd-unit (ExecStart + LimitNOFILE), worker-service (running + restart triggers), worker-firewall (tcp allow). Dogfood: `examples/dogfood-repartir.yaml` — validates OK (1 machine, 10 resources). | **Done** |
| FJ-245 | `recipes/renacer-observability.yaml` — observability stack recipe. **Inputs**: `otlp_endpoint` (localhost:4317), `grafana_port` (3000), `jaeger_port` (16686), `retention_days` (7), `user` (renacer). **10 resources**: obs-user (system account), obs-data-dir (/opt/renacer), obs-grafana-data (uid 472), renacer-pkg (cargo install), jaeger (all-in-one with OTLP + Badger storage), grafana-provisioning (Jaeger datasource YAML), grafana (10.3.1 with provisioning mount), firewall-grafana, firewall-jaeger, firewall-otlp. Enables `renacer --otlp-endpoint` traced operations. Dogfood: `examples/dogfood-renacer.yaml` — validates OK (1 machine, 10 resources). | **Done** |
| FJ-246 | `recipes/sovereign-ai-stack.yaml` — meta-recipe composing FJ-243 + FJ-244 + FJ-245. **Inputs**: `model_source`, `model_format` (gguf), `api_port` (8080), `worker_port` (9000), `grafana_port` (3000), `user` (forjar). **Coordination resources**: coord-user, coord-dir, fleet-inventory (stack config YAML), health-dashboard (checks API/GPU/workers/Grafana), health-cron (15-min interval). `requires:` declares composition deps. Dogfood: `examples/dogfood-sovereign-stack.yaml` — 3 machines (gpu-box, worker-1, monitor), 4 recipes (observability + worker + inference + coordination), 33 total resources, parallel execution. Validates and plans correctly with interleaved DAG ordering. | **Done** |
| FJ-247 | Copia delta sync Criterion benchmarks in `benches/core_bench.rs`. **Results**: signatures scale linearly (294µs/1MB, 1.19ms/4MB, 5.0ms/16MB → extrapolates to ~1.2s for 4GB). Delta computation: 1.18ms/4MB at 2% change, 1.26ms at 100% change (negligible overhead from hash comparison). Patch script generation: 60µs for 1MB at 10% change. Signature parsing: 57µs for 1024 blocks. For 4GB model with 2% delta: signature ~1.2s + delta ~1.2s + transfer ~80MB (vs 4GB full) + patch ~60ms ≈ 2.5s total overhead. **4 benchmark groups**: copia_signatures (1/4/16MB), copia_delta (2/10/50/100%), copia_patch_script, copia_parse_signatures. Results documented in spec §9. | **Done** |
| FJ-248 | Dogfood: full sovereign AI stack validation. **24 dogfood configs all validate** (dogfood-*.yaml). Sovereign stack: 3 machines (gpu-box, worker-1, monitor), 4 recipes (observability + worker + inference + coordination), 33 resources. `forjar plan` shows correct interleaved DAG ordering: GPU driver → model → systemd → service on gpu-box, TLS → config → systemd → service on worker-1, Jaeger → Grafana → health cron on monitor. All resources correctly namespaced (inference/, worker/, observability/, coordination/). Recipe composition works across machines via separate `type: recipe` resources with `parallel_machines: true`. | **Done** |

### Phase 11: Production Hardening (v1.1)

**Goal**: Close DX gaps and harden forjar for production fleet management. Template functions enable expressive configs without shell escapes. SSH multiplexing reduces connection overhead. Shell completions and `forjar doctor` improve onboarding. Config includes enable DRY multi-environment setups.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-250 | Template functions in `resolver.rs` — `{{upper(x)}}`, `{{lower(x)}}`, `{{default(x, "fallback")}}`, `{{trim(x)}}`, `{{replace(x, "old", "new")}}`, `{{env("HOME")}}`, `{{join(list, ",")}}`, `{{split(x, ",")}}`  , `{{b3sum(x)}}`. Expression parser handles `{{func(args)}}` syntax alongside `{{params.key}}`. Nested calls: `{{upper(params.name)}}`. Dogfood: dogfood-template-funcs.yaml. 26 tests (1620→1646). | **Done** |
| FJ-251 | `forjar doctor` — pre-flight system checker. Validates: bash ≥ 4.0, ssh available (if SSH machines configured), docker/podman available (if container machines), age identity accessible (if `ENC[age,...]` markers present), state dir writable, git repo clean. Color-coded pass/warn/fail output. `--json` flag for CI. 7 tests (1646→1653). | **Done** |
| FJ-252 | SSH connection multiplexing — `ControlMaster auto` + `ControlPath` + `ControlPersist=60s` for same-machine connection reuse. Reduces SSH handshake overhead from O(n) to O(1) per machine per apply. `transport/ssh.rs` manages ControlMaster lifecycle (start/stop/stop_all). Transparent fallback: mux args only injected when control socket exists. 13 tests (1653→1666). | **Done** |
| FJ-253 | Shell completions — `forjar completion bash/zsh/fish` via `clap_complete`. Generates shell-specific completion scripts from derive-based CLI. 5 tests (1666→1671). New dep: clap_complete. | **Done** |
| FJ-254 | Config includes — `includes: [base.yaml, overrides.yaml]` field in `ForjarConfig`. Merges params/machines/resources by key (later wins), policy replaced wholesale. Validation on merged config. 11 tests (1671→1682). Dogfood: dogfood-includes.yaml + dogfood-includes-machines.yaml. | **Done** |
| FJ-255 | Content diff in plan output — `forjar plan` shows file content preview for create/update actions. `--no-diff` flag suppresses. Content limited to 50 lines with `[... N more lines]` truncation. 6 tests (1682→1688). | **Done** |
| FJ-256 | `forjar lock` — generate lock file without applying. Reads config, resolves templates, computes BLAKE3 hashes for desired state, writes lock file. Useful for CI pipelines that validate state without executing. `--verify` flag checks lock matches config (exit 1 on mismatch). `--json` flag for machine-readable output. 9 tests (1688→1697). | **Done** |
| FJ-257 | Parallel apply within machines — execute independent resources concurrently on the same machine using `std::thread::scope`. Respects DAG dependencies (only parallelize resources with no inter-dependencies). `policy.parallel_resources: true` opt-in. Speedup: 2-4x for configs with many independent resources. | Done |

### Phase 12: Production Fleet DX (v1.2)

**Goal**: Harden forjar for multi-team fleet management with colored output, retry resilience, state snapshots, lifecycle hooks, and schema export. Focus on the "day 2" operations that distinguish a production tool from a prototype.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-260 | `forjar snapshot` — save/list/restore named state snapshots. `forjar snapshot save <name>` copies `state/` to `state/snapshots/<name>/`. `forjar snapshot list` shows available snapshots with timestamps. `forjar snapshot restore <name>` replaces current state. Pre-change checkpoint pattern for safe rollbacks. | **Done** — 10 tests (1703→1713) |
| FJ-261 | SSH retry with exponential backoff — transport-level retry on transient failures (connection refused, timeout, broken pipe). `policy.ssh_retries: 3` opt-in (default 1 = no retry). Backoff: 200ms × 2^attempt. Max 3 retries. Logs each retry attempt. | **Done** — 12 tests (1713→1725) |
| FJ-262 | Apply report with per-resource timing — after apply, write structured summary to `state/<machine>/last-apply.yaml` with per-resource duration, script size, exit code, hash before/after. `forjar apply --report` prints human-readable report. `--json` for CI. | **Done** — 7 tests (1725→1732) |
| FJ-263 | Colored CLI output — ANSI colors for plan (green=create, yellow=update, red=destroy, dim=noop), status, drift, doctor. No new deps (inline ANSI escape codes). Respects `NO_COLOR` env var and `--no-color` global flag. `--color=always/auto/never`. | **Done** — 8 tests (1732→1740) |
| FJ-264 | `forjar schema` — export JSON Schema for `forjar.yaml`. Generated from `ForjarConfig` struct via code, not serde. Enables IDE autocomplete (VS Code YAML extension), external validation, and documentation. No new deps. | **Done** — 5 tests (1740→1745) |
| FJ-265 | Resource lifecycle hooks — `pre_apply` and `post_apply` string fields on resources. Shell commands run on the target machine before/after the resource's main script. `pre_apply` failure skips the resource (does not apply). `post_apply` failure marks resource as failed. Use case: backup config before overwrite, restart service after config deploy. | **Done** — 7 tests (1745→1752) |
| FJ-266 | State locking — prevent concurrent applies to the same state directory. `state/.forjar.lock` PID file created on apply start, removed on completion. `--force-unlock` flag for stuck locks. Stale lock detection (PID no longer running via `/proc/<pid>`). | **Done** — (1752→1758) |
| FJ-267 | `forjar watch` — watch `forjar.yaml` for changes and auto-plan. Filesystem polling (no inotify dep) at configurable interval (`--interval N` seconds, default 2). Prints updated plan on each change. `Ctrl-C` to stop. Useful during config development. `--apply --yes` auto-applies on change (requires both flags). | **Done** — 6 tests (1758→1764) |

---

### Phase 13: Observability & Testing (v1.3)

**Goal**: Production observability, structured logging, progress indicators, dry-run improvements, and comprehensive dogfood testing. Focus on making forjar transparent, debuggable, and trustworthy in production fleet operations.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-270 | Structured event output — `forjar apply --output events` emits newline-delimited JSON events during apply (resource_started, resource_converged, resource_failed, apply_complete). Machine-parseable for CI pipelines and monitoring integration. | Done |
| FJ-271 | `forjar explain <resource>` — show the full resolution chain for a resource: raw YAML → template expansion → codegen script → transport command. Debugging tool for understanding what forjar will actually do. | Done |
| FJ-272 | Apply progress indicator — during apply, print `[3/12]` resource progress counter before each resource name. Shows position in execution order. Useful for long-running applies with many resources. | Done |
| FJ-273 | `forjar test` — run `check` scripts for all resources and report pass/fail. Like `--check` but dedicated command with summary table output. Exit code 0 = all pass, 1 = failures. CI integration: `forjar test -f forjar.yaml --json`. | Done |
| FJ-274 | Dry-run diff improvements — `forjar plan` shows unified diff of content changes (file resources). For updates, reads current file and shows line-by-line `-`/`+` diff between old and new content. | Done |
| FJ-275 | Resource dependency visualization — `forjar graph --format ascii` outputs colored ASCII tree with execution order and dependency arrows. Complements existing `mermaid` and `dot` formats. | Done |
| FJ-276 | Apply timing summary — after apply, show wall-clock time breakdown: parse+resolve, apply. `--timing` flag. | Done |
| FJ-277 | `forjar env` — show resolved environment: forjar version, OS, arch, config path, project name, machine/resource/param counts. Debugging aid for support requests. JSON output with `--json`. | Done |

### Phase 14: Resilience & Automation (v1.4)

**Goal**: Self-healing, scheduled operations, dry-run safety, and resource tagging/grouping. Focus on making forjar autonomous and reliable for unattended fleet operations.

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-280 | `forjar cron` — generate crontab/systemd timer entries for scheduled drift checks and auto-remediation. `forjar cron --interval 5m --auto-remediate` outputs installable timer config. | Planned |
| FJ-281 | Resource groups — `resource_group: network` field on resources. `forjar apply --group network` applies only that group. `forjar test --group database` tests only database resources. Orthogonal to tags. | Done |
| FJ-282 | `forjar validate --strict` — extended validation: check that paths are absolute, depends_on targets exist, no circular deps, template vars resolve. Currently only checks YAML schema. | Done |
| FJ-283 | Apply retry with backoff — `--retry N` flag retries failed resources up to N times with exponential backoff (1s, 2s, 4s, max 16s). Useful for transient network/package mirror failures. | Done |
| FJ-284 | `forjar history --since 24h` — time-based history filtering. Show only events from the last N hours/days. Supports s/m/h/d units. | Done |
| FJ-285 | `forjar plan --target resource-id` — plan a single resource and its transitive dependencies. Filters config before planning. | Done |
| FJ-286 | Apply confirmation prompt — without `--yes`, show plan summary and prompt "Apply N changes? [y/N]". Prevents accidental applies. `--yes` skips prompt (CI mode). | Done |
| FJ-287 | `forjar doctor --fix` — auto-fix common issues: stale locks, missing state dirs. Without --fix, warns about fixable issues. | Done |

### Phase 15 — v1.5 Polish & UX

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-290 | `forjar apply --parallel` — explicit flag to enable parallel wave execution. Currently controlled by `policy.parallel_resources`. CLI override for one-off parallel runs. | Done |
| FJ-291 | `forjar diff --resource` — show unified diff for a single resource between current state and desired state. Complements plan output with focused comparison. | Done |
| FJ-292 | `forjar status --json` improvements — include resource_group, tags, depends_on in JSON output via `--file` flag. | Done |
| FJ-293 | `forjar apply --dry-run --json` — JSON output for dry-run plan. Machine-readable plan output for CI integration. | Done |
| FJ-294 | `forjar graph --filter` — filter graph output to specific machines (`--machine`) or resource groups (`--group`). | Done |
| FJ-295 | `forjar validate --json` — JSON output for validation results including errors, strict mode, and summary. | Done |
| FJ-296 | `forjar history --json --since` — structured JSON history with summary counts (total_events, started, completed). | Done |
| FJ-297 | `forjar plan --output-dir` improvements — exported scripts include metadata headers (project, machine, type, group, tags, deps). | Done |

### Phase 16 — v1.6 CI & Observability

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-300 | `forjar apply --output json` — full JSON results (not just dry-run). Machine-readable apply output for CI pipelines with per-resource status, timing, and hashes. | Done |
| FJ-301 | `forjar plan --json` improvements — include resource metadata (tags, group, depends_on) in JSON plan output. Currently plan JSON is minimal. | Done |
| FJ-302 | `forjar drift --json` — structured JSON drift output. Currently text only. Include machine, resource, hash comparison, and drift status. | Done |
| FJ-303 | `forjar status --summary` — one-line summary mode for dashboards. Format: `PROJECT: N converged, N failed, N drifted`. | Done |
| FJ-304 | `forjar apply --resource-timeout` — per-resource timeout override (vs global transport timeout). Kill long-running scripts after N seconds. | Done |
| FJ-305 | `forjar check --json` — enhanced structured JSON check results with name, all_passed, total fields. Machine-readable CI gates. | Done |
| FJ-306 | `forjar env --json` — enhanced JSON with resolved_params, machine_names, resource_names. Debug tool for CI. | Done |
| FJ-307 | `forjar explain --json` — structured JSON output for resource explain. Machine-readable resource detail for tooling integration. | Done |

### Phase 17 — v1.7: Operational Hardening

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-310 | `forjar apply --rollback-on-failure` — auto-rollback to previous state on any resource failure. Restore last-known-good lock. | Done |
| FJ-311 | `forjar validate --strict` enhancements — warn on unused params, missing descriptions, duplicate tags. Lint-grade validation. | Done |
| FJ-312 | `forjar plan --cost` — show estimated change cost (resource weight × count). Prevent accidental mass-destructive applies. | Done |
| FJ-313 | `forjar apply --max-parallel N` — cap concurrent wave execution. Prevent resource exhaustion on large fleets. | Done |
| FJ-314 | `forjar status --watch` — live-updating status dashboard (re-poll every N seconds). For ops monitoring. | Done |
| FJ-315 | `forjar drift --auto-remediate` — automatically re-apply drifted resources. Daemon-friendly self-healing mode. | Done (pre-existing) |
| FJ-316 | `forjar graph --output dot` — export DAG in Graphviz DOT format for external visualization tools. | Done (pre-existing) |
| FJ-317 | `forjar apply --notify webhook` — POST JSON results to a webhook URL after apply completes. CI/CD integration. | Done |

### Phase 18 — v1.8: Fleet Management & Multi-Machine

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-320 | `forjar apply --machine <name>` — target single machine in multi-machine config. Skip others. | Done (pre-existing) |
| FJ-321 | `forjar status --machine <name>` — per-machine status filtering. Show only one machine's resources. | Done (pre-existing) |
| FJ-322 | `forjar drift --machine <name>` — per-machine drift check. Skip SSH to unrelated hosts. | Done (pre-existing) |
| FJ-323 | `forjar plan --machine <name>` — per-machine plan output. Show only changes for one machine. | Done (pre-existing) |
| FJ-324 | `forjar rolling` — rolling deployment: apply N machines at a time, stop on failure. Zero-downtime fleet updates. | Done |
| FJ-325 | `forjar canary` — canary deployment: apply to one machine first, pause for confirmation, then apply to rest. | Done |
| FJ-326 | `forjar inventory` — list all machines with connection status (reachable/unreachable). Fleet health overview. | Done |
| FJ-327 | `forjar retry-failed` — re-run only previously failed resources. Resume from partial apply without re-running converged resources. | Done |

### Phase 19 — v1.9: Advanced Templating & Config Composition

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-330 | `forjar validate --dry-expand` — show fully expanded config after template resolution without applying. Debug template issues. | Done |
| FJ-331 | `forjar apply --subset <glob>` — apply only resources matching a glob pattern (e.g., `web-*`). Fine-grained targeting. | Done |
| FJ-332 | `forjar lint --fix` — auto-fix common lint issues (normalize quotes, sort keys, fix indentation). | Done |
| FJ-333 | `forjar plan --what-if KEY=VALUE` — show plan with hypothetical param override without modifying config. | Done |
| FJ-334 | `forjar diff --format json` — structured JSON diff between state snapshots for programmatic consumption. | Done (pre-existing) |
| FJ-335 | `forjar apply --confirm-destructive` — require explicit confirmation for destroy/remove actions. Safety gate for production. | Done |
| FJ-336 | `forjar status --stale N` — show resources not updated in N days. Find abandoned infrastructure. | Done |
| FJ-337 | `forjar apply --tag <tag>` — apply only resources with a specific tag. Targeted deployment by tag. | Done (pre-existing) |

### Phase 20 — v2.0: Production Readiness

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-340 | `forjar apply --schedule <cron>` — schedule apply to run at a specific time via cron expression. Deferred execution. | Deferred |
| FJ-341 | `forjar audit` — show full audit trail: who applied what, when, from which config revision. Compliance reporting. | Done |
| FJ-342 | `forjar apply --backup` — snapshot state before apply. Auto-create named snapshot for rollback. | Done |
| FJ-343 | `forjar doctor --network` — test SSH connectivity to all machines, report latency and key issues. Network diagnostics. | Done |
| FJ-344 | `forjar plan --compact` — one-line-per-resource plan output for large configs. Dashboard-friendly format. | Done |
| FJ-345 | `forjar apply --exclude <glob>` — exclude resources matching pattern from apply. Inverse of --subset. | Done |
| FJ-346 | `forjar status --health` — aggregate health score (0-100) based on convergence rate, drift, and failure history. | Done |
| FJ-347 | `forjar apply --sequential` — force sequential execution (no parallel waves). Debug mode for ordering issues. | Done |

### Phase 21 — v2.1: Advanced Observability & Compliance

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-350 | `forjar apply --diff-only` — show what would change without generating scripts. Faster than dry-run for large configs. | Done |
| FJ-351 | `forjar compliance` — validate infrastructure against policy rules (required tags, naming conventions, security baselines). | Done |
| FJ-352 | `forjar export --format <fmt>` — export state to external formats (Terraform state, Ansible inventory, CSV). Interop bridge. | Done |
| FJ-353 | `forjar apply --notify-slack <webhook>` — post apply results to Slack channel via webhook URL. Team visibility. | Done |
| FJ-354 | `forjar graph --affected <resource>` — show transitive dependents of a resource. Impact analysis before changes. | Done |
| FJ-355 | `forjar status --drift-details` — show detailed drift report with field-level diffs for each drifted resource. | Done |
| FJ-356 | `forjar apply --cost-limit <n>` — abort apply if estimated cost (resource count) exceeds limit. Safety guardrail. | Done |
| FJ-357 | `forjar history --resource <name>` — show change history for a specific resource across all applies. Resource timeline. | Done |

### Phase 22 — v2.2: Infrastructure Intelligence

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-360 | `forjar apply --preview` — show generated shell scripts before execution. Audit what will run on machines. | Done |
| FJ-361 | `forjar suggest` — analyze config and suggest improvements (missing depends_on, unused params, security hardening). | Done |
| FJ-362 | `forjar apply --tag-filter <expr>` — boolean tag filter expressions (e.g., "web AND NOT staging"). Advanced targeting. | Done |
| FJ-363 | `forjar compare <file1> <file2>` — compare two config files and show differences. Migration planning tool. | Done |
| FJ-364 | `forjar status --timeline` — show resource convergence timeline with timestamps. Visual history per machine. | Done |
| FJ-365 | `forjar apply --dry-run --output-scripts <dir>` — write generated scripts to directory for manual review. | Done |
| FJ-366 | `forjar lock prune` — remove lock entries for resources no longer in config. State hygiene. | Done |
| FJ-367 | `forjar env diff <env1> <env2>` — compare environments (workspaces). Cross-environment drift detection. | Done |

### Phase 23 — v2.3: Developer Experience

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-370 | `forjar apply --resume` — resume from last failed resource instead of re-running everything. Checkpoint-based recovery. | Done |
| FJ-371 | `forjar template <recipe> --vars KEY=VAL` — expand a recipe template to stdout without applying. Preview recipe output. | Done |
| FJ-372 | `forjar status --changes-since <commit>` — show resources changed since a git commit. Git-aware state diffing. | Done |
| FJ-373 | `forjar apply --confirm` — interactive per-resource confirmation before execution. Manual approval mode. | Done |
| FJ-374 | `forjar lint --rules <file>` — custom lint rules from YAML file. Organization-specific policy enforcement. | Done |
| FJ-375 | `forjar graph --critical-path` — highlight the longest dependency chain. Bottleneck identification. | Done |
| FJ-376 | `forjar status --summary-by machine|type|status` — group status output by dimension. Dashboard aggregation. | Done |
| FJ-377 | `forjar apply --max-failures <n>` — allow N failures before stopping (override jidoka for partial deploys). | Done |

### Phase 24 — v2.4: Enterprise & Scale

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-380 | `forjar apply --rate-limit <n>` — limit concurrent SSH connections. Prevent SSH throttling on large fleets. | Done |
| FJ-381 | `forjar validate --schema-version <v>` — validate config against a specific schema version. Forward compatibility. | Done |
| FJ-382 | `forjar status --prometheus` — expose metrics in Prometheus exposition format. Monitoring integration. | Done |
| FJ-383 | `forjar apply --label KEY=VALUE` — add metadata labels to apply run for audit trail filtering. | Done |
| FJ-384 | `forjar lock info` — show lock file metadata (schema version, generator, machines, resource count). | Done |
| FJ-385 | `forjar graph --reverse` — show reverse dependency graph (what depends on what). Reverse impact analysis. | Done |
| FJ-386 | `forjar apply --plan-file <path>` — execute a previously saved plan file. Separation of plan and apply. | Done |
| FJ-387 | `forjar status --expired <duration>` — show resources whose lock entry is older than duration. Staleness detection. | Done |

### Phase 25 — v2.5: Operational Maturity

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-390 | `forjar apply --dry-run --json` — structured dry-run output for CI pipelines. Machine-readable plan. | Done |
| FJ-391 | `forjar validate --exhaustive` — validate all cross-references, machine existence, and param usage. Deep validation. | Done |
| FJ-392 | `forjar status --count` — simple resource count by status (converged/failed/drifted). Quick dashboard metric. | Done |
| FJ-393 | `forjar apply --notify-email <addr>` — send apply results via email (requires sendmail/SMTP). | Done |
| FJ-394 | `forjar graph --depth <n>` — limit graph traversal depth. Focused dependency visualization. | Done |
| FJ-395 | `forjar lock compact` — compact lock file by removing historical entries. Reduce state file size. | Done |
| FJ-396 | `forjar apply --skip <resource>` — skip specific resource during apply. Temporary exclusion. | Done |
| FJ-397 | `forjar status --format table|json|csv` — configurable status output format. Report generation. | Done |

### Phase 26 — v2.6: Advanced Automation & Governance

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-400 | `forjar apply --schedule <cron>` — schedule apply for future execution via at/cron. Deferred apply. | Deferred |
| FJ-401 | `forjar validate --policy-file <path>` — validate against external policy rules (OPA-style YAML). | Done |
| FJ-402 | `forjar status --anomalies` — detect anomalous resource states from historical patterns. | Done |
| FJ-403 | `forjar apply --snapshot-before` — named snapshot before apply (alias for --backup with custom name). | Done |
| FJ-404 | `forjar graph --cluster` — group resources by machine in graph output. Clustered visualization. | Done |
| FJ-405 | `forjar lock verify` — verify lock file integrity (BLAKE3 checksums). Corruption detection. | Done |
| FJ-406 | `forjar apply --concurrency <n>` — explicit concurrency limit across all machines. Global throttle. | Done |
| FJ-407 | `forjar status --diff-from <snapshot>` — diff current state against a named snapshot. Historical comparison. | Done |

### Phase 27 — v2.7: Platform Integration & Extensibility

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-410 | `forjar apply --webhook-before <url>` — POST to webhook before apply starts. Pre-flight notification. | Done |
| FJ-411 | `forjar validate --check-connectivity` — test SSH connectivity to all machines during validation. | Done |
| FJ-412 | `forjar status --resources-by-type` — group status output by resource type. Type-centric view. | Done |
| FJ-413 | `forjar apply --rollback-snapshot <name>` — auto-rollback to named snapshot on failure. | Done |
| FJ-414 | `forjar graph --orphans` — show resources with no dependencies and no dependents. Cleanup targets. | Done |
| FJ-415 | `forjar lock export --format json` — export lock file in JSON format. Interop with external tools. | Done |
| FJ-416 | `forjar apply --dry-run --diff` — combined dry-run with content diff output. CI review mode. | Done |
| FJ-417 | `forjar status --machines-only` — show only machine-level summary (no resource details). | Done |

### Phase 28 — v2.8: Resilience & Diagnostics

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-420 | `forjar apply --retry-delay <seconds>` — delay between retry attempts. Backoff tuning. | Done |
| FJ-421 | `forjar validate --check-templates` — verify all template variables resolve. Template completeness. | Done |
| FJ-422 | `forjar status --stale-resources` — show resources not updated in any recent apply. Dead resource detection. | Done |
| FJ-423 | `forjar apply --tags <tag1,tag2>` — apply only resources matching any of the tags. Multi-tag filter. | Done |
| FJ-424 | `forjar graph --stats` — show graph statistics (nodes, edges, depth, width). DAG metrics. | Done |
| FJ-425 | `forjar lock gc` — garbage collect orphaned lock entries with no matching config. State cleanup. | Done |
| FJ-426 | `forjar apply --log-file <path>` — write detailed apply log to file. Audit logging. | Done |
| FJ-427 | `forjar status --health-threshold <n>` — set custom health score threshold (default: 80). Alerting tuning. | Done |

### Phase 29 — v2.9: Workflow & Collaboration

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-430 | `forjar apply --comment <text>` — attach a comment to the apply run in event log. Audit notes. | Done |
| FJ-431 | `forjar validate --strict-deps` — verify dependency ordering matches resource declaration order. | Done |
| FJ-432 | `forjar status --json-lines` — output status as newline-delimited JSON (NDJSON). Stream processing. | Done |
| FJ-433 | `forjar apply --only-changed` — apply only resources whose config hash changed since last apply. | Done |
| FJ-434 | `forjar graph --json` — output graph as JSON adjacency list. Programmatic graph analysis. | Done |
| FJ-435 | `forjar lock diff <a> <b>` — compare two lock files and show resource-level differences. | Done |
| FJ-436 | `forjar apply --pre-script <path>` — run a script before apply starts. Custom pre-flight checks. | Done |
| FJ-437 | `forjar status --since <duration>` — show only resources changed within duration (e.g., 1h, 7d). | Done |

### Phase 30 — v3.0: Production Hardening & Observability

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-440 | `forjar apply --dry-run-json` — output dry-run results as structured JSON. CI pipeline integration. | Done |
| FJ-441 | `forjar validate --check-secrets` — scan config for hardcoded secrets or credentials. Security lint. | Done |
| FJ-442 | `forjar status --export <path>` — export status report to file (JSON/CSV/YAML). Reporting. | Done |
| FJ-443 | `forjar apply --notify-webhook <url>` — POST structured results to any webhook. Generic notification. | Done |
| FJ-444 | `forjar graph --highlight <resource>` — highlight a resource and its transitive deps in graph output. | Done |
| FJ-445 | `forjar lock merge <a> <b>` — merge two lock files (multi-team workflow). State reconciliation. | Done |
| FJ-446 | `forjar apply --post-script <path>` — run a script after apply completes. Custom post-flight checks. | Done |
| FJ-447 | `forjar status --format prometheus` — native Prometheus metrics endpoint output. Monitoring integration. | Done |

### Phase 31 — v3.1: Enterprise & Scale

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-450 | `forjar apply --approval-required` — require explicit approval before destructive changes. Gate. | Done |
| FJ-451 | `forjar validate --check-idempotency` — verify all resources produce idempotent scripts. Safety lint. | Done |
| FJ-452 | `forjar status --compact` — minimal one-line-per-machine output for large fleets. Fleet view. | Done |
| FJ-453 | `forjar apply --canary-percent <n>` — apply to N% of machines first, then rest. Gradual rollout. | Done |
| FJ-454 | `forjar graph --prune <resource>` — show graph with a resource and its subtree removed. Impact analysis. | Done |
| FJ-455 | `forjar lock rebase <from> <to>` — rebase lock file from one config version to another. Migration. | Done |
| FJ-456 | `forjar apply --schedule <cron>` — schedule apply for later execution. Deferred apply. | Done |
| FJ-457 | `forjar status --alerts` — show resources in alert state (failed, drifted, or stale). Dashboard view. | Done |

### Phase 32 — v3.2: Multi-Environment & Compliance

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-460 | `forjar apply --env <name>` — apply using named environment config overlay. Multi-env support. | Done |
| FJ-461 | `forjar validate --check-drift-coverage` — verify all resources have drift detection configured. Coverage lint. | Done |
| FJ-462 | `forjar status --diff-lock <path>` — diff current lock against a saved lock snapshot. Change detection. | Done |
| FJ-463 | `forjar apply --dry-run-diff` — show unified diff of what would change. Review-friendly output. | Done |
| FJ-464 | `forjar graph --layers` — show graph organized by dependency layers (depth levels). Layer visualization. | Done |
| FJ-465 | `forjar lock sign <key>` — cryptographically sign lock file with BLAKE3 key. Tamper detection. | Done |
| FJ-466 | `forjar apply --notify-pagerduty <key>` — send apply events to PagerDuty. Incident integration. | Done |
| FJ-467 | `forjar status --compliance <policy>` — check compliance against named policy. Governance. | Done |

### Phase 33 — v3.3: Advanced Operations

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-470 | `forjar apply --batch-size <n>` — process resources in batches of N. Memory-bounded execution. | Done |
| FJ-471 | `forjar validate --check-cycles-deep` — detect indirect circular dependencies via transitive closure. | Done |
| FJ-472 | `forjar status --histogram` — show resource status distribution as ASCII histogram. Visual overview. | Done |
| FJ-473 | `forjar apply --notify-teams <webhook>` — send apply results to Microsoft Teams. Teams integration. | Done |
| FJ-474 | `forjar graph --critical-resources` — identify resources with the most dependents. Bottleneck analysis. | Done |
| FJ-475 | `forjar lock verify-sig <key>` — verify lock file signature. Integrity verification. | Done |
| FJ-476 | `forjar apply --abort-on-drift` — abort apply if drift detected before execution. Safety gate. | Done |
| FJ-477 | `forjar status --dependency-health` — show health score weighted by dependency position. Risk analysis. | Done |

### Phase 34 — v3.4: Fleet Management & Automation

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-480 | `forjar apply --dry-run-summary` — one-line summary of what would change per machine. Fleet overview. | Done |
| FJ-481 | `forjar validate --check-naming` — enforce resource naming conventions (kebab-case, prefix rules). | Done |
| FJ-482 | `forjar status --top-failures` — show most frequently failing resources. Reliability analysis. | Done |
| FJ-483 | `forjar apply --notify-discord <webhook>` — send apply results to Discord. Discord integration. | Done |
| FJ-484 | `forjar graph --weight` — show edge weights based on resource dependency strength. Weighted graph. | Done |
| FJ-485 | `forjar lock compact-all` — compact all machine lock files in one operation. Bulk maintenance. | Done |
| FJ-486 | `forjar apply --rollback-on-threshold <n>` — auto-rollback if more than N resources fail. Blast radius. | Done |
| FJ-487 | `forjar status --convergence-rate` — show convergence percentage over time. Trend analysis. | Done |

### Phase 35 — v3.5: Observability & Audit

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-490 | `forjar apply --metrics-port <port>` — expose apply metrics on HTTP port for Prometheus scraping. | Done |
| FJ-491 | `forjar validate --check-overlaps` — detect resources targeting the same path/port/name on same machine. | Done |
| FJ-492 | `forjar status --drift-summary` — one-line per-machine drift count and percentage. Fleet drift view. | Done |
| FJ-493 | `forjar apply --notify-opsgenie <key>` — send apply alerts to OpsGenie. Incident management. | Done |
| FJ-494 | `forjar graph --subgraph <resource>` — extract and display a resource's dependency subgraph. | Done |
| FJ-495 | `forjar lock audit-trail` — show full audit trail of lock file changes with timestamps. | Done |
| FJ-496 | `forjar apply --circuit-breaker <n>` — pause apply after N consecutive failures. Circuit breaker pattern. | Done |
| FJ-497 | `forjar status --resource-age` — show age of each resource since last successful apply. Staleness view. | Done |

### Phase 36 — v3.6: Policy Engine & Governance

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-500 | `forjar apply --require-approval <users>` — require named approvers before apply proceeds. Change control. | Done |
| FJ-501 | `forjar validate --check-limits` — enforce resource count limits per machine/type. Governance. | Done |
| FJ-502 | `forjar status --sla-report` — show SLA compliance based on convergence timing. SLA tracking. | Done |
| FJ-503 | `forjar apply --notify-datadog <key>` — send apply events to Datadog. APM integration. | Done |
| FJ-504 | `forjar graph --impact-radius <resource>` — show blast radius of changing a resource. Risk analysis. | Done |
| FJ-505 | `forjar lock rotate-keys` — rotate all lock file signing keys. Key management. | Done |
| FJ-506 | `forjar apply --change-window <cron>` — restrict applies to defined maintenance windows. Change control. | Done |
| FJ-507 | `forjar status --compliance-report <policy>` — generate full compliance report. Audit readiness. | Done |

### Phase 37 — v3.7: Resilience & Recovery

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-510 | `forjar apply --canary-machine <name>` — apply to single machine first, confirm before fleet. Canary deploy. | Done |
| FJ-511 | `forjar validate --check-complexity` — warn on resources with high dependency fan-out. Complexity guard. | Done |
| FJ-512 | `forjar status --mttr` — show mean time to recovery per resource. Reliability metric. | Done |
| FJ-513 | `forjar apply --notify-newrelic <key>` — send apply events to New Relic. Observability integration. | Done |
| FJ-514 | `forjar graph --dependency-matrix` — output resource dependency matrix (CSV/JSON). Analysis export. | Done |
| FJ-515 | `forjar lock backup` — create timestamped backup of all lock files. Disaster recovery. | Done |
| FJ-516 | `forjar apply --max-duration <secs>` — abort entire apply if it exceeds time limit. Timeout guard. | Done |
| FJ-517 | `forjar status --trend <n>` — show status trend over last N applies. Historical analysis. | Done |

### Phase 38 — v3.8: Intelligence & Prediction

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-520 | `forjar apply --notify-grafana <url>` — send apply annotations to Grafana. Dashboard integration. | Done |
| FJ-521 | `forjar validate --check-security` — scan for insecure permissions/ports/users. Security audit. | Done |
| FJ-522 | `forjar status --prediction` — predict next failure based on historical patterns. ML-lite. | Done |
| FJ-523 | `forjar apply --rate-limit-resources <n>` — apply at most N resources per minute. Throttle guard. | Done |
| FJ-524 | `forjar graph --hotspots` — highlight resources with most changes/failures. Heat map. | Done |
| FJ-525 | `forjar lock gc` — garbage collect orphaned lock entries. State hygiene. | Done |
| FJ-526 | `forjar apply --checkpoint-interval <secs>` — save intermediate state during long applies. Resumability. | Done |
| FJ-527 | `forjar status --capacity` — show resource utilization vs limits per machine. Capacity planning. | Done |

### Phase 39 — v3.9: Multi-Environment & Workflow

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-530 | `forjar apply --notify-victorops <key>` — send apply events to VictorOps/Splunk On-Call. Incident integration. | Done |
| FJ-531 | `forjar validate --check-deprecation` — warn on deprecated resource fields/types. Migration aid. | Done |
| FJ-532 | `forjar status --cost-estimate` — estimate resource cost based on type counts. Budget planning. | Done |
| FJ-533 | `forjar apply --blue-green <name>` — blue/green deployment with machine pairs. Zero-downtime. | Done |
| FJ-534 | `forjar graph --timeline` — show resource application order as ASCII timeline. Visualization. | Done |
| FJ-535 | `forjar lock verify-chain` — verify full chain of custody from lock signatures. Provenance. | Done |
| FJ-536 | `forjar apply --dry-run-cost` — show estimated cost without applying. Pre-flight analysis. | Done |
| FJ-537 | `forjar status --staleness-report` — show resources not applied within configurable window. Hygiene. | Done |

### Phase 40 — v4.0: Advanced Analytics & Reporting

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-540 | `forjar apply --notify-msteams-adaptive <url>` — send Adaptive Card to MS Teams. Rich notifications. | Done |
| FJ-541 | `forjar validate --check-drift-risk` — score drift risk based on resource volatility. Proactive guard. | Done |
| FJ-542 | `forjar status --health-score` — composite health score (0-100) across all machines. Dashboard metric. | Done |
| FJ-543 | `forjar apply --progressive <percent>` — progressive rollout (apply to N% of machines). Gradual deploy. | Done |
| FJ-544 | `forjar graph --what-if <resource>` — simulate removing a resource, show impact. Analysis tool. | Done |
| FJ-545 | `forjar lock stats` — show lock file statistics (sizes, ages, resource counts). State overview. | Done |
| FJ-546 | `forjar apply --approval-webhook <url>` — POST for approval before applying. GitOps gate. | Done |
| FJ-547 | `forjar status --executive-summary` — one-line per machine summary for dashboards. Executive view. | Done |

### Phase 41 — v4.1: Compliance & Governance

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-550 | `forjar apply --notify-incident <url>` — POST incident to PagerDuty/Opsgenie with full context. Incident response. | Done |
| FJ-551 | `forjar validate --check-compliance <policy>` — validate resources against compliance policy (CIS, SOC2). Governance. | Done |
| FJ-552 | `forjar status --audit-trail` — show full audit trail with who/what/when for each change. Compliance reporting. | Done |
| FJ-553 | `forjar apply --change-window <cron>` — only allow apply during maintenance windows. Change control. | Done (FJ-506) |
| FJ-554 | `forjar graph --blast-radius <resource>` — show all resources affected by a change to target. Risk assessment. | Done |
| FJ-555 | `forjar lock audit` — verify lock file integrity and show tampering evidence. Security audit. | Done |
| FJ-556 | `forjar apply --sign-off <user>` — require named sign-off before apply proceeds. Approval chain. | Done |
| FJ-557 | `forjar status --sla-report` — SLA compliance report (uptime, MTTR, change frequency). Service level. | Done (FJ-502) |

### Phase 42 — v4.2: Observability & Telemetry

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-560 | `forjar apply --notify-sns <topic-arn>` — publish apply events to AWS SNS topic. Cloud integration. | Done |
| FJ-561 | `forjar validate --check-portability` — check resources for platform-specific assumptions. Cross-platform. | Done |
| FJ-562 | `forjar status --resource-graph` — show resource dependency graph from live state. State visualization. | Done |
| FJ-563 | `forjar apply --telemetry-endpoint <url>` — POST OpenTelemetry spans for apply execution. Observability. | Done |
| FJ-564 | `forjar graph --change-impact <resource>` — show direct + indirect impact of changing a resource. Planning. | Done |
| FJ-565 | `forjar lock compress` — compress old lock files with zstd. Storage optimization. | Done |
| FJ-566 | `forjar apply --runbook <url>` — attach runbook URL to apply for audit trail. Documentation link. | Done |
| FJ-567 | `forjar status --drift-velocity` — show drift rate over time (changes per day/week). Trend analysis. | Done |

### Phase 43 — v4.3: Fleet Management & Scale

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-570 | `forjar apply --notify-pubsub <topic>` — publish apply events to Google Cloud Pub/Sub. GCP integration. | Done |
| FJ-571 | `forjar validate --check-resource-limits` — validate resource counts don't exceed per-machine limits. Scale guard. | Done |
| FJ-572 | `forjar status --fleet-overview` — aggregated fleet summary across all machines. Fleet dashboard. | Done |
| FJ-573 | `forjar apply --fleet-strategy <strategy>` — fleet-wide rollout strategy (parallel, rolling, canary). Fleet deploy. | Done |
| FJ-574 | `forjar graph --resource-types` — show graph colored/grouped by resource type. Visualization. | Done |
| FJ-575 | `forjar lock defrag` — defragment lock files (reorder resources alphabetically). Maintenance. | Done |
| FJ-576 | `forjar apply --pre-check <script>` — run validation script before apply proceeds. Gate check. | Done |
| FJ-577 | `forjar status --machine-health` — per-machine health details with resource breakdown. Diagnostics. | Done |

### Phase 44 — v4.4: Configuration Intelligence

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-580 | `forjar apply --notify-eventbridge <bus>` — publish to AWS EventBridge for event-driven workflows. AWS integration. | Done |
| FJ-581 | `forjar validate --check-unused` — detect resources not referenced by any dependency chain. Cleanup. | Done |
| FJ-582 | `forjar status --config-drift` — compare running config against declared config. Config drift. | Done |
| FJ-583 | `forjar apply --dry-run-graph` — show execution graph without applying. Plan visualization. | Done |
| FJ-584 | `forjar graph --topological-levels` — show resources grouped by topological depth level. Layering. | Done |
| FJ-585 | `forjar lock normalize` — normalize lock file format (consistent key ordering, whitespace). Standardization. | Done |
| FJ-586 | `forjar apply --post-check <script>` — run validation script after apply completes. Verification gate. | Done |
| FJ-587 | `forjar status --convergence-time` — show average time to convergence per resource. Performance insight. | Done |

### Phase 45 — v4.5: Advanced Orchestration

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-590 | `forjar apply --notify-kafka <topic>` — publish apply events to Apache Kafka. Stream integration. | Done |
| FJ-591 | `forjar validate --check-dependencies` — validate all depends_on references resolve correctly. Integrity. | Done |
| FJ-592 | `forjar status --resource-timeline` — show per-resource status changes over time. History view. | Done |
| FJ-593 | `forjar apply --max-retries <n>` — retry failed resources up to N times before giving up. Resilience. | Done |
| FJ-594 | `forjar graph --execution-order` — show exact execution order with timing estimates. Planning. | Done |
| FJ-595 | `forjar lock validate` — validate lock file schema and cross-references. Integrity check. | Done |
| FJ-596 | `forjar apply --rollback-window <duration>` — auto-rollback if issues detected within window. Safety. | Done |
| FJ-597 | `forjar status --error-summary` — aggregated error summary across all machines. Debugging. | Done |

### Phase 46 — v4.6: Security Hardening & Audit

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-600 | `forjar apply --notify-azure-servicebus <conn>` — publish events to Azure Service Bus. Cloud integration. | Done |
| FJ-601 | `forjar validate --check-permissions` — validate resource ownership/mode fields are secure. Security. | Done |
| FJ-602 | `forjar status --security-posture` — show security-relevant resource states (modes, ownership). Audit. | Done |
| FJ-603 | `forjar apply --approval-timeout <duration>` — timeout for interactive approval prompts. Safety. | Done |
| FJ-604 | `forjar graph --security-boundaries` — highlight resources crossing security boundaries. Visualization. | Done |
| FJ-605 | `forjar lock verify-hmac` — verify lock file BLAKE3-based HMAC signatures. Integrity. | Done |
| FJ-606 | `forjar apply --pre-flight <script>` — run pre-flight validation script before apply. Safety gate. | Done |
| FJ-607 | `forjar status --compliance-report` — generate compliance report from resource states. Audit. | Done |

### Phase 47 — v4.7: Resource Intelligence & Analytics

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-610 | `forjar apply --notify-gcp-pubsub-v2 <topic>` — enhanced GCP Pub/Sub with ordering keys. Cloud integration. | Done |
| FJ-611 | `forjar validate --check-idempotency-deep` — deep idempotency analysis with simulation. Quality. | Done |
| FJ-612 | `forjar status --resource-cost` — estimate resource cost based on type and count. Planning. | Done |
| FJ-613 | `forjar apply --checkpoint <name>` — create named checkpoint before apply for rollback. Safety. | Done |
| FJ-614 | `forjar graph --resource-age` — show resource age based on last apply timestamp. Analytics. | Done |
| FJ-615 | `forjar lock archive` — archive old lock files to compressed storage. Housekeeping. | Done |
| FJ-616 | `forjar apply --post-flight <script>` — run post-flight validation script after apply. Verification. | Done |
| FJ-617 | `forjar status --drift-forecast` — predict likely drift based on historical patterns. Intelligence. | Done |

### Phase 48 — v4.8: Workflow Automation & Pipelines

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-620 | `forjar apply --notify-rabbitmq <queue>` — publish events to RabbitMQ. Message queue integration. | Done |
| FJ-621 | `forjar validate --check-machine-reachability` — verify machines are reachable before apply. Preflight. | Done |
| FJ-622 | `forjar status --pipeline-status` — show CI/CD pipeline integration status. DevOps. | Done |
| FJ-623 | `forjar apply --gate <name>` — require named approval gate before apply proceeds. Safety. | Done |
| FJ-624 | `forjar graph --parallel-groups` — show which resources can execute in parallel. Optimization. | Done |
| FJ-625 | `forjar lock snapshot` — create point-in-time lock file snapshot with metadata. Recovery. | Done |
| FJ-626 | `forjar apply --schedule <cron>` — schedule apply for future execution. Automation. | Done |
| FJ-627 | `forjar status --resource-dependencies` — show runtime dependency graph from lock files. Analytics. | Done |

### Phase 49 — v4.9: Advanced Diagnostics & Debugging

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-630 | `forjar apply --notify-nats <subject>` — publish events to NATS messaging. Lightweight integration. | Done |
| FJ-631 | `forjar validate --check-circular-refs` — detect circular template/param references. Integrity. | Done |
| FJ-632 | `forjar status --diagnostic` — comprehensive diagnostic report with recommendations. Debugging. | Done |
| FJ-633 | `forjar apply --dry-run-verbose` — verbose dry-run showing all planned commands. Debugging. | Done |
| FJ-634 | `forjar graph --critical-chain` — show longest dependency chain (critical path analysis). Planning. | Done |
| FJ-635 | `forjar lock repair` — attempt automatic repair of corrupted lock files. Recovery. | Done |
| FJ-636 | `forjar apply --explain` — explain what each step will do before executing. Education. | Done |
| FJ-637 | `forjar status --stale-resources` — identify resources not updated in configurable threshold. Hygiene. | Done |

### Phase 50 — v5.0: Production Readiness & Polish

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-640 | `forjar apply --notify-mqtt <topic>` — publish events to MQTT broker. IoT integration. | Done |
| FJ-641 | `forjar validate --check-naming-conventions` — enforce naming conventions across resources. Style. | Done |
| FJ-642 | `forjar status --uptime` — show resource uptime based on convergence history. Reliability. | Done |
| FJ-643 | `forjar apply --confirmation-message <msg>` — custom confirmation message before apply. UX. | Done |
| FJ-644 | `forjar graph --dependency-depth` — show max dependency depth per resource. Complexity. | Done |
| FJ-645 | `forjar lock history` — show lock file change history with diffs. Audit trail. | Done |
| FJ-646 | `forjar apply --summary-only` — only show summary, no per-resource output. Brevity. | Done |
| FJ-647 | `forjar status --recommendations` — AI-powered recommendations based on state analysis. Intelligence. | Done |

### Phase 51 — Environment & Configuration Management (FJ-650→FJ-657)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-650 | `forjar apply --notify-redis <channel>` — publish events to Redis pub/sub. Caching integration. | Done |
| FJ-651 | `forjar validate --check-resource-limits` — warn on excessive resource counts per machine. Governance. | Done |
| FJ-652 | `forjar status --convergence-rate` — show percentage of resources converged over time. Trending. | Done |
| FJ-653 | `forjar apply --rollback-on-failure` — auto-rollback to last known good state on failure. Safety. | Done |
| FJ-654 | `forjar graph --orphan-detection` — find resources with no dependents or dependencies. Cleanup. | Done |
| FJ-655 | `forjar lock gc` — garbage collect stale entries from lock files. Maintenance. | Done |
| FJ-656 | `forjar apply --max-parallel <n>` — limit concurrent resource operations. Throttling. | Done |
| FJ-657 | `forjar status --machine-summary` — per-machine resource count and health summary. Fleet. | Done |

### Phase 52 — Compliance & Policy Automation (FJ-660→FJ-667)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-660 | `forjar apply --notify-amqp <exchange>` — publish events to AMQP exchange. Enterprise messaging. | Done |
| FJ-661 | `forjar validate --check-owner-consistency` — ensure all resources have consistent ownership. Governance. | Done |
| FJ-662 | `forjar status --change-frequency` — show how often each resource changes. Stability metrics. | Done |
| FJ-663 | `forjar apply --pre-apply-hook <cmd>` — run arbitrary command before each resource apply. Extensibility. | Done |
| FJ-664 | `forjar graph --cross-machine-deps` — visualize dependencies across machines. Fleet topology. | Done |
| FJ-665 | `forjar lock merge` — merge two lock files for split-brain recovery. Consistency. | Done |
| FJ-666 | `forjar apply --resource-filter <glob>` — only apply resources matching glob pattern. Selective. | Done |
| FJ-667 | `forjar status --lock-age` — show age of each lock file entry. Freshness. | Done |

### Phase 53 — Multi-Environment & Secrets Management (FJ-670→FJ-677)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-670 | `forjar apply --notify-stomp <destination>` — publish events to STOMP protocol. Legacy integration. | Done |
| FJ-671 | `forjar validate --check-path-conflicts` — detect overlapping file paths across resources. Safety. | Done |
| FJ-672 | `forjar status --failed-since <timestamp>` — show resources failed since a given time. Incident response. | Done |
| FJ-673 | `forjar apply --post-apply-hook <cmd>` — run command after each resource apply. Extensibility. | Done |
| FJ-674 | `forjar graph --machine-groups` — group resources by machine in graph output. Clarity. | Done |
| FJ-675 | `forjar lock prune` — remove entries for resources no longer in config. Cleanup. | Done |
| FJ-676 | `forjar apply --dry-run-shell` — output shell scripts instead of executing. Debugging. | Done |
| FJ-677 | `forjar status --hash-verify` — verify BLAKE3 hashes in lock match computed hashes. Integrity. | Done |

### Phase 54 — Observability & Telemetry Deep Dive (FJ-680→FJ-687)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-680 | `forjar apply --notify-zeromq <endpoint>` — publish events to ZeroMQ socket. High-perf messaging. | Done |
| FJ-681 | `forjar validate --check-service-deps` — validate service dependency chains are satisfiable. Safety. | Done |
| FJ-682 | `forjar status --resource-size` — show estimated resource sizes (file bytes, package count). Capacity. | Done |
| FJ-683 | `forjar apply --canary-resource <name>` — apply single resource first as canary. Safety. | Done |
| FJ-684 | `forjar graph --resource-clusters` — identify tightly-coupled resource clusters. Architecture. | Done |
| FJ-685 | `forjar lock rehash` — recompute all lock file hashes from current state. Migration. | Done |
| FJ-686 | `forjar apply --timeout-per-resource <secs>` — per-resource timeout override. Reliability. | Done |
| FJ-687 | `forjar status --drift-details-all` — show drift details for all machines at once. Fleet. | Done |

### Phase 55 — Advanced Workflow & Pipeline Integration (FJ-690→FJ-697)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-690 | `forjar apply --notify-grpc <endpoint>` — publish events to gRPC endpoint. High-perf messaging. | Done |
| FJ-691 | `forjar validate --check-template-vars` — validate all template variables are defined. Safety. | Done |
| FJ-692 | `forjar status --last-apply-duration` — show duration of last apply per resource. Performance. | Done |
| FJ-693 | `forjar apply --skip-unchanged` — skip resources whose hash hasn't changed. Efficiency. | Done |
| FJ-694 | `forjar graph --fan-out` — show resource fan-out metrics (dependents count). Architecture. | Done |
| FJ-695 | `forjar lock restore` — restore lock state from named snapshot. Versioning. | Done |
| FJ-696 | `forjar apply --retry-backoff <factor>` — exponential backoff factor for retries. Reliability. | Done |
| FJ-697 | `forjar status --config-hash` — show hash of current config for change detection. Integrity. | Done |

### Phase 56 — Security Hardening & Audit Trail (FJ-700→FJ-707)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-700 | `forjar apply --notify-sqs <queue-url>` — publish events to AWS SQS queue. Cloud messaging. | Done |
| FJ-701 | `forjar validate --check-mode-consistency` — validate file mode consistency across resources. Safety. | Done |
| FJ-702 | `forjar status --stale-resources` — show resources not applied recently. Freshness. | Done |
| FJ-703 | `forjar apply --plan-output-file <path>` — save plan output to file. Preview. | Done |
| FJ-704 | `forjar graph --leaf-resources` — identify leaf resources with no dependents. Architecture. | Done |
| FJ-705 | `forjar lock verify-schema` — validate lock file against expected schema version. Migration. | Done |
| FJ-706 | `forjar apply --resource-priority <name>=<n>` — set execution priority for specific resources. Scheduling. | Done |
| FJ-707 | `forjar status --convergence-history` — show convergence trend over time. Observability. | Done |

### Phase 57 — Fleet Management & Multi-Machine Orchestration (FJ-710→FJ-717)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-710 | `forjar apply --notify-eventbridge <bus>` — publish events to AWS EventBridge. Cloud messaging. | Done |
| FJ-711 | `forjar validate --check-group-consistency` — validate user/group consistency across resources. Safety. | Done |
| FJ-712 | `forjar status --resource-inputs` — show resource input fields per resource. Insight. | Done |
| FJ-713 | `forjar apply --apply-window <secs>` — time window for apply operations. Throttling. | Done |
| FJ-714 | `forjar graph --reverse-deps` — show reverse dependency graph (who depends on me). Architecture. | Done |
| FJ-715 | `forjar lock tag <name> <value>` — add metadata tags to lock files. Organization. | Done |
| FJ-716 | `forjar apply --fail-fast-machine` — stop all machines on first machine failure. Safety. | Done |
| FJ-717 | `forjar status --drift-trend` — show drift trend over time. Fleet. | Done |

### Phase 58 — Configuration Validation & Schema Evolution (FJ-720→FJ-727)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-720 | `forjar apply --notify-mattermost <webhook>` — publish events to Mattermost webhook. Messaging. | Done |
| FJ-721 | `forjar validate --check-mount-points` — validate mount point paths don't conflict. Safety. | Done |
| FJ-722 | `forjar status --failed-resources` — show only failed resources across machines. Debugging. | Done |
| FJ-723 | `forjar apply --cooldown <secs>` — wait between resource applies. Rate limiting. | Done |
| FJ-724 | `forjar graph --depth-first` — show depth-first traversal order. Analysis. | Done |
| FJ-725 | `forjar lock migrate <from-version>` — migrate lock file schema between versions. Migration. | Done |
| FJ-726 | `forjar apply --exclude-machine <name>` — exclude specific machine from apply. Targeting. | Done |
| FJ-727 | `forjar status --resource-types-summary` — show count per resource type. Overview. | Done |

### Phase 59 — Resource Lifecycle & Dependency Analysis (FJ-730→FJ-737)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-730 | `forjar apply --notify-ntfy <topic>` — publish events to ntfy.sh notification service. Push notifications. | Done |
| FJ-731 | `forjar validate --check-cron-syntax` — validate cron schedule expressions. Safety. | Done |
| FJ-732 | `forjar status --resource-health` — show health status per resource (converged/failed/drifted). Health. | Done |
| FJ-733 | `forjar apply --dry-run-json` — output dry-run results as JSON. Automation. | Done |
| FJ-734 | `forjar graph --breadth-first` — show breadth-first traversal order. Analysis. | Done |
| FJ-735 | `forjar lock compact-all` — compact all lock files removing redundant entries. Maintenance. | Done |
| FJ-736 | `forjar apply --only-machine <name>` — apply only to specific machine. Targeting. | Done |
| FJ-737 | `forjar status --machine-health` — show overall health per machine. Fleet. | Done |

### Phase 60 — Observability & Operational Insights (FJ-741→FJ-748)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-741 | `forjar validate --check-env-refs` — verify all `{{env.*}}` template references have matching env vars. Safety. | Done |
| FJ-742 | `forjar status --dependency-count` — show inbound/outbound dependency count per resource. Insight. | Done |
| FJ-743 | `forjar graph --subgraph-stats` — show node/edge/depth stats for each connected component. Analysis. | Done |
| FJ-744 | `forjar apply --notify-webhook-headers <json>` — custom headers for webhook notifications. Integration. | Done |
| FJ-745 | `forjar validate --check-resource-names` — enforce resource naming regex pattern. Governance. | Done |
| FJ-746 | `forjar status --last-apply-status` — show last apply success/failure per machine. Fleet. | Done |
| FJ-747 | `forjar graph --dependency-count` — show in-degree and out-degree per resource. Metrics. | Done |
| FJ-748 | `forjar status --resource-staleness` — show time since last successful apply per resource. Freshness. | Done |

### Phase 61 — Governance & Compliance Automation (FJ-749→FJ-756)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-749 | `forjar validate --check-resource-count` — warn if resource count exceeds threshold per machine. Governance. | Done |
| FJ-750 | `forjar status --convergence-percentage` — show % of resources converged per machine. Dashboard. | Done |
| FJ-751 | `forjar graph --root-resources` — show resources with no dependencies (roots of the DAG). Analysis. | Done |
| FJ-752 | `forjar apply --notify-log <path>` — append structured JSON events to a local file. Audit. | Done |
| FJ-753 | `forjar validate --check-duplicate-paths` — detect duplicate file/path across resources on same machine. Safety. | Done |
| FJ-754 | `forjar status --failed-count` — show count of failed resources per machine. Dashboard. | Done |
| FJ-755 | `forjar graph --edge-list` — output graph as simple edge list (source→target pairs). Export. | Done |
| FJ-756 | `forjar status --drift-count` — show count of drifted resources per machine. Dashboard. | Done |

### Phase 62 — Advanced Observability & Diagnostics (FJ-757→FJ-764)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-757 | `forjar validate --check-circular-deps` — detect circular dependency chains. Safety. | Done |
| FJ-758 | `forjar status --resource-age` — show time since each resource was last applied. Observability. | Done |
| FJ-759 | `forjar graph --connected-components` — show disconnected subgraphs. Analysis. | Done |
| FJ-760 | `forjar apply --notify-exec <cmd>` — run arbitrary command as notification handler. Extensibility. | Done |
| FJ-761 | `forjar validate --check-machine-refs` — verify all machine references in resources exist. Safety. | Done |
| FJ-762 | `forjar status --resource-duration` — show last apply duration per resource. Performance. | Done |
| FJ-763 | `forjar graph --adjacency-matrix` — output graph as adjacency matrix. Export. | Done |
| FJ-764 | `forjar status --machine-resource-map` — show which resources target each machine. Dashboard. | Done |

### Phase 63 — Policy Enforcement & Fleet Intelligence (FJ-765→FJ-772)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-765 | `forjar validate --check-provider-consistency` — verify all package resources use consistent providers per machine. Governance. | Done |
| FJ-766 | `forjar status --fleet-convergence` — aggregate convergence across all machines. Fleet. | Done |
| FJ-767 | `forjar graph --longest-path` — show longest dependency chain length. Analysis. | Done |
| FJ-768 | `forjar apply --notify-file <path>` — write one-line status to a file (for monitoring). Ops. | Done |
| FJ-769 | `forjar validate --check-state-values` — verify state field values are valid for each resource type. Safety. | Done |
| FJ-770 | `forjar status --resource-hash` — show BLAKE3 hash per resource from lock file. Debug. | Done |
| FJ-771 | `forjar graph --in-degree` — show in-degree (number of dependents) per resource. Analysis. | Done |
| FJ-772 | `forjar status --machine-drift-summary` — show drift percentage per machine. Fleet. | Done |

### Phase 64 — Dependency Intelligence & Audit Trail (FJ-773→FJ-780)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-773 | `forjar validate --check-unused-machines` — detect machines defined but not referenced by any resource. Cleanup. | Done |
| FJ-774 | `forjar status --apply-history-count` — show total apply count per machine from event log. Audit. | Done |
| FJ-775 | `forjar graph --out-degree` — show out-degree (number of dependencies) per resource. Analysis. | Done |
| FJ-776 | `forjar apply --notify-json` — print structured JSON notification to stdout. Composability. | Done |
| FJ-777 | `forjar validate --check-tag-consistency` — verify resource tags follow naming conventions. Governance. | Done |
| FJ-778 | `forjar status --lock-file-count` — show number of lock files per machine. Ops. | Done |
| FJ-779 | `forjar graph --density` — show graph density (edges / max-possible-edges). Analysis. | Done |
| FJ-780 | `forjar status --resource-type-distribution` — show resource type breakdown across fleet. Dashboard. | Done |

### Phase 65 — Operational Readiness & Deep Analysis (FJ-781→FJ-788)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-781 | `forjar validate --check-dependency-exists` — verify all depends_on targets reference existing resources. Safety. | Done |
| FJ-782 | `forjar status --resource-apply-age` — show time since last apply per resource. Staleness. | Done |
| FJ-783 | `forjar graph --topological-sort` — output resources in valid execution order. Planning. | Done |
| FJ-784 | `forjar apply --notify-slack-webhook` — send apply results to Slack webhook. Integration. | Done |
| FJ-785 | `forjar validate --check-path-conflicts-strict` — detect resources targeting the same file path. Safety. | Done |
| FJ-786 | `forjar status --machine-uptime` — show time since first apply per machine. Fleet. | Done |
| FJ-787 | `forjar graph --critical-path-resources` — show resources on the longest dependency chain. Planning. | Done |
| FJ-788 | `forjar status --resource-churn` — show apply frequency per resource over time. Ops. | Done |

### Phase 66 — Fleet Intelligence & Compliance (FJ-789→FJ-796)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-789 | `forjar validate --check-duplicate-names` — detect duplicate resource names across groups. Safety. | Done |
| FJ-790 | `forjar status --last-drift-time` — show timestamp of last drift detection per resource. Monitoring. | Done |
| FJ-791 | `forjar graph --sink-resources` — show resources that nothing depends on (leaf nodes). Analysis. | Done |
| FJ-792 | `forjar apply --notify-telegram` — send apply results to Telegram bot. Integration. | Done |
| FJ-793 | `forjar validate --check-resource-groups` — verify resource groups are non-empty. Governance. | Done |
| FJ-794 | `forjar status --machine-resource-count` — show resource count per machine. Fleet. | Done |
| FJ-795 | `forjar graph --bipartite-check` — check if dependency graph is bipartite. Analysis. | Done |
| FJ-796 | `forjar status --convergence-score` — weighted convergence score across fleet. Dashboard. | Done |

### Phase 67 — Advanced Graph Analysis & Monitoring (FJ-797→FJ-804)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-797 | `forjar validate --check-orphan-resources` — detect resources not reachable from any root. Safety. | Done |
| FJ-798 | `forjar status --apply-success-rate` — show success/failure ratio per machine. Monitoring. | Done |
| FJ-799 | `forjar graph --strongly-connected` — find strongly connected components (Tarjan's). Analysis. | Done |
| FJ-800 | `forjar apply --notify-webhook-v2` — enhanced webhook with retry and custom headers. Integration. | Done |
| FJ-801 | `forjar validate --check-machine-arch` — verify resource compatibility with machine architecture. Safety. | Done |
| FJ-802 | `forjar status --error-rate` — show error rate per resource type. Monitoring. | Done |
| FJ-803 | `forjar graph --dependency-matrix-csv` — export dependency matrix as CSV. Export. | Done |
| FJ-804 | `forjar status --fleet-health-summary` — one-line per machine with health + convergence. Dashboard. | Done |

### Phase 68 — Fleet Intelligence & Advanced Validation (FJ-805→FJ-812)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-805 | `forjar validate --check-resource-health-conflicts` — detect resources with conflicting health indicators. Safety. | Done |
| FJ-806 | `forjar status --machine-convergence-history` — convergence trend per machine over time. Dashboard. | Done |
| FJ-807 | `forjar graph --resource-weight` — assign weights to edges by dependency criticality. Analysis. | Done |
| FJ-808 | `forjar apply --notify-pagerduty` — PagerDuty Events v2 integration for apply notifications. Integration. | Done |
| FJ-809 | `forjar validate --check-resource-overlap` — detect resources with overlapping scope on same machine. Safety. | Done |
| FJ-810 | `forjar status --drift-history` — drift events timeline across fleet. Monitoring. | Done |
| FJ-811 | `forjar graph --dependency-depth-per-resource` — show max chain depth per resource. Analysis. | Done |
| FJ-812 | `forjar status --resource-failure-rate` — failure rate per resource across applies. Monitoring. | Done |

### Phase 69 — Operational Insights & Governance (FJ-813→FJ-820)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-813 | `forjar validate --check-resource-tags` — enforce tag conventions (required tags, naming rules). Governance. | Done |
| FJ-814 | `forjar status --machine-last-apply` — show last apply timestamp per machine. Dashboard. | Done |
| FJ-815 | `forjar graph --resource-fanin` — show fan-in count per resource (how many depend on it). Analysis. | Done |
| FJ-816 | `forjar apply --notify-discord-webhook` — Discord webhook with rich embeds. Integration. | Done |
| FJ-817 | `forjar validate --check-resource-state-consistency` — verify state fields match resource type. Governance. | Done |
| FJ-818 | `forjar status --fleet-drift-summary` — aggregated drift summary across all machines. Monitoring. | Done |
| FJ-819 | `forjar graph --isolated-subgraphs` — detect disconnected subgraphs in the DAG. Analysis. | Done |
| FJ-820 | `forjar status --resource-apply-duration` — average apply duration per resource type. Performance. | Done |

### Phase 70 — Advanced Governance & Analytics (FJ-821→FJ-828)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-821 | `forjar validate --check-resource-dependencies-complete` — verify all depends_on targets exist. Integrity. | Done |
| FJ-822 | `forjar status --machine-resource-health` — per-machine breakdown of resource health status. Dashboard. | Done |
| FJ-823 | `forjar graph --resource-dependency-chain` — full chain from root to leaf per resource. Analysis. | Done |
| FJ-824 | `forjar apply --notify-teams-webhook` — MS Teams webhook with adaptive card. Integration. | Done |
| FJ-825 | `forjar validate --check-machine-connectivity` — verify machines are reachable (dry-run). Governance. | Done |
| FJ-826 | `forjar status --fleet-convergence-trend` — convergence % over last N applies. Trends. | Done |
| FJ-827 | `forjar graph --bottleneck-resources` — resources with highest fan-in AND fan-out. Hotspots. | Done |
| FJ-828 | `forjar status --resource-state-distribution` — distribution of resource states across fleet. Analytics. | Done |

### Phase 71 — Compliance & Observability (FJ-829→FJ-836)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-829 | `forjar validate --check-resource-naming-pattern` — enforce naming pattern for resources. Governance. | Done |
| FJ-830 | `forjar status --machine-apply-count` — total apply count per machine. Dashboard. | Done |
| FJ-831 | `forjar graph --critical-dependency-path` — longest weighted path through the DAG. Analysis. | Done |
| FJ-832 | `forjar apply --notify-slack-blocks` — Slack Block Kit rich notifications. Integration. | Done |
| FJ-833 | `forjar validate --check-resource-provider-support` — verify providers match resource types. Governance. | Done |
| FJ-834 | `forjar status --fleet-apply-history` — recent apply history across all machines. Timeline. | Done |
| FJ-835 | `forjar graph --resource-depth-histogram` — histogram of dependency depths. Visualization. | Done |
| FJ-836 | `forjar status --resource-hash-changes` — track hash changes over time per resource. Forensics. | Done |

### Phase 72 — Security & Fleet Insights (FJ-837→FJ-844)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-837 | `forjar validate --check-resource-secret-refs` — verify secret references exist and are valid. Security. | Done |
| FJ-838 | `forjar status --machine-uptime-estimate` — estimate machine uptime from apply history. Dashboard. | Done |
| FJ-839 | `forjar graph --resource-coupling-score` — coupling score between resource pairs. Analysis. | Done |
| FJ-840 | `forjar apply --notify-custom-template` — custom notification template support. Integration. | Done |
| FJ-841 | `forjar validate --check-resource-idempotency-hints` — check resources have idempotency markers. Quality. | Done |
| FJ-842 | `forjar status --fleet-resource-type-breakdown` — resource type distribution across fleet. Analytics. | Done |
| FJ-843 | `forjar graph --resource-change-frequency` — overlay change frequency on dependency graph. Visualization. | Done |
| FJ-844 | `forjar status --resource-convergence-time` — average time to converge per resource. Performance. | Done |

### Phase 73 — Drift Intelligence & Governance (FJ-845→FJ-852)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-845 | `forjar validate --check-resource-dependency-depth` — warn if dependency chain exceeds threshold. Governance. | Done |
| FJ-846 | `forjar status --machine-drift-age` — age of oldest drift per machine. Monitoring. | Done |
| FJ-847 | `forjar graph --resource-impact-score` — impact score based on dependents + depth. Analysis. | Done |
| FJ-848 | `forjar apply --notify-custom-webhook` — configurable webhook with custom headers. Integration. | Done |
| FJ-849 | `forjar validate --check-resource-machine-affinity` — verify resources match machine capabilities. Governance. | Done |
| FJ-850 | `forjar status --fleet-failed-resources` — list all failed resources across fleet. Alerting. | Done |
| FJ-851 | `forjar graph --resource-stability-score` — stability score based on status history. Analysis. | Done |
| FJ-852 | `forjar status --resource-dependency-health` — health of upstream dependencies per resource. Monitoring. | Done |

### Phase 74 — Predictive Analysis & Fleet Governance (FJ-853→FJ-860)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-853 | `forjar validate --check-resource-drift-risk` — score drift risk per resource based on type + deps. Governance. | Done |
| FJ-854 | `forjar status --machine-resource-age-distribution` — age distribution of resources per machine. Analytics. | Done |
| FJ-855 | `forjar graph --resource-dependency-fanout` — fan-out count per resource. Analysis. | Done |
| FJ-856 | `forjar apply --notify-custom-headers` — custom HTTP headers for webhook notifications. Integration. | Done |
| FJ-857 | `forjar validate --check-resource-tag-coverage` — verify all resources have required tags. Governance. | Done |
| FJ-858 | `forjar status --fleet-convergence-velocity` — rate of convergence across fleet. Monitoring. | Done |
| FJ-859 | `forjar graph --resource-dependency-weight` — weighted edges based on resource coupling. Analysis. | Done |
| FJ-860 | `forjar status --resource-failure-correlation` — correlate failures across resources. Intelligence. | Done |

### Phase 75 — Resource Lifecycle & Operational Intelligence (FJ-861→FJ-868)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-861 | `forjar validate --check-resource-lifecycle-hooks` — verify lifecycle hook references are valid. Governance. | Done |
| FJ-862 | `forjar status --machine-resource-churn-rate` — resource change frequency per machine over time. Analytics. | Done |
| FJ-863 | `forjar graph --resource-dependency-bottleneck` — identify bottleneck resources with high fan-in + fan-out. Analysis. | Done |
| FJ-864 | `forjar apply --notify-custom-json` — custom JSON template for webhook notifications. Integration. | Done |
| FJ-865 | `forjar validate --check-resource-provider-version` — verify provider version compatibility. Governance. | Done |
| FJ-866 | `forjar status --fleet-resource-staleness` — identify resources not applied in configurable window. Monitoring. | Done |
| FJ-867 | `forjar graph --resource-type-clustering` — cluster resources by type and show interconnections. Analysis. | Done |
| FJ-868 | `forjar status --machine-convergence-trend` — convergence trend per machine over time. Intelligence. | Done |

### Phase 76 — Capacity Planning & Configuration Analytics (FJ-869→FJ-876)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-869 | `forjar validate --check-resource-naming-convention` — enforce naming patterns across resources. Governance. | Done |
| FJ-870 | `forjar status --machine-capacity-utilization` — resource density and capacity metrics per machine. Analytics. | Done |
| FJ-871 | `forjar graph --resource-dependency-cycle-risk` — identify near-cycle patterns in dependency graph. Analysis. | Done |
| FJ-872 | `forjar apply --notify-custom-filter` — filter notifications by resource type or status. Integration. | Done |
| FJ-873 | `forjar validate --check-resource-idempotency` — verify resources are idempotent-safe. Governance. | Done |
| FJ-874 | `forjar status --fleet-configuration-entropy` — measure configuration diversity across fleet. Analytics. | Done |
| FJ-875 | `forjar graph --resource-impact-radius` — calculate blast radius of resource changes. Analysis. | Done |
| FJ-876 | `forjar status --machine-resource-freshness` — time since last successful apply per resource. Intelligence. | Done |

### Phase 77 — Operational Maturity & Compliance Automation (FJ-877→FJ-884)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-877 | `forjar validate --check-resource-documentation` — verify resources have descriptions or comments. Governance. | Done |
| FJ-878 | `forjar status --machine-error-budget` — track error budget consumption per machine. SRE. | Done |
| FJ-879 | `forjar graph --resource-dependency-health-map` — overlay health status on dependency graph. Analysis. | Done |
| FJ-880 | `forjar apply --notify-custom-retry` — retry failed notifications with exponential backoff. Integration. | Done |
| FJ-881 | `forjar validate --check-resource-ownership` — verify all resources have assigned owners. Governance. | Done |
| FJ-882 | `forjar status --fleet-compliance-score` — aggregate compliance score across fleet. Compliance. | Done |
| FJ-883 | `forjar graph --resource-change-propagation` — trace how changes propagate through dependencies. Analysis. | Done |
| FJ-884 | `forjar status --machine-mean-time-to-recovery` — MTTR metrics per machine. Intelligence. | Done |

### Phase 78 — Automation Intelligence & Fleet Optimization (FJ-885→FJ-892)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-885 | `forjar validate --check-resource-secret-exposure` — detect secrets accidentally exposed in resource content. Security. | Done |
| FJ-886 | `forjar status --machine-resource-dependency-health` — health of upstream dependencies per resource. Intelligence. | Done |
| FJ-887 | `forjar graph --resource-dependency-depth-analysis` — show max dependency chain depth per resource. Analysis. | Done |
| FJ-888 | `forjar apply --notify-custom-transform` — transform notification payload via template. Integration. | Done |
| FJ-889 | `forjar validate --check-resource-tag-standards` — enforce tag naming standards across resources. Governance. | Done |
| FJ-890 | `forjar status --fleet-resource-type-health` — health breakdown by resource type across fleet. Intelligence. | Done |
| FJ-891 | `forjar graph --resource-dependency-fan-analysis` — combined fan-in/fan-out analysis per resource. Analysis. | Done |
| FJ-892 | `forjar status --machine-resource-convergence-rate` — convergence rate per resource per machine. Intelligence. | Done |

### Phase 79 — Security Hardening & Operational Insights (FJ-893→FJ-900)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-893 | `forjar validate --check-resource-privilege-escalation` — detect resources that could enable privilege escalation. Security. | Done |
| FJ-894 | `forjar status --machine-resource-failure-correlation` — correlate resource failures across machines. Intelligence. | Done |
| FJ-895 | `forjar graph --resource-dependency-isolation-score` — isolation score per resource in dependency graph. Analysis. | Done |
| FJ-896 | `forjar apply --notify-custom-batch` — batch multiple resource notifications into single payload. Integration. | Done |
| FJ-897 | `forjar validate --check-resource-update-safety` — verify resources can be safely updated without downtime. Safety. | Done |
| FJ-898 | `forjar status --fleet-resource-age-distribution` — age distribution of resources across fleet. Intelligence. | Done |
| FJ-899 | `forjar graph --resource-dependency-stability-score` — stability score based on dependency change frequency. Analysis. | Done |
| FJ-900 | `forjar status --machine-resource-rollback-readiness` — readiness for rollback per machine based on state history. Intelligence. | Done |

### Phase 80 — Operational Resilience & Configuration Intelligence (FJ-901→FJ-908)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-901 | `forjar validate --check-resource-cross-machine-consistency` — detect config inconsistencies across machines. Governance. | Done |
| FJ-902 | `forjar status --machine-resource-health-trend` — health trend over time per machine. Intelligence. | Done |
| FJ-903 | `forjar graph --resource-dependency-critical-path-length` — critical path length through dependency graph. Analysis. | Done |
| FJ-904 | `forjar apply --notify-custom-deduplicate` — deduplicate repeated notifications. Integration. | Done |
| FJ-905 | `forjar validate --check-resource-version-pinning` — verify resources pin explicit versions. Governance. | Done |
| FJ-906 | `forjar status --fleet-resource-drift-velocity` — rate of drift accumulation across fleet. Intelligence. | Done |
| FJ-907 | `forjar graph --resource-dependency-redundancy-score` — redundancy score for resources with fallbacks. Analysis. | Done |
| FJ-908 | `forjar status --machine-resource-apply-success-trend` — apply success trend per machine over time. Intelligence. | Done |

### Phase 81 — Predictive Analytics & Configuration Quality (FJ-909→FJ-916)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-909 | `forjar validate --check-resource-dependency-completeness` — verify all dependencies exist and are reachable. Governance. | Done |
| FJ-910 | `forjar status --machine-resource-mttr-estimate` — estimated MTTR per resource based on history. Intelligence. | Done |
| FJ-911 | `forjar graph --resource-dependency-centrality-score` — betweenness centrality for critical resources. Analysis. | Done |
| FJ-912 | `forjar apply --notify-custom-throttle` — throttle notification rate per time window. Integration. | Done |
| FJ-913 | `forjar validate --check-resource-state-coverage` — verify all resources declare explicit states. Governance. | Done |
| FJ-914 | `forjar status --fleet-resource-convergence-forecast` — forecast time to full convergence. Intelligence. | Done |
| FJ-915 | `forjar graph --resource-dependency-bridge-detection` — find bridge edges whose removal disconnects the graph. Analysis. | Done |
| FJ-916 | `forjar status --machine-resource-error-budget-forecast` — forecast error budget consumption rate. Intelligence. | Done |

### Phase 82 — Infrastructure Insight & Configuration Maturity (FJ-917→FJ-924)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-917 | `forjar validate --check-resource-rollback-safety` — verify resources can be safely rolled back without side effects. Governance. | Done |
| FJ-918 | `forjar status --machine-resource-dependency-lag` — detect lag between dependent resource convergence. Intelligence. | Done |
| FJ-919 | `forjar graph --resource-dependency-cluster-coefficient` — clustering coefficient per resource in dependency graph. Analysis. | Done |
| FJ-920 | `forjar apply --notify-custom-aggregate` — aggregate multiple events into summary notification. Integration. | Done |
| FJ-921 | `forjar validate --check-resource-config-maturity` — score resource configuration maturity (tags, docs, versioning). Governance. | Done |
| FJ-922 | `forjar status --fleet-resource-dependency-lag` — fleet-wide dependency convergence lag analysis. Intelligence. | Done |
| FJ-923 | `forjar graph --resource-dependency-modularity-score` — modularity score for resource dependency communities. Analysis. | Done |
| FJ-924 | `forjar status --machine-resource-config-drift-rate` — rate of configuration drift per machine over time. Intelligence. | Done |

### Phase 83 — Advanced Graph Analytics & Fleet Observability (FJ-925→FJ-932)

| Ticket | Description | Status |
|--------|-------------|--------|
| FJ-925 | `forjar validate --check-resource-dependency-ordering` — verify dependency ordering is topologically valid. Governance. | Planned |
| FJ-926 | `forjar status --machine-resource-convergence-lag` — per-resource convergence lag within machine. Intelligence. | Planned |
| FJ-927 | `forjar graph --resource-dependency-diameter` — longest shortest path (graph diameter) in dependency graph. Analysis. | Planned |
| FJ-928 | `forjar apply --notify-custom-priority` — assign priority levels to notifications based on severity. Integration. | Planned |
| FJ-929 | `forjar validate --check-resource-tag-completeness` — ensure all resources have required tag categories. Governance. | Planned |
| FJ-930 | `forjar status --fleet-resource-convergence-lag` — fleet-wide per-resource convergence lag analysis. Intelligence. | Planned |
| FJ-931 | `forjar graph --resource-dependency-eccentricity` — eccentricity (max shortest path) per resource. Analysis. | Planned |
| FJ-932 | `forjar status --machine-resource-dependency-depth` — dependency chain depth per resource per machine. Intelligence. | Planned |

---

## 9. Performance Targets

| Operation | Target | Rationale |
|-----------|--------|-----------|
| `forjar validate` | < 10ms | Pure YAML parse, no I/O |
| `forjar plan` (3 machines, 20 resources) | < 2s | Parallel SSH + BLAKE3 hash |
| `forjar drift` (3 machines, 100 files) | < 1s | BLAKE3 is 4GB/s on modern CPUs |
| `forjar apply` (no changes) | < 500ms | Hash compare only, no shell exec |
| Copia signature (1MB) | ~294 µs | BLAKE3 per-block hashing (256 blocks) |
| Copia signature (4MB) | ~1.19 ms | BLAKE3 per-block hashing (1024 blocks) |
| Copia signature (16MB) | ~5.0 ms | BLAKE3 per-block hashing (4096 blocks) |
| Copia delta (4MB, 2% change) | ~1.18 ms | Block-by-block hash comparison |
| Copia delta (4MB, 100% change) | ~1.26 ms | Worst case: all blocks differ |
| Copia patch script (1MB, 10%) | ~60 µs | Base64 encode literals + dd commands |
| Copia parse sigs (1024 blocks) | ~57 µs | Parse remote signature output |
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

**GPU test matrix** (feature-gated: `--features gpu-container-test`):

| Test | Description |
|------|-------------|
| CUDA lifecycle | ensure → exec → cleanup with `--gpus all` |
| CUDA nvidia-smi | Verify `nvidia-smi --query-gpu` returns GPU name |
| CUDA env vars | `CUDA_VISIBLE_DEVICES` passed through `--env` |
| ROCm lifecycle | ensure → exec → cleanup with `--device /dev/kfd /dev/dri` |
| ROCm device access | Verify `/dev/kfd` and `/dev/dri` accessible in container |
| ROCm env vars | `ROCR_VISIBLE_DEVICES` passed through `--env` |
| Cross-vendor config | Same model config deployed to both CUDA and ROCm containers |

**Running**:
```bash
# Basic container tests
docker build -t forjar-test-target -f tests/Dockerfile.test-target .
cargo test --features container-test

# GPU container tests (requires NVIDIA Container Toolkit + AMD ROCm drivers)
cargo test --features gpu-container-test
```

### 10.6 Dogfood Workflow

30 dogfood configs exercise all 11 resource types and cross-cutting features. Container transport configs enable end-to-end testing without root or host pollution; localhost configs validate codegen and planning.

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
| `dogfood-conditions.yaml` | file, package | Conditional resources (`when:`), expression evaluation |
| `dogfood-iteration.yaml` | file | Resource iteration: `count:` ({{index}}), `for_each:` ({{item}}), dep rewriting |
| `dogfood-outputs.yaml` | file | Output values: `outputs:` block, `{{params.*}}`, `{{machine.NAME.FIELD}}` |
| `dogfood-data.yaml` | file | External data sources (`data:` block), file/env/dns lookups |
| `dogfood-notify.yaml` | file | Notification hooks, post-apply webhooks |
| `dogfood-policies.yaml` | file | Policy-as-code rules, compliance checking |
| `dogfood-triggers.yaml` | file | Resource triggers, restart_on dependencies |
| `dogfood-age-secrets.yaml` | file | Age-encrypted secrets, `ENC[age,...]` markers, env-var fallback |
| `dogfood-apr-serve.yaml` | recipe | GPU inference server recipe (apr-inference-server), 8 resources |
| `dogfood-repartir.yaml` | recipe | Distributed worker recipe (repartir-worker), 10 resources |
| `dogfood-renacer.yaml` | recipe | Observability stack recipe (renacer-observability), 10 resources |
| `dogfood-sovereign-stack.yaml` | recipe | Multi-machine sovereign AI stack, 3 machines, 33 resources |
| `dogfood-multi-gpu.yaml` | file | Multi-vendor GPU container transport: NVIDIA CUDA (--gpus) + AMD ROCm (--device/--group-add), env vars, parallel machines |
| `dogfood-apr-qa.yaml` | recipe | Multi-vendor GPU model QA: apr-model-qa recipe × 2 machines (CUDA + ROCm), 10 resources, playbook test matrix |
| `dogfood-template-funcs.yaml` | file | Template functions: upper/lower/trim/default/replace/env/b3sum/join/split, nested calls |
| `dogfood-includes.yaml` | file | Config includes: merge params/machines/resources from included files |

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

All 11 resource types (`file`, `package`, `service`, `mount`, `user`, `docker`, `cron`, `network`, `pepita`, `model`, `gpu`) plus the `recipe` composite type have dedicated dogfood configs validating their codegen, state queries, and edge cases.

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
age = { version = "0.11", default-features = false }  # Age encryption for secrets (FJ-200)
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

1. ~~**Secrets**: Encrypt in git (age/sops-style) or external vault?~~ **Resolved**: Dual approach — `{{secrets.KEY}}` resolves from `FORJAR_SECRET_*` env vars (FJ-062), and `ENC[age,...]` markers provide encrypted-at-rest values decrypted at resolve time (FJ-200/201). Identity from `FORJAR_AGE_KEY` env var or `--identity` flag. Secret rotation via `forjar secrets rotate --re-encrypt`.
2. ~~**Rollback**: Should `forjar rollback` replay the previous state, or just show the diff?~~ **Resolved**: `forjar rollback -n N` reads previous `forjar.yaml` from `git show HEAD~N`, compares changes, and re-applies with `--force`. Supports `--dry-run` for safe preview (FJ-080).
3. ~~**Import**: Should `forjar import` be able to adopt existing infrastructure?~~ **Resolved**: `forjar import --addr <host>` scans packages, services, and config files, generates forjar.yaml (FJ-065).
4. ~~**Multi-repo**: Should machines be able to be managed by multiple forjar repos?~~ **Resolved**: No — one repo per fleet, sovereignty principle. Enforced by convention; lint warns on cross-machine dependencies.
5. ~~**Systemd in containers**: Service resources require systemd. Should forjar detect when running inside a container without systemd and skip/warn?~~ **Resolved**: All service scripts now include a systemd guard (`command -v systemctl`) that gracefully exits 0 with a `FORJAR_WARN` message when systemd is unavailable (FJ-081).
