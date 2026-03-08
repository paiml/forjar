# Resource Types

## Package

Install or remove system packages.

```yaml
resources:
  dev-tools:
    type: package
    machine: m1
    provider: apt          # apt | cargo | uv | brew
    packages: [curl, git, htop]
    state: present         # present (default) | absent
    version: "1.2.3"       # optional version pin
```

### Providers

| Provider | Install Command | Version Syntax | Remove Command |
|----------|----------------|----------------|----------------|
| `apt` | `apt-get install -y` (auto-sudo if non-root) | `package=version` | `apt-get remove -y` |
| `cargo` | `cargo install --force` | `package@version` | — |
| `uv` | `uv tool install --force` | `package==version` | `uv tool uninstall` |
| `brew` | `brew install` | `package@version` | `brew uninstall` |

### Cross-Platform Packages (Homebrew)

Use the `brew` provider for cross-platform package management on macOS and Linux:

```yaml
resources:
  dev-tools:
    type: package
    machine: m1
    provider: brew
    packages: [jq, ripgrep, fd, bat]

  python-pinned:
    type: package
    machine: m1
    provider: brew
    packages: [python]
    version: "3.12"
```

The brew provider generates idempotent scripts that check `brew list` before installing, avoiding redundant operations.

### Version Pinning

Pin all packages in a resource to a specific version:

```yaml
  nginx:
    type: package
    machine: web-server
    provider: apt
    packages: [nginx]
    version: "1.18.0-0ubuntu1"
```

## File

Manage files, directories, and symlinks.

### Regular File

```yaml
resources:
  config:
    type: file
    machine: m1
    path: /etc/app/config.yaml
    content: |
      database:
        host: localhost
        port: 5432
    owner: app
    group: app
    mode: "0640"
```

Content is written via heredoc (`<<'FORJAR_EOF'`) — shell variable expansion is prevented.

### Source File Transfer

Instead of inline content, use `source` to transfer a local file:

```yaml
resources:
  entrypoint:
    type: file
    machine: m1
    path: /opt/app/entrypoint.sh
    source: scripts/entrypoint.sh    # local path, read at apply time
    owner: app
    mode: "0755"
```

For files ≤ 1MB, the source is base64-encoded locally and decoded on the remote machine via `base64 -d`. For files > 1MB, forjar uses **copia delta sync** (FJ-242) — an rsync-style block-level transfer that only sends changed 4KB blocks. This is critical for deploying multi-GB model files where only a small percentage of blocks change after fine-tuning. The delta protocol:

1. Signature: get per-block BLAKE3 hashes from the remote file
2. Delta: compare local blocks against remote hashes
3. Patch: transfer only changed blocks + copy unchanged blocks from existing file
4. Atomic replace: temp file + `mv` (no partial writes)

Falls back to full base64 for new files (no remote state to diff against). Both modes work with all transports (local, SSH, container, pepita).

`content` and `source` are mutually exclusive — use one or the other.

### Directory

```yaml
resources:
  data-dir:
    type: file
    machine: m1
    state: directory
    path: /var/lib/app/data
    owner: app
    mode: "0755"
```

Creates the directory (and parents) with `mkdir -p`.

### Symlink

```yaml
resources:
  tool-link:
    type: file
    machine: m1
    state: symlink
    path: /usr/local/bin/tool
    target: /opt/tool/bin/tool
```

### Absent (Delete)

```yaml
resources:
  old-config:
    type: file
    machine: m1
    state: absent
    path: /etc/old-app.conf
```

Removes the file or directory with `rm -rf`.

### File Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | string | required | Absolute file path |
| `content` | string | — | Inline file content (mutually exclusive with source) |
| `source` | string | — | Local file path to transfer (mutually exclusive with content) |
| `state` | string | `file` | file, directory, symlink, absent |
| `target` | string | — | Symlink target (state=symlink only) |
| `owner` | string | — | File owner |
| `group` | string | — | File group |
| `mode` | string | — | Octal permissions (e.g. "0644") |

## Service

Manage systemd services. Includes automatic systemd detection — if `systemctl` is not available (e.g. inside containers without systemd), service resources are gracefully skipped with a warning rather than failing.

```yaml
resources:
  nginx:
    type: service
    machine: m1
    name: nginx
    state: running         # running | stopped
    enabled: true          # Enable on boot
    restart_on: [config]   # Restart when these resources change
```

### Service States

| State | Action |
|-------|--------|
| `running` | `systemctl start` + optionally `systemctl enable` |
| `stopped` | `systemctl stop` + optionally `systemctl disable` |
| `enabled` | `systemctl enable` (no start/stop) |
| `disabled` | `systemctl disable` (no start/stop) |

### Restart Triggers

Use `restart_on` to restart a service when a dependency changes:

```yaml
resources:
  nginx-conf:
    type: file
    machine: m1
    path: /etc/nginx/nginx.conf
    content: "..."

  nginx:
    type: service
    machine: m1
    name: nginx
    state: running
    restart_on: [nginx-conf]
    depends_on: [nginx-conf]
```

## Mount

Manage filesystem mounts.

```yaml
resources:
  data-mount:
    type: mount
    machine: m1
    source: /dev/sdb1
    path: /mnt/data
    fstype: ext4
    options: "defaults,noatime"
    state: mounted         # mounted | unmounted | absent
```

### NFS Mount

```yaml
resources:
  nfs-share:
    type: mount
    machine: m1
    source: "192.168.1.10:/exports/data"
    path: /mnt/nfs
    fstype: nfs
    options: "rw,soft,intr"
    state: mounted
```

### Mount Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `source` | string | required | Device or NFS path |
| `path` | string | required | Mount point |
| `fstype` | string | — | Filesystem type (ext4, nfs, etc.) |
| `options` | string | — | Mount options |
| `state` | string | `mounted` | mounted, unmounted, absent |

## User

Manage local system users and groups via `useradd`/`usermod`/`userdel`.

```yaml
resources:
  deploy-user:
    type: user
    machine: m1
    name: deploy
    shell: /bin/bash
    home: /home/deploy
    groups: [docker, sudo]
    ssh_authorized_keys:
      - "ssh-ed25519 AAAA... deploy@workstation"
```

### System Users

```yaml
resources:
  prometheus:
    type: user
    machine: m1
    name: prometheus
    system_user: true
    shell: /usr/sbin/nologin
```

System users are created with `--system` and do not get a home directory by default.

### Remove a User

```yaml
resources:
  old-user:
    type: user
    machine: m1
    name: olduser
    state: absent
```

### User Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Username |
| `state` | string | `present` | present, absent |
| `uid` | integer | — | Explicit UID |
| `group` | string | — | Primary group (--gid) |
| `groups` | [string] | [] | Supplementary groups (auto-created if missing) |
| `shell` | string | — | Login shell |
| `home` | string | `/home/{name}` | Home directory |
| `system_user` | bool | false | Create as system user (--system) |
| `ssh_authorized_keys` | [string] | [] | SSH public keys for ~/.ssh/authorized_keys |

## Docker

Manage Docker containers as deployed resources. This is distinct from container *transport* (using containers as execution targets) — this manages containers running ON machines.

```yaml
resources:
  web:
    type: docker
    machine: m1
    name: web
    image: nginx:latest
    state: running
    ports: ["8080:80", "443:443"]
    volumes: ["/data/web:/usr/share/nginx/html"]
    environment: ["NGINX_HOST=example.com"]
    restart: unless-stopped
```

### Docker States

| State | Action |
|-------|--------|
| `running` | Pull image, stop/remove existing, `docker run -d` |
| `stopped` | `docker stop` |
| `absent` | `docker stop` + `docker rm` |

### Port Mappings

Expose container ports to the host using `host:container` syntax:

```yaml
resources:
  reverse-proxy:
    type: docker
    machine: m1
    name: traefik
    image: traefik:v3.0
    state: running
    ports:
      - "80:80"           # HTTP
      - "443:443"         # HTTPS
      - "8080:8080"       # Dashboard
    restart: unless-stopped
```

Each entry in `ports` maps to a `-p` flag in the generated `docker run` command. The format follows Docker's standard `[host_ip:]host_port:container_port[/protocol]` syntax. All port values are single-quoted in the generated script to prevent injection.

### Volume Mounts

Persist data across container restarts with host-to-container volume mappings:

```yaml
resources:
  database:
    type: docker
    machine: m1
    name: postgres
    image: postgres:16
    state: running
    volumes:
      - "/data/pg:/var/lib/postgresql/data"   # Data directory
      - "/etc/pg/pg_hba.conf:/etc/postgresql/pg_hba.conf:ro"  # Read-only config
    environment:
      - "POSTGRES_PASSWORD={{secrets.db-password}}"
    restart: unless-stopped
```

Volume entries map to `-v` flags. The standard Docker volume syntax applies: `host_path:container_path[:options]`. Options include `ro` (read-only), `rw` (read-write, default), and propagation flags.

### Environment Variables

Pass configuration to containers via environment variables:

```yaml
resources:
  api-server:
    type: docker
    machine: m1
    name: api
    image: myapp/api:v2.1
    state: running
    ports: ["8080:8080"]
    environment:
      - "NODE_ENV=production"
      - "DATABASE_URL=postgresql://app:{{secrets.db-password}}@{{machine.db.addr}}:5432/myapp"
      - "REDIS_URL=redis://{{machine.cache.addr}}:6379"
      - "LOG_LEVEL=info"
    restart: on-failure
```

Environment variables support template resolution -- `{{params.*}}`, `{{secrets.*}}`, and `{{machine.*.*}}` references are resolved before script generation. Each entry maps to a `-e` flag.

### Restart Policies

Control container restart behavior on failure or host reboot:

```yaml
resources:
  worker:
    type: docker
    machine: m1
    name: background-worker
    image: myapp/worker:v2.1
    state: running
    restart: on-failure       # Restart only on non-zero exit
    command: "./worker --queue=default --concurrency=4"

  monitoring:
    type: docker
    machine: m1
    name: prometheus
    image: prom/prometheus:v2.51.0
    state: running
    ports: ["9090:9090"]
    volumes:
      - "/etc/prometheus:/etc/prometheus:ro"
      - "/data/prometheus:/prometheus"
    restart: always           # Always restart, including on host reboot
```

| Restart Policy | Behavior |
|---------------|----------|
| `no` | Never restart (default Docker behavior) |
| `always` | Always restart, including on daemon startup |
| `unless-stopped` | Like `always`, but not if explicitly stopped |
| `on-failure` | Restart only on non-zero exit code |

When `restart` is omitted from the forjar resource, no `--restart` flag is passed to Docker, which means Docker's default (`no`) applies.

### Complete Docker Example

A full-stack deployment combining multiple Docker resources with dependencies:

```yaml
resources:
  app-data:
    type: file
    machine: m1
    state: directory
    path: /data/app
    mode: "0755"

  redis:
    type: docker
    machine: m1
    name: redis
    image: redis:7-alpine
    state: running
    ports: ["6379:6379"]
    volumes: ["/data/app/redis:/data"]
    restart: unless-stopped
    command: "redis-server --appendonly yes"

  app:
    type: docker
    machine: m1
    name: app
    image: myapp:{{params.app_version}}
    state: running
    ports: ["8080:8080"]
    environment:
      - "REDIS_URL=redis://localhost:6379"
      - "SECRET_KEY={{secrets.app-secret}}"
    restart: unless-stopped
    depends_on: [redis, app-data]
```

### Docker Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Container name |
| `image` | string | required | OCI image (e.g. `nginx:latest`) |
| `state` | string | `running` | running, stopped, absent |
| `ports` | [string] | [] | Port mappings (`host:container`) |
| `volumes` | [string] | [] | Volume mounts (`host:container[:options]`) |
| `environment` | [string] | [] | Environment variables (`KEY=VALUE`) |
| `restart` | string | -- | Restart policy (no, always, unless-stopped, on-failure) |
| `command` | string | -- | Override container command |

## Cron

Manage scheduled tasks via crontab entries. Jobs are tagged with `# forjar:{name}` comments for idempotent updates.

```yaml
resources:
  backup:
    type: cron
    machine: m1
    name: nightly-backup
    schedule: "0 2 * * *"
    command: /usr/local/bin/backup.sh
    owner: root
```

### Remove a Cron Job

```yaml
resources:
  old-job:
    type: cron
    machine: m1
    name: old-job
    state: absent
    owner: root
```

### Cron Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Job identifier (used as tag in crontab) |
| `state` | string | `present` | present, absent |
| `schedule` | string | `* * * * *` | Cron schedule expression |
| `command` | string | required | Command to execute |
| `owner` | string | `root` | Crontab user |

## Network

Manage firewall rules via ufw (Uncomplicated Firewall).

```yaml
resources:
  allow-ssh:
    type: network
    machine: m1
    name: ssh-access
    port: "22"
    protocol: tcp
    action: allow
    from_addr: 192.168.1.0/24
```

### Network States

| State | Action |
|-------|--------|
| `present` | Add ufw rule (enables ufw if not active) |
| `absent` | `ufw delete` the rule |

### Network Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | — | Rule comment |
| `state` | string | `present` | present, absent |
| `port` | string | required | Port number |
| `protocol` | string | `tcp` | tcp, udp |
| `action` | string | `allow` | allow, deny, reject |
| `from_addr` | string | — | Source address/CIDR (e.g. `192.168.1.0/24`) |

## Pepita (Kernel Isolation)

Manage Linux kernel namespace isolation without a container runtime. Pepita resources use kernel primitives (cgroups v2, overlayfs, network namespaces, chroot, seccomp) for bare-metal isolation.

```yaml
resources:
  sandbox:
    type: pepita
    machine: m1
    name: sandbox
    state: present
    memory_limit: 536870912    # 512 MiB
    cpuset: "0-3"
    netns: true
    seccomp: true
    chroot_dir: /var/lib/forjar/sandbox
```

### Pepita States

| State | Action |
|-------|--------|
| `present` | Create cgroups, mount overlayfs, add network namespace, create chroot |
| `absent` | Unmount overlayfs, delete namespace, remove cgroup, remove chroot |

### Cgroup Resource Limits

Control memory and CPU allocation via cgroups v2:

```yaml
resources:
  worker-limits:
    type: pepita
    machine: m1
    name: worker
    memory_limit: 1073741824   # 1 GiB in bytes
    cpuset: "0,2,4"            # Bind to CPUs 0, 2, 4
```

Generated script:

```bash
set -euo pipefail
mkdir -p '/sys/fs/cgroup/forjar-worker'
echo '1073741824' > '/sys/fs/cgroup/forjar-worker/memory.max'
echo '0,2,4' > '/sys/fs/cgroup/forjar-worker/cpuset.cpus'
```

### Network Namespace Isolation

Isolate network stacks per workload:

```yaml
resources:
  isolated-net:
    type: pepita
    machine: m1
    name: isolated
    netns: true
```

Creates `forjar-isolated` network namespace with loopback interface:

```bash
ip netns add 'forjar-isolated' 2>/dev/null || true
ip netns exec 'forjar-isolated' ip link set lo up
```

### Overlay Filesystem

Copy-on-write filesystem layers — write changes to upper layer without modifying the base:

```yaml
resources:
  build-env:
    type: pepita
    machine: m1
    name: build
    overlay_lower: /base/rootfs
    overlay_upper: /var/forjar/upper
    overlay_work: /var/forjar/work
    overlay_merged: /mnt/build
```

### Full Sandbox

Combine all isolation features:

```yaml
resources:
  full-sandbox:
    type: pepita
    machine: m1
    name: sandbox
    state: present
    chroot_dir: /var/sandbox
    namespace_uid: 65534
    namespace_gid: 65534
    seccomp: true
    netns: true
    cpuset: "0-1"
    memory_limit: 536870912
    overlay_lower: /base
    overlay_upper: /var/sandbox/upper
    overlay_work: /var/sandbox/work
    overlay_merged: /var/sandbox/merged
```

### Pepita Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Isolation domain name |
| `state` | string | `present` | present, absent |
| `chroot_dir` | string | — | Chroot directory path |
| `namespace_uid` | u32 | — | UID for user namespace mapping |
| `namespace_gid` | u32 | — | GID for user namespace mapping |
| `seccomp` | bool | `false` | Enable seccomp syscall filtering |
| `netns` | bool | `false` | Create network namespace |
| `cpuset` | string | — | CPU set binding (e.g., `"0-3"` or `"0,2,4"`) |
| `memory_limit` | u64 | — | Memory limit in bytes |
| `overlay_lower` | string | `/` | Overlay lower (read-only) directory |
| `overlay_upper` | string | `/tmp/forjar-upper` | Overlay upper (writable) directory |
| `overlay_work` | string | `/tmp/forjar-work` | Overlay work directory |
| `overlay_merged` | string | — | Overlay merged mount point (enables overlayfs) |

## Model (ML)

Manages ML model files — download, integrity verification, and cache management.

```yaml
resources:
  llama-7b:
    type: model
    machine: gpu-box
    name: llama-7b
    source: "TheBloke/Llama-2-7B-GGUF"
    path: /models/llama-7b-q4.gguf
    format: gguf
    quantization: q4_k_m
    checksum: "abc123def456..."
    cache_dir: /opt/model-cache
    owner: noah
```

### Model States

- **present** (default): Download model if missing, verify checksum if provided
- **absent**: Remove the model file

### Model Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Model identifier |
| `source` | string | — | Download source: HuggingFace repo ID (`user/repo`), URL (`https://...`), or local path (`/path/to/model`) |
| `path` | string | — | Destination path on the target machine |
| `format` | string | — | Model format: `gguf`, `safetensors`, `apr` |
| `quantization` | string | — | Quantization level: `q4_k_m`, `q5_k_m`, `q8_0`, `f16`, `none` |
| `checksum` | string | — | BLAKE3 hash for integrity pinning (prevents unauthorized model swaps) |
| `cache_dir` | string | `~/.cache/apr/` | Local cache directory for downloaded models |
| `owner` | string | — | File owner after download |

### Download Sources

The `source` field supports three formats:

1. **HuggingFace repo ID** (`TheBloke/Llama-2-7B-GGUF`): Uses `apr pull` if available, falls back to `huggingface-cli`
2. **URL** (`https://example.com/model.gguf`): Downloads via `curl`
3. **Local path** (`/shared/models/llama.gguf`): Copies from local filesystem

### Drift Detection

When `checksum` is set, `forjar drift` detects unauthorized model file changes by comparing the stored BLAKE3 hash against the live file hash. This catches:
- Model file corruption
- Unauthorized model swaps (e.g., replacing a quantized model with a different version)
- Accidental overwrites

## GPU

Multi-vendor GPU resource handler supporting NVIDIA (CUDA), AMD (ROCm), and CPU (no-op) backends.

```yaml
resources:
  # NVIDIA GPU (default)
  gpu-nvidia:
    type: gpu
    machine: gpu-box
    gpu_backend: nvidia
    driver_version: "550"
    cuda_version: "12.4"
    devices: [0, 1]
    persistence_mode: true
    compute_mode: exclusive_process

  # AMD ROCm GPU
  gpu-rocm:
    type: gpu
    machine: amd-box
    gpu_backend: rocm
    driver_version: "6.3"
    rocm_version: "6.3"

  # CPU-only (no-op, useful for testing)
  gpu-cpu:
    type: gpu
    machine: test-box
    gpu_backend: cpu
```

### GPU Backends

| Backend | Check | Apply | State Query |
|---------|-------|-------|-------------|
| `nvidia` (default) | `nvidia-smi` + driver version | `apt install nvidia-driver-{ver}`, `cuda-toolkit-{ver}` | `nvidia-smi --query-gpu=...` |
| `rocm` | `rocminfo` + `/sys/module/amdgpu/version` | `apt install amdgpu-dkms rocm-hip-runtime` | `rocminfo` device listing |
| `cpu` | Always passes (no-op) | No-op | `echo cpu-only` |

### GPU States

- **present** (default): Install driver and toolkit, enable persistence, set compute mode
- **absent**: Remove GPU drivers (nvidia or rocm)

### GPU Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `gpu_backend` | string | `nvidia` | GPU vendor: `nvidia`, `rocm`, or `cpu` |
| `name` | string | `gpu0` | GPU resource identifier |
| `driver_version` | string | required | Driver version (nvidia driver or amdgpu-dkms) |
| `cuda_version` | string | — | CUDA toolkit version (nvidia only) |
| `rocm_version` | string | — | ROCm version (rocm only, e.g., `6.0`) |
| `devices` | [integer] | all | GPU device indices |
| `persistence_mode` | bool | `true` | Enable `nvidia-persistenced` (nvidia) |
| `compute_mode` | string | `default` | `default`, `exclusive_process`, or `prohibited` |
| `gpu_memory_limit_mb` | integer | — | cgroup GPU memory limit in MB |

### State Query

GPU state is queried per backend:
- **nvidia**: `nvidia-smi --query-gpu=driver_version,compute_mode,memory.total`
- **rocm**: `rocminfo` for device listing, `/sys/module/amdgpu/version` for driver version
- **cpu**: Returns `cpu-only` (always converged)

### Conditional GPU Resources in Recipes

When using GPU resources in recipes, you can conditionally skip GPU-dependent resources in cpu mode using the `when` field with input templates:

```yaml
recipe:
  inputs:
    gpu_backend:
      default: nvidia

resources:
  gpu-driver:
    type: gpu
    gpu_backend: "{{inputs.gpu_backend}}"

  model-download:
    type: model
    depends_on: [gpu-driver]
    when: '{{inputs.gpu_backend}} != "cpu"'  # skipped in cpu mode
```

## Task

Run arbitrary commands with idempotency checks, timeouts, and output artifact tracking. Tasks are the escape hatch for operations that don't fit other resource types.

```yaml
resources:
  build-app:
    type: task
    machine: m1
    command: |
      gcc -o /opt/app/bin src/main.c -Wall -Wextra
    working_dir: /opt/app
    timeout: 120
    completion_check: "test -x /opt/app/bin"
    output_artifacts:
      - /opt/app/bin
```

### Idempotency

Tasks are not inherently idempotent. Use `completion_check` or `output_artifacts` to make them so:

- **`completion_check`**: A shell command that exits 0 if the task is already done. If it exits 0, the task command is skipped.
- **`output_artifacts`**: A list of file paths. If all exist, the task is considered complete.
- If neither is set, the task runs on every apply.

### Timeout

The `timeout` field (in seconds) wraps the command with `timeout(1)`. If the command exceeds the limit, it is killed and the resource fails:

```yaml
resources:
  long-build:
    type: task
    machine: m1
    command: "make -j$(nproc) all"
    working_dir: /opt/project
    timeout: 300
```

### Pipeline Pattern

Chain tasks with `depends_on` to build multi-stage pipelines:

```yaml
resources:
  generate:
    type: task
    machine: m1
    command: "python3 generate.py > output.json"
    working_dir: /opt/pipeline
    output_artifacts: [/opt/pipeline/output.json]

  validate:
    type: task
    machine: m1
    command: "python3 validate.py output.json"
    working_dir: /opt/pipeline
    completion_check: "test -f /opt/pipeline/validated.ok"
    depends_on: [generate]

  deploy:
    type: task
    machine: m1
    command: "cp output.json /srv/data/"
    working_dir: /opt/pipeline
    depends_on: [validate]
```

### Task Modes (FJ-2700)

Tasks support four execution modes via the `task_mode` field:

| Mode | Description | Key Fields |
|------|-------------|------------|
| `batch` | Run-once task (default) | `command`, `completion_check`, `output_artifacts` |
| `pipeline` | Multi-stage execution | `stages`, `cache` |
| `service` | Long-running process | `command`, `restart`, `restart_delay` |
| `dispatch` | On-demand via `forjar run` | `command`, `params` |

**Pipeline mode** replaces `command` with `stages`:

```yaml
resources:
  ci-pipeline:
    type: task
    machine: m1
    task_mode: pipeline
    stages:
      - name: test
        command: "cargo test"
        gate: true           # Pipeline stops if this stage fails
      - name: build
        command: "cargo build --release"
        inputs: ["src/**/*.rs"]
        outputs: ["target/release/app"]
    cache: true               # Skip stages whose inputs haven't changed
```

**Service mode** manages long-running processes:

```yaml
resources:
  app-server:
    type: task
    machine: m1
    task_mode: service
    command: "app serve --port 8080"
    gpu_device: 0             # Injects CUDA_VISIBLE_DEVICES=0
    restart: on_failure
    restart_delay: 10
```

### Task Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | string | required* | Shell command to execute (*optional for pipeline mode) |
| `task_mode` | string | `batch` | Execution mode: batch, pipeline, service, dispatch |
| `working_dir` | string | — | Working directory (cd before execution) |
| `timeout` | u64 | — | Timeout in seconds |
| `completion_check` | string | — | Shell command that exits 0 if already done |
| `output_artifacts` | list | `[]` | Files that must exist for task to be considered complete |
| `stages` | list | `[]` | Pipeline stages (pipeline mode only) |
| `cache` | bool | `false` | Content-addressed stage caching (pipeline mode) |
| `gpu_device` | u32 | — | GPU device index for CUDA_VISIBLE_DEVICES |
| `restart` | string | — | Restart policy: on_failure, always, never (service mode) |
| `restart_delay` | u64 | — | Seconds before restart (service mode) |

### Quality Gates (FJ-2702)

Pipeline stages can include quality gates that control pipeline flow:

- **Exit code**: Non-zero exit stops the pipeline (when `gate: true`)
- **JSON field**: Parse stdout as JSON, check a field value against allowed thresholds
- **Regex**: Match stdout against a regex pattern
- **Numeric threshold**: Verify a JSON field meets a minimum value

The `on_fail` field controls behavior: `block` (default), `warn`, or `skip_dependents`.

### Input/Output Tracking (FJ-2701)

Pipeline stages support content-addressed caching via BLAKE3 hashing:

```yaml
stages:
  - name: build
    command: cargo build --release
    inputs: ["src/**/*.rs", "Cargo.toml"]
    outputs: ["target/release/app"]
    gate: true
```

When `cache: true`, forjar hashes all input files before execution. If the input hash matches the stored hash from the previous run, the stage is skipped. Output artifacts are also hashed for drift detection.

### Pipeline Execution Engine

The pipeline engine processes stages sequentially with cache-aware skipping:

```
for each stage:
    if cache enabled AND inputs unchanged (BLAKE3 match) → SKIP
    execute stage command
    if gate AND exit_code != 0 → FAIL pipeline
    record stage state (exit_code, duration, input_hash)
```

Run the pipeline execution example:

```bash
cargo run --example pipeline_execution
```

### Task State Model (FJ-2706)

Each task mode tracks specific state in the lock file:

| Mode | State | Key Metrics |
|------|-------|-------------|
| Pipeline | Per-stage status (pending/running/passed/failed/skipped) | `last_completed` stage index, per-stage duration, `input_hash` |
| Service | PID, health status, restart count | `consecutive_failures`, `last_check` timestamp |
| Dispatch | Invocation history | `total_invocations`, per-invocation duration and exit code |

## Recipe (Composition)

Compose reusable child recipes into larger configurations. Recipe resources reference external recipe YAML files and forward inputs.

```yaml
resources:
  web-stack:
    type: recipe
    machine: m1
    recipe: web-server
    inputs:
      domain: example.com
      port: 8080
```

### How Expansion Works

When forjar encounters a `type: recipe` resource, it:

1. Loads the child recipe file from `recipes/{recipe_name}.yaml`
2. Validates provided inputs against the recipe's `inputs:` declarations
3. Expands child resources with namespaced IDs (e.g., `web-stack/nginx-pkg`)
4. Rewrites internal `depends_on` references to use namespaced IDs
5. Propagates the parent's `machine` target to all child resources

### Input Forwarding

Forward parent params to child recipe inputs:

```yaml
params:
  app_name: myapp
  listen_port: "8080"

resources:
  app-scaffold:
    type: recipe
    machine: m1
    recipe: app-scaffold
    inputs:
      name: "{{params.app_name}}"
      port: "{{params.listen_port}}"

  app-config:
    type: recipe
    machine: m1
    recipe: app-config
    inputs:
      name: "{{params.app_name}}"
    depends_on: [app-scaffold]
```

### Nested Recipes

Recipes can include other recipes up to 8 levels deep. Cycle detection prevents infinite recursion:

```yaml
# recipes/full-stack.yaml — includes web-server which includes logrotate
recipe:
  name: full-stack
  inputs:
    domain: { type: string }

resources:
  web:
    type: recipe
    recipe: web-server
    inputs:
      domain: "{{inputs.domain}}"
```

### Recipe Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `recipe` | string | required | Recipe name (resolves to `recipes/{name}.yaml`) |
| `inputs` | map | `{}` | Key-value inputs forwarded to the child recipe |
| `machine` | string | — | Target machine (propagated to all child resources) |

## Common Patterns

### Template Resolution in Resources

All string fields in resources support `{{params.X}}` and `{{secrets.X}}` templates. This enables environment-specific configs from a single YAML source:

```yaml
params:
  env: production
  app_port: "8080"

resources:
  app-config:
    type: file
    machine: web
    path: /etc/app/config.yaml
    content: |
      environment: {{params.env}}
      port: {{params.app_port}}
      database_url: postgresql://app:{{secrets.db_pass}}@{{machine.db.addr}}:5432/app

  app-firewall:
    type: network
    machine: web
    port: "{{params.app_port}}"
    action: allow
    protocol: tcp
```

Templates are resolved just before codegen — the planner sees the resolved values, so changing a parameter value changes the BLAKE3 hash and triggers an update.

### Multi-Machine Resources

A single resource can target multiple machines using array syntax:

```yaml
resources:
  base-packages:
    type: package
    machine: [web, db, monitor]
    provider: apt
    packages: [curl, htop, jq]
```

This creates one logical resource that applies to all three machines. The executor runs it once per target machine. Each machine gets its own lock entry.

### Dependency Chains

Use `depends_on` to enforce ordering across resources:

```yaml
resources:
  create-dirs:
    type: file
    machine: web
    state: directory
    path: /opt/app
    mode: "0755"

  deploy-binary:
    type: file
    machine: web
    path: /opt/app/server
    source: builds/server
    mode: "0755"
    depends_on: [create-dirs]

  app-service:
    type: service
    machine: web
    name: app
    state: running
    enabled: true
    restart_on: [deploy-binary]
    depends_on: [deploy-binary]
```

The DAG ensures: `create-dirs` → `deploy-binary` → `app-service`. Forjar detects cycles at plan time and reports the participants.

### Architecture-Filtered Resources

Target specific CPU architectures to handle heterogeneous fleets:

```yaml
resources:
  arm-packages:
    type: package
    machine: edge-cluster
    provider: apt
    packages: [libraspberrypi-bin]
    arch: [aarch64]

  x86-packages:
    type: package
    machine: edge-cluster
    provider: apt
    packages: [intel-gpu-tools]
    arch: [x86_64]

  universal-packages:
    type: package
    machine: edge-cluster
    provider: apt
    packages: [curl, jq]
    # No arch filter → applies on all architectures
```

Valid architectures: `x86_64`, `aarch64`, `armv7l`, `riscv64`, `s390x`, `ppc64le`.

### Tagged Resources

Tags enable selective apply and filtering:

```yaml
resources:
  nginx:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
    tags: [web, critical]

  monitoring-agent:
    type: package
    machine: web
    provider: apt
    packages: [datadog-agent]
    tags: [monitoring]
```

```bash
# Apply only web-tagged resources
forjar apply -f forjar.yaml --tag web

# Check only monitoring resources
forjar check -f forjar.yaml --tag monitoring
```

### Conditional Resources

The `when:` field makes a resource conditional. Resources with a false `when:` expression are excluded from the plan and execution entirely.

```yaml
resources:
  # Only on x86_64 machines
  cuda-driver:
    type: package
    machine: gpu-server
    provider: apt
    packages: [nvidia-driver-535]
    when: '{{machine.arch}} == "x86_64"'

  # Only in non-production environments
  debug-tools:
    type: package
    machine: web
    provider: apt
    packages: [strace, ltrace]
    when: '{{params.env}} != "production"'

  # Only on machines with the gpu role
  gpu-config:
    type: file
    machine: gpu-server
    path: /etc/gpu.conf
    content: "gpu=enabled"
    when: '{{machine.roles}} contains "gpu"'

  # Feature flag — disabled until ready
  new-feature:
    type: file
    machine: web
    path: /etc/feature.conf
    content: "v2=true"
    when: "false"
```

Supported operators: `==`, `!=`, `contains`. Template variables: `{{machine.arch}}`, `{{machine.hostname}}`, `{{machine.addr}}`, `{{machine.user}}`, `{{machine.roles}}`, `{{params.*}}`.

**Validating conditional resources:**

```bash
# Check when-field syntax (FJ-1018)
forjar validate --check-resource-when-condition-syntax -f forjar.yaml

# Visualize conditional vs unconditional subgraphs (FJ-1019)
forjar graph --resource-dependency-conditional-subgraph -f forjar.yaml
```

**GPU backend consistency:**

When multiple GPU resources exist in a stack, validate that they all reference the same backend:

```bash
# Check GPU backend consistency across resources (FJ-1014)
forjar validate --check-resource-gpu-backend-consistency -f forjar.yaml
```

**Security posture and profiling:**

```bash
# Fleet security posture score (FJ-1017)
forjar status --fleet-resource-security-posture-score

# Apply latency p95 per machine (FJ-1013)
forjar status --machine-resource-apply-latency-p95

# Bridge criticality in dependency graph (FJ-1015)
forjar graph --resource-dependency-bridge-criticality -f forjar.yaml
```

### Resource Iteration

Use `count:` or `for_each:` to create multiple resources from a single declaration.

**count:** creates N copies with `{{index}}` (0-based):

```yaml
resources:
  data-shard:
    type: file
    machine: m1
    state: directory
    path: "/data/shard-{{index}}"
    mode: "0755"
    count: 3
    # Expands to: data-shard-0, data-shard-1, data-shard-2
    # Paths: /data/shard-0, /data/shard-1, /data/shard-2
```

**for_each:** creates per-item copies with `{{item}}`:

```yaml
resources:
  user-home:
    type: file
    machine: m1
    state: directory
    path: "/home/{{item}}"
    owner: "{{item}}"
    mode: "0750"
    for_each: [alice, bob, charlie]
    # Expands to: user-home-alice, user-home-bob, user-home-charlie
```

Template variables `{{index}}` and `{{item}}` are replaced in: `path`, `content`, `name`, `owner`, `source`, `target`, `port`, and `packages`.

Dependencies referencing an expanded resource are rewritten to point at the last expanded copy. For example, `depends_on: [shards]` becomes `depends_on: [shards-2]` when shards has `count: 3`.

A resource cannot have both `count:` and `for_each:`. Validation rejects `count: 0` and empty `for_each: []`.

### Resource State Lifecycle

Every resource goes through a defined lifecycle during apply:

```
Plan Phase:
  desired hash = blake3(all resource fields)
  lock hash    = previous apply hash (or missing)
  action       = Create | Update | NoOp | Destroy

Apply Phase:
  1. Generate check script  → verify preconditions
  2. Generate apply script  → converge to desired state
  3. Execute via transport   → local/SSH/container
  4. Record in lock file     → blake3 hash + status
  5. Append to event log     → provenance trail

Status Values:
  Converged  — resource matches desired state
  Failed     — apply script returned non-zero exit
  Drifted    — live state differs from lock (detected by drift check)
  Unknown    — no lock entry exists
```

## Resource Script Anatomy

Every resource handler in forjar produces exactly three shell scripts. Understanding this three-script pattern is essential for debugging, auditing, and extending forjar.

### The Three-Script Pattern

| Script | Purpose | Exit Behavior |
|--------|---------|---------------|
| **check** | Read current state, report whether resource exists | Outputs `exists:<id>` or `missing:<id>` |
| **apply** | Converge the resource to its desired state | Exits 0 on success, non-zero on failure |
| **state_query** | Capture observable state for BLAKE3 hashing | Outputs key=value pairs or `MISSING` |

The executor runs them in order: check first (to determine if apply is needed), then apply (to converge), then state_query (to record the post-apply state in the lock file).

### Concrete Example: File Resource

Consider this file resource declaration:

```yaml
resources:
  app-config:
    type: file
    machine: web
    path: /etc/app/config.yaml
    content: |
      database:
        host: db.internal
        port: 5432
    owner: app
    group: app
    mode: "0640"
```

Forjar generates the following three scripts:

**check script** -- determines whether the file already exists:

```bash
test -f '/etc/app/config.yaml' && echo 'exists:file' || echo 'missing:file'
```

**apply script** -- converges the file to its desired state:

```bash
set -euo pipefail
mkdir -p '/etc/app'
cat > '/etc/app/config.yaml' <<'FORJAR_EOF'
database:
  host: db.internal
  port: 5432
FORJAR_EOF
chown 'app:app' '/etc/app/config.yaml'
chmod '0640' '/etc/app/config.yaml'
```

Key details: `set -euo pipefail` ensures any failure aborts the script immediately. The heredoc uses hard-quoting (`<<'FORJAR_EOF'`) to prevent shell variable expansion in the content. Parent directories are created with `mkdir -p` before writing. Ownership and permissions are applied after the write.

**state_query script** -- captures the live file state for drift detection:

```bash
if [ -e '/etc/app/config.yaml' ]; then
  stat -c 'owner=%U group=%G mode=%a size=%s' '/etc/app/config.yaml' 2>/dev/null || \
  stat -f 'owner=%Su group=%Sg mode=%Lp size=%z' '/etc/app/config.yaml' 2>/dev/null
  if [ -f '/etc/app/config.yaml' ]; then
    cat '/etc/app/config.yaml' | blake3sum 2>/dev/null || sha256sum '/etc/app/config.yaml' | cut -d' ' -f1
  fi
else
  echo 'MISSING'
fi
```

The state query uses Linux `stat -c` format with a macOS `stat -f` fallback. Content is hashed with BLAKE3 (preferred) or SHA-256 (fallback). The output of this script is itself BLAKE3-hashed and stored in the lock file for future drift comparison.

### Pattern Across Resource Types

Every resource handler follows the same structure. Here is how the three-script pattern maps across types:

| Type | check | apply | state_query |
|------|-------|-------|-------------|
| **file** | `test -f` / `test -d` / `test -L` | heredoc write, `mkdir -p`, `chown`, `chmod` | `stat` + content hash |
| **package** | `dpkg -l` / `command -v` / `uv tool list` | `apt-get install` / `cargo install` / `uv tool install` | `dpkg-query -W` / version check |
| **service** | `systemctl is-active` + `is-enabled` | `systemctl start/stop/enable/disable` | `systemctl show` properties |
| **mount** | `mountpoint -q` | `mount -t` + fstab entry | `findmnt -n` |
| **user** | `id <user>` | `useradd` / `usermod` / `userdel` | `id` + `getent passwd` |
| **docker** | `docker inspect` | `docker pull` + `docker run -d` | `docker inspect` |
| **cron** | `crontab -l` + `grep forjar:<name>` | crontab filter + append | `crontab -l` + `grep -A1` |
| **network** | `ufw status numbered` + grep | `ufw allow/deny/reject` | `ufw status verbose` |

## bashrs Lint Compliance

Forjar integrates with [bashrs](https://crates.io/crates/bashrs) for shell script validation and purification. The bashrs pipeline provides three levels of safety:

1. **`validate_script()`** -- lint-based validation that fails only on Error-severity diagnostics (warnings pass)
2. **`lint_script()`** -- full linter pass returning all diagnostics including warnings
3. **`purify_script()`** -- parse to AST, purify (injection prevention, proper quoting, determinism), reformat

### Clean Handlers

The following resource handlers produce scripts that pass bashrs lint with zero diagnostics:

| Handler | Why Clean |
|---------|-----------|
| **file** | Uses only POSIX builtins (`test`, `mkdir`, `cat`, `chown`, `chmod`, `stat`). No variable expansion in user content (hard-quoted heredoc `<<'FORJAR_EOF'`). No sudo pattern needed. |
| **service** | Uses `systemctl` commands with single-quoted arguments. The systemd guard (`command -v systemctl`) is clean POSIX. Conditional logic uses `if ! systemctl is-active --quiet`. |
| **mount** | Uses `mountpoint`, `mount`, `umount`, `findmnt`, `grep`, `sed`. All arguments are single-quoted. No dynamic variable patterns. |

These handlers pass both `validate_script()` (zero errors) and full `lint_script()` (zero or near-zero diagnostics). Their generated scripts can also be round-tripped through `purify_script()` (parse, purify AST, reformat) without semantic changes.

### Handlers with Known Lint Patterns

The remaining handlers use the `$SUDO` auto-detection pattern, which produces bashrs lint warnings (not errors). The pattern looks like this:

```bash
SUDO=""
[ "$(id -u)" -ne 0 ] && SUDO="sudo"
$SUDO apt-get install -y 'curl'
```

The `$SUDO` variable is intentionally unquoted when used as a command prefix. When the user is root, `$SUDO` expands to an empty string and the command runs directly. When non-root, it expands to `sudo`. bashrs flags the unquoted `$SUDO` usage as a warning (similar to ShellCheck SC2086), but forjar's `validate_script()` passes these scripts because it filters on Error severity only -- warnings are acceptable in generated scripts.

| Handler | Lint Pattern | Reason |
|---------|-------------|--------|
| **package** | `$SUDO` in apt install/remove | Non-root users need sudo for apt-get. The `SUDO` variable is set conditionally based on `id -u`. |
| **user** | `$SUDO` in useradd/usermod/userdel/groupadd | User management commands require root privileges. SSH key deployment also uses `$SUDO mkdir`, `$SUDO mv`, `$SUDO chmod`, `$SUDO chown`. |
| **cron** | `$SUDO` in crontab read/write | Editing another user's crontab (`crontab -u <user>`) requires root. Both present and absent states pipe through `$SUDO crontab -u <user> -`. |
| **network** | `$SUDO` in ufw enable/allow/deny/delete | All ufw operations require root. The handler also runs `$SUDO ufw --force enable` to ensure the firewall is active before adding rules. |

### Validation vs. Purification

For generated scripts, forjar uses `validate_script()` (lint-only, errors block) rather than `purify_script()` (full AST round-trip). The reason is that purification reformats the script, which can change whitespace and ordering in ways that affect the BLAKE3 content hash used for drift detection. Validation provides the safety guarantee (no shell injection, no syntax errors) without altering the script bytes.

The purifier is available for user-facing script auditing (`forjar plan --output-dir`) where deterministic formatting is desirable.

## Script Generation

Forjar generates three types of shell scripts for each resource. Understanding these scripts helps with debugging and auditing.

### Check Scripts

Check scripts verify preconditions before applying. They exit 0 if the resource already exists in the desired state:

```bash
# Package check (apt)
dpkg-query -W -f='${Status}\n' curl 2>/dev/null | grep -q '^install ok installed$'

# File check
test -f '/etc/app/config.yaml' && \
  echo "$(cat '/etc/app/config.yaml' | b3sum --no-names)" | \
  grep -q 'expected_hash'

# Service check
systemctl is-active --quiet nginx

# Mount check
mountpoint -q /mnt/data
```

### Apply Scripts

Apply scripts converge the resource to its desired state. They are idempotent — running them multiple times produces the same result:

```bash
# Package apply (apt, non-root)
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y 'curl' 'git' 'htop'

# File apply (content via heredoc)
mkdir -p "$(dirname '/etc/app/config.yaml')"
cat <<'FORJAR_EOF' > '/etc/app/config.yaml'
database:
  host: localhost
  port: 5432
FORJAR_EOF
chown 'app:app' '/etc/app/config.yaml'
chmod '0640' '/etc/app/config.yaml'

# Service apply
systemctl start nginx
systemctl enable nginx

# Mount apply
mkdir -p '/mnt/data'
mount -t ext4 -o 'defaults,noatime' '/dev/sdb1' '/mnt/data'
```

### State Query Scripts

State query scripts capture the current live state for drift detection. Their output is BLAKE3-hashed:

```bash
# Package state query
dpkg-query -W -f='${Package}=${Version}\n' curl git htop 2>/dev/null || echo 'MISSING'

# Service state query
systemctl show 'nginx' --property=ActiveState,SubState,UnitFileState 2>/dev/null || echo 'MISSING'

# User state query
id 'deploy' >/dev/null 2>&1 && {
  echo "user=deploy"
  echo "uid=$(id -u 'deploy')"
  echo "shell=$(getent passwd 'deploy' | cut -d: -f7)"
} || echo 'user=MISSING'

# Cron state query
crontab -l -u root 2>/dev/null | grep 'forjar:nightly-backup' || echo 'MISSING'
```

### Script Security

All generated scripts follow these security principles:

| Principle | Implementation |
|-----------|---------------|
| **No shell injection** | All user values are single-quoted in generated scripts |
| **No variable expansion** | File content uses heredocs with `<<'FORJAR_EOF'` (hard-quoted) |
| **Sudo auto-detection** | `$SUDO` prefix is set to `sudo` when user != root, empty otherwise |
| **Idempotent operations** | `install -y` (apt), `mkdir -p`, `mount` checks `mountpoint` first |
| **Binary-safe transfers** | `source` files are base64-encoded locally and decoded remotely |

## Resource Ordering Guarantees

### DAG Construction

Resources are ordered using a Directed Acyclic Graph (DAG) built from `depends_on` declarations. The algorithm is Kahn's topological sort with alphabetical tie-breaking for determinism:

```
Input: Resources {A, B, C, D}
  B depends_on: [A]
  C depends_on: [A]
  D depends_on: [B, C]

DAG edges: A → B, A → C, B → D, C → D

Topological sort: [A, B, C, D]
  (B before C due to alphabetical tie-breaking)
```

### Execution Within a Machine

All resources targeting the same machine are executed sequentially in topological order. This ensures:

1. Dependencies are satisfied before dependents run
2. Error propagation stops dependent resources (with `stop_on_first` policy)
3. Side effects from one resource are visible to the next

### Cross-Machine Independence

Resources on different machines are independent — machine A's resources don't wait for machine B. This enables future parallel machine execution (planned for `parallel_machines: true` policy).

## Resource Type Details

### Package Provider Comparison

| Feature | `apt` | `cargo` | `uv` |
|---------|-------|---------|------|
| **Sudo** | Auto-added for non-root | Not used | Not used |
| **Version pin** | `package=version` | `package@version` | `package==version` |
| **Remove** | `apt-get remove -y` | Not supported | `uv tool uninstall` |
| **State query** | `dpkg-query -W` | `cargo install --list` | `uv tool list` |
| **Env variable** | `DEBIAN_FRONTEND=noninteractive` | None | None |

### File Encoding Pipeline

When using `source` (file transfer), the encoding pipeline is:

```
Local machine:
  1. Read source file as bytes
  2. Base64-encode the bytes
  3. Generate script: echo '<base64>' | base64 -d > '<path>'

Remote machine (via transport):
  4. Receive script via stdin pipe
  5. base64 -d decodes the content
  6. Write to target path
  7. Apply owner/group/mode
```

This handles binary files (executables, images, compressed archives) safely through any transport.

### Service Restart Semantics

The `restart_on` field creates a conditional restart trigger:

```yaml
  nginx:
    type: service
    state: running
    restart_on: [nginx-conf, ssl-cert]
    depends_on: [nginx-conf, ssl-cert]
```

During apply:
1. If `nginx-conf` or `ssl-cert` was **actually changed** (action = Create or Update), nginx is restarted
2. If both are **unchanged** (action = NoOp), nginx is not restarted
3. The restart happens after the service's own apply script (start/enable)

This prevents unnecessary service restarts when only re-running `forjar apply` without config changes.

### Docker Container Lifecycle

Docker resources follow a replace-on-change strategy:

```
On apply (state: running):
  1. docker pull <image>            # Always pull latest
  2. docker stop <name> 2>/dev/null  # Stop if exists
  3. docker rm <name> 2>/dev/null    # Remove if exists
  4. docker run -d --name <name> \   # Start fresh
       --restart <policy> \
       -p <ports> -v <volumes> \
       -e <env> <image> [command]

On apply (state: stopped):
  1. docker stop <name>

On apply (state: absent):
  1. docker stop <name> 2>/dev/null
  2. docker rm <name>
```

This ensures containers always run the latest image version with the correct configuration.

### Network Rule Idempotency

UFW rules are managed idempotently using rule comments:

```bash
# Apply: add rule only if not already present
$SUDO ufw allow from '10.0.0.0/8' to any port '22' proto 'tcp' comment 'ssh-access'

# Absent: delete by matching the rule specification
$SUDO ufw delete allow from '10.0.0.0/8' to any port '22' proto 'tcp'
```

UFW deduplicates rules automatically — adding the same rule twice is a no-op.

### Cron Job Tagging

Cron jobs are tagged with comments for idempotent updates:

```crontab
# forjar:nightly-backup
0 2 * * * /usr/local/bin/backup.sh
```

When updating a cron job, forjar:
1. Reads the existing crontab
2. Removes lines between `# forjar:{name}` markers
3. Appends the updated entry
4. Writes the new crontab atomically

This prevents duplicate entries and enables clean removal with `state: absent`.

### Pepita Isolation Lifecycle

Pepita resources follow a create-or-teardown strategy using kernel primitives:

```
On apply (state: present):
  1. mkdir -p <chroot_dir>                    # Create chroot
  2. mkdir -p /sys/fs/cgroup/forjar-<name>    # Create cgroup
  3. echo <limit> > cgroup/memory.max         # Set memory limit
  4. echo <cpus> > cgroup/cpuset.cpus         # Bind CPUs
  5. mount -t overlay overlay <merged>        # Mount overlayfs
  6. ip netns add forjar-<name>               # Create network namespace
  7. ip netns exec forjar-<name> ip link set lo up

On apply (state: absent):
  1. umount <merged>                          # Unmount overlay
  2. ip netns del forjar-<name>               # Delete namespace
  3. rmdir /sys/fs/cgroup/forjar-<name>       # Remove cgroup
  4. rm -rf <chroot_dir>                      # Remove chroot
```

All teardown steps use `|| true` to tolerate already-removed resources. This is distinct from Docker container management (FJ-030) — pepita uses kernel interfaces directly without a container runtime.

## WASM Bundle

Deploy WASM applications (e.g., presentar apps) with size budget tracking.

```yaml
resources:
  app-bundle:
    type: wasm_bundle
    machine: deploy-host
    path: /var/www/app/bundle.wasm
    content: "build output path"
```

WASM bundles support size budget checks via `WasmSizeBudget` and drift detection via `BundleSizeDrift`.

## Image

Build and deploy OCI container images.

```yaml
resources:
  app-image:
    type: image
    machine: build-host
    path: /var/lib/forjar/images/app
    content: "Dockerfile or build context"
```

Image resources go through the layer build pipeline, converting file and package resources into OCI layers via `LayerStrategy::from_resource()`.

## Build

Cross-compile on one machine and deploy the artifact to another.

```yaml
resources:
  apr-binary:
    type: build
    machine: jetson            # deploy target
    build_machine: intel       # where compilation runs
    command: "cargo build --release --target aarch64-unknown-linux-gnu -p apr-cli"
    working_dir: ~/src/aprender
    source: /tmp/cross/release/apr    # artifact path on build machine
    target: ~/.cargo/bin/apr          # deploy path on target machine
    completion_check: "apr --version"
```

### Execution Phases

1. **Build**: SSH to `build_machine`, run `command` in `working_dir`
2. **Transfer**: SCP artifact from `build_machine:source` to `target`
3. **Verify**: Run `completion_check` locally on deploy machine

When `build_machine` is `localhost`, local `cp` replaces SSH/SCP.

### Build Fields

| Field | Required | Description |
|-------|----------|-------------|
| `build_machine` | Yes | Machine that performs the build |
| `command` | Yes | Build command to execute |
| `working_dir` | No | Directory to cd into before building |
| `source` | Yes | Artifact path on build machine |
| `target` | Yes | Deploy path on target machine |
| `completion_check` | No | Command to verify deployment |

### Cargo Binary Cache

Package resources using the `cargo` provider benefit from automatic binary caching. Compiled binaries are stored at `~/.forjar/cache/cargo/<pkg>-<version>-<arch>/bin/` and reused on subsequent installs, skipping recompilation.

Control caching with environment variables:
- `FORJAR_CACHE_DIR`: Override cache directory
- `FORJAR_NO_CARGO_CACHE=1`: Disable caching
