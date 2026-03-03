# Cookbook

Real-world configuration examples.

## Home Lab GPU Server

```yaml
version: "1.0"
name: home-lab
description: "Sovereign AI development environment"

params:
  data_dir: /mnt/nvme-raid0/data
  user: noah

machines:
  gpu-box:
    hostname: lambda
    addr: 192.168.50.100
    user: noah
    ssh_key: ~/.ssh/id_ed25519

resources:
  # Development tools
  dev-packages:
    type: package
    machine: gpu-box
    provider: apt
    packages:
      - build-essential
      - cmake
      - curl
      - git
      - htop
      - jq
      - ripgrep
      - tmux
      - vim

  # Data directory
  data-dir:
    type: file
    machine: gpu-box
    state: directory
    path: "{{params.data_dir}}"
    owner: "{{params.user}}"
    mode: "0755"
    depends_on: [dev-packages]

  # Git config
  gitconfig:
    type: file
    machine: gpu-box
    path: "/home/{{params.user}}/.gitconfig"
    content: |
      [user]
        name = Noah Gift
        email = noah@example.com
      [core]
        editor = vim
      [pull]
        rebase = true
    owner: "{{params.user}}"
    mode: "0644"

policy:
  failure: stop_on_first
  tripwire: true
```

## Multi-Machine Web Stack

```yaml
version: "1.0"
name: web-stack
description: "Web application with load balancer"

params:
  app_version: "2.1.0"
  domain: example.com

machines:
  lb:
    hostname: lb1
    addr: 10.0.0.10
    user: deploy
  web1:
    hostname: web1
    addr: 10.0.0.11
    user: deploy
  web2:
    hostname: web2
    addr: 10.0.0.12
    user: deploy

resources:
  # Install nginx on all web servers
  nginx-pkg:
    type: package
    machine: [web1, web2]
    provider: apt
    packages: [nginx]

  # App config (templated)
  app-config:
    type: file
    machine: [web1, web2]
    path: /etc/app/config.yaml
    content: |
      version: {{params.app_version}}
      domain: {{params.domain}}
      listen: 0.0.0.0:8080
    owner: deploy
    mode: "0640"
    depends_on: [nginx-pkg]

  # Nginx service
  nginx-svc:
    type: service
    machine: [web1, web2]
    name: nginx
    state: running
    enabled: true
    restart_on: [app-config]
    depends_on: [app-config]

  # HAProxy on load balancer
  haproxy:
    type: package
    machine: lb
    provider: apt
    packages: [haproxy]
```

## Edge Device Fleet

```yaml
version: "1.0"
name: edge-fleet
description: "Jetson Orin fleet provisioning"

params:
  model_version: "v3.2"

machines:
  jetson-1:
    hostname: jetson-edge-1
    addr: 192.168.55.1
    user: nvidia
    arch: aarch64
  jetson-2:
    hostname: jetson-edge-2
    addr: 192.168.55.2
    user: nvidia
    arch: aarch64

resources:
  base:
    type: package
    machine: [jetson-1, jetson-2]
    provider: apt
    packages: [curl, htop, python3-pip]

  model-dir:
    type: file
    machine: [jetson-1, jetson-2]
    state: directory
    path: /opt/models
    owner: nvidia
    mode: "0755"
    depends_on: [base]

  inference-config:
    type: file
    machine: [jetson-1, jetson-2]
    path: /opt/models/config.yaml
    content: |
      model_version: {{params.model_version}}
      device: cuda
      batch_size: 1
    owner: nvidia
    mode: "0644"
    depends_on: [model-dir]
```

## Removing Old Resources

Use `state: absent` to clean up:

```yaml
resources:
  old-config:
    type: file
    machine: web1
    state: absent
    path: /etc/old-app/config.yaml

  old-mount:
    type: mount
    machine: web1
    state: absent
    path: /mnt/old-nfs
```

## NFS Data Mount

```yaml
resources:
  nfs-data:
    type: mount
    machine: gpu-box
    source: "192.168.1.10:/exports/data"
    path: /mnt/shared
    fstype: nfs
    options: "rw,soft,intr,timeo=30"
    state: mounted
```

## User Management

```yaml
resources:
  deploy-user:
    type: user
    machine: web1
    name: deploy
    shell: /bin/bash
    home: /home/deploy
    groups: [docker, sudo]
    ssh_authorized_keys:
      - "ssh-ed25519 AAAA... deploy@workstation"

  prometheus-user:
    type: user
    machine: web1
    name: prometheus
    system_user: true
    shell: /usr/sbin/nologin
```

## Docker Containers

```yaml
resources:
  postgres:
    type: docker
    machine: web1
    name: postgres
    image: postgres:16
    state: running
    ports: ["5432:5432"]
    volumes: ["/data/pg:/var/lib/postgresql/data"]
    environment: ["POSTGRES_PASSWORD=secret"]
    restart: unless-stopped

  redis:
    type: docker
    machine: web1
    name: redis
    image: redis:7-alpine
    state: running
    ports: ["6379:6379"]
    restart: unless-stopped
    depends_on: [postgres]
```

## Cron Jobs

```yaml
resources:
  db-backup:
    type: cron
    machine: web1
    name: db-backup
    schedule: "0 2 * * *"
    command: /opt/scripts/backup-db.sh
    owner: postgres
    depends_on: [postgres]

  log-rotate:
    type: cron
    machine: web1
    name: log-rotate
    schedule: "0 0 * * 0"
    command: /usr/sbin/logrotate /etc/logrotate.conf
```

## Firewall Rules

```yaml
resources:
  allow-ssh:
    type: network
    machine: web1
    name: ssh-access
    port: "22"
    protocol: tcp
    action: allow
    from: 10.0.0.0/8

  allow-http:
    type: network
    machine: web1
    name: http-access
    port: "80"
    protocol: tcp
    action: allow

  allow-https:
    type: network
    machine: web1
    name: https-access
    port: "443"
    protocol: tcp
    action: allow

  deny-mysql:
    type: network
    machine: web1
    name: block-mysql
    port: "3306"
    protocol: tcp
    action: deny
```

## Container Dogfood (Local Development)

Test package, file, and service resources locally using a Docker container — no SSH or root required:

```yaml
version: "1.0"
name: dogfood
description: "Local dogfood via container transport"

machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      name: forjar-dogfood
      ephemeral: false       # Keep alive for drift testing
      privileged: false
      init: true

resources:
  base-packages:
    type: package
    machine: test-box
    provider: apt
    packages: [curl, jq, tree]

  app-config:
    type: file
    machine: test-box
    path: /etc/forjar/config.yaml
    content: |
      version: 1.0
      environment: dogfood
    owner: root
    mode: "0644"
    depends_on: [base-packages]

  motd:
    type: file
    machine: test-box
    path: /etc/motd
    content: "Managed by forjar"
    depends_on: [base-packages]
```

Dogfood workflow:

```bash
# Apply and verify convergence
cargo run -- apply -f dogfood.yaml --state-dir /tmp/dogfood

# Prove idempotency (second apply should show 0 converged)
cargo run -- apply -f dogfood.yaml --state-dir /tmp/dogfood

# Check for drift (should be clean)
cargo run -- drift -f dogfood.yaml --state-dir /tmp/dogfood

# Tamper and detect
docker exec forjar-dogfood bash -c "echo tampered > /etc/motd"
cargo run -- drift -f dogfood.yaml --state-dir /tmp/dogfood
# => DRIFT DETECTED for motd

# Remediate
cargo run -- apply -f dogfood.yaml --state-dir /tmp/dogfood --force
```

## Forjar Score

Every config receives a **Forjar Score** — a multi-dimensional quality grade from A through F. Run `forjar score` to get a breakdown:

```bash
# Score a config (static analysis only)
forjar score --file forjar.yaml

# Score with custom status/idempotency
forjar score --file forjar.yaml --status qualified --idempotency strong

# JSON output for CI pipelines
forjar score --file forjar.yaml --json
```

Example output:

```
Forjar Score: 83 (Grade C)
========================================
  COR Correctness     100/100  20%w  [####################]
  IDM Idempotency     100/100  20%w  [####################]
  PRF Performance      85/100  15%w  [#################...]
  SAF Safety           82/100  15%w  [################....]
  OBS Observability    60/100  10%w  [############........]
  DOC Documentation    90/100  8%w   [##################..]
  RES Resilience       50/100  7%w   [##########..........]
  CMP Composability    35/100  5%w   [#######.............]

  Composite: 83/100
  Grade:     C
```

### Scoring Dimensions

| Code | Dimension | Weight | What It Measures |
|------|-----------|--------|------------------|
| COR | Correctness | 20% | Converges from clean state, all resources pass |
| IDM | Idempotency | 20% | Zero changes on re-apply, stable hashes |
| PRF | Performance | 15% | Within time budget, fast re-apply |
| SAF | Safety | 15% | No 0777, no curl\|bash, explicit modes/owners |
| OBS | Observability | 10% | Outputs, tripwire, notify hooks |
| DOC | Documentation | 8% | Description, naming, comments |
| RES | Resilience | 7% | Failure policy, dependency DAG, lifecycle hooks |
| CMP | Composability | 5% | Params, templates, tags, includes |

### Grade Gates

| Grade | Composite | Min Dimension | Meaning |
|-------|-----------|---------------|---------|
| A | >= 90 | >= 80 | Production-hardened |
| B | >= 75 | >= 60 | Solid, minor gaps |
| C | >= 60 | >= 40 | Functional but rough |
| D | >= 40 | any | Bare minimum |
| F | < 40 | any | Blocked/pending/failing |

Grade A requires *every* dimension >= 80, so you can't game the score by overperforming in easy dimensions.

### Improving Your Score

Common improvements to raise your grade:

- **SAF**: Add explicit `mode` and `owner` to all file resources, pin package versions
- **OBS**: Add `outputs:` section, enable `tripwire: true`, configure `notify:` hooks
- **RES**: Set `failure: continue_independent`, add `depends_on` for DAG coverage, add `pre_apply`/`post_apply` hooks
- **CMP**: Use `params:` for configurable values, add `tags:` and `resource_group:` to resources

### Programmatic Scoring

```rust
use forjar::core::scoring;
use forjar::core::parser;

let yaml = std::fs::read_to_string("forjar.yaml").unwrap();
let config = parser::parse_config(&yaml).unwrap();
let input = scoring::ScoringInput {
    status: "qualified".to_string(),
    idempotency: "strong".to_string(),
    budget_ms: 60000,
    runtime: None, // static-only
};
let result = scoring::compute(&config, &input);
println!("Grade: {} ({})", result.grade, result.composite);
```

See `cargo run --example score_cookbook` for a complete example that scores all cookbook recipes.

## Emergency Rollback

When a bad config change reaches production, rollback to the previous known-good state:

```bash
# 1. Preview what rollback would change (no machines contacted)
forjar rollback -f forjar.yaml --dry-run

# 2. Rollback to the previous config (HEAD~1)
forjar rollback -f forjar.yaml

# 3. Or go further back
forjar rollback -f forjar.yaml -n 3

# 4. Rollback only a specific machine
forjar rollback -f forjar.yaml -m web-server

# 5. Verify drift is clean after rollback
forjar drift -f forjar.yaml --tripwire
```

Rollback reads the previous `forjar.yaml` from git history, compares it to the current config, and re-applies the old version with `--force`.

## CI/CD Integration

Use forjar in CI pipelines for automated infrastructure validation and deployment:

```yaml
# .github/workflows/infra.yml
name: Infrastructure
on:
  push:
    paths: ['forjar.yaml', 'recipes/**']

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Validate config
        run: forjar validate -f forjar.yaml

      - name: Lint config
        run: forjar lint -f forjar.yaml

      - name: Check formatting
        run: forjar fmt -f forjar.yaml --check

      - name: Preview plan
        run: forjar plan -f forjar.yaml --json

  deploy:
    needs: validate
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - name: Apply
        run: forjar apply -f forjar.yaml --auto-commit

      - name: Verify no drift
        run: forjar drift -f forjar.yaml --tripwire
```

### Scheduled Drift Detection

```yaml
# .github/workflows/drift.yml
name: Drift Watch
on:
  schedule:
    - cron: '0 */6 * * *'  # every 6 hours

jobs:
  drift:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check for drift
        run: forjar drift -f forjar.yaml --tripwire --json

      - name: Anomaly report
        run: forjar anomaly --min-events 1 --json
```

## Anomaly-Driven Maintenance

Use anomaly detection to identify resources that need attention:

```bash
# Find resources with unusual behavior patterns
forjar anomaly --min-events 3

# Lower threshold for newer deployments
forjar anomaly --min-events 1

# JSON output for alerting systems
forjar anomaly --json | jq '.findings[] | .resource + ": " + (.reasons | join(", "))'

# Common patterns and what they mean:
#   "high churn (z=3.9)"     → Resource re-converging too often, check dependencies
#   "high failure rate (80%)" → Resource consistently failing, investigate root cause
#   "2 drift event(s)"       → Someone is making manual changes, enforce via drift --tripwire
```

Combine with scheduled drift checks for proactive maintenance:

```bash
#!/bin/bash
# maintenance.sh — run weekly

# 1. Check for drift
forjar drift -f forjar.yaml --tripwire --json > /tmp/drift-report.json

# 2. Auto-remediate if drift found
DRIFT_COUNT=$(jq '.drift_count' /tmp/drift-report.json)
if [ "$DRIFT_COUNT" -gt 0 ]; then
    echo "Found $DRIFT_COUNT drifted resources, remediating..."
    forjar drift -f forjar.yaml --auto-remediate
fi

# 3. Check for anomalous patterns
forjar anomaly --json > /tmp/anomaly-report.json
ANOMALIES=$(jq '.anomalies' /tmp/anomaly-report.json)
if [ "$ANOMALIES" -gt 0 ]; then
    echo "WARNING: $ANOMALIES anomalous resources found"
    jq '.findings[]' /tmp/anomaly-report.json
fi
```

## Source File Deployment

Deploy local scripts or config files to remote machines using `source` instead of inline `content`:

```yaml
resources:
  deploy-script:
    type: file
    machine: web1
    path: /opt/app/deploy.sh
    source: scripts/deploy.sh     # Local file, transferred at apply time
    owner: deploy
    mode: "0755"

  nginx-config:
    type: file
    machine: [web1, web2]
    path: /etc/nginx/sites-available/app.conf
    source: configs/nginx-app.conf
    owner: root
    mode: "0644"
```

Files ≤ 1MB are base64-encoded locally and decoded on the remote machine. Files > 1MB use copia delta sync (FJ-242) — only changed 4KB blocks are transferred, critical for multi-GB model files. Both modes support binary files and all transports.

## Partial Failure Recovery

When an apply fails partway through, forjar saves partial state. The next apply only re-runs what failed:

```bash
# First apply — nginx-pkg succeeds, site-config fails (typo in content)
forjar apply -f forjar.yaml --state-dir state/
# Output: web1: 1 converged, 0 unchanged, 1 failed

# Check status to see what's in the lock
forjar status --state-dir state/
# nginx-pkg: Converged
# site-config: Failed

# Fix the typo in forjar.yaml, then re-apply
forjar apply -f forjar.yaml --state-dir state/
# Only site-config and its dependents re-run
# nginx-pkg shows as "unchanged" (hash still matches)
```

If you need to force re-apply everything (e.g., after manual machine changes):

```bash
forjar apply -f forjar.yaml --state-dir state/ --force
```

## Lock File Management

Lock files live in `state/{machine}/state.lock.yaml`. Each contains BLAKE3 hashes of every managed resource:

```bash
# View current lock state
cat state/web1/state.lock.yaml

# Compare two machines
diff state/web1/state.lock.yaml state/web2/state.lock.yaml

# Compare state across time (git history)
forjar diff state-backup/ state/
```

If a lock file becomes corrupted (e.g., disk error, interrupted write):

```bash
# Delete the corrupted lock — next apply will re-apply all resources
rm state/web1/state.lock.yaml

# Or selectively force one resource
forjar apply -f forjar.yaml -r nginx-pkg --force --state-dir state/
```

Lock files are designed to be disposable. Deleting them just means the next apply re-converges everything — resources are idempotent, so this is always safe.

## Auditing and Compliance

Every apply generates provenance events. Use them for compliance and debugging:

```bash
# View apply history for a machine
forjar history --state-dir state/ -m web1 -n 20

# Export as JSON for SIEM integration
forjar history --state-dir state/ --json > /tmp/history.json

# Check which resources have unusual patterns
forjar anomaly --state-dir state/ --min-events 3

# Compare two state snapshots
forjar diff state-before/ state-after/ --json
```

Event log format (`state/{machine}/events.jsonl`):

```json
{"ts":"2026-02-25T14:30:00Z","event":"apply_started","machine":"web1","run_id":"r-abc123","forjar_version":"0.1.0"}
{"ts":"2026-02-25T14:30:01Z","event":"resource_converged","machine":"web1","resource":"nginx-pkg","duration_seconds":1.2,"hash":"blake3:..."}
{"ts":"2026-02-25T14:30:02Z","event":"apply_completed","machine":"web1","run_id":"r-abc123","resources_converged":3,"resources_failed":0,"total_seconds":2.1}
```

## Script Auditing Workflow

Before applying to production, audit the generated scripts:

```bash
# Write all scripts to an audit directory
forjar plan -f forjar.yaml --output-dir /tmp/audit/

# Review each script
ls /tmp/audit/
# web1/nginx-pkg.check.sh
# web1/nginx-pkg.apply.sh
# web1/nginx-pkg.state_query.sh
# web1/site-config.check.sh
# ...

# Diff against previous audit
diff -r /tmp/audit-old/ /tmp/audit/

# Run check scripts manually for verification
ssh deploy@web1 bash < /tmp/audit/web1/nginx-pkg.check.sh
```

## Multi-Environment Promotion

Use parameters to manage staging → production promotion:

```yaml
version: "1.0"
name: my-app
params:
  env: staging
  replicas: "1"
  log_level: debug

machines:
  app:
    hostname: app
    addr: 10.0.0.1

resources:
  app-config:
    type: file
    machine: app
    path: /etc/myapp/config.yaml
    content: |
      environment: {{params.env}}
      replicas: {{params.replicas}}
      log_level: {{params.log_level}}
    mode: "0640"
```

```bash
# Deploy to staging (defaults)
forjar apply -f forjar.yaml --state-dir state-staging/

# Promote to production (override params)
forjar apply -f forjar.yaml --state-dir state-prod/ \
  -p env=production -p replicas=3 -p log_level=warn
```

## Cross-Architecture Fleet

Manage mixed x86_64 and aarch64 fleets with architecture-specific resources:

```yaml
version: "1.0"
name: mixed-fleet

machines:
  x86-srv:
    hostname: x86
    addr: 10.0.0.1
    arch: x86_64
  arm-srv:
    hostname: arm
    addr: 10.0.0.2
    arch: aarch64

resources:
  # Applies to ALL machines
  common-tools:
    type: package
    machine: [x86-srv, arm-srv]
    provider: apt
    packages: [curl, htop, jq]

  # Only applies to x86_64 machines
  intel-microcode:
    type: package
    machine: [x86-srv, arm-srv]
    provider: apt
    packages: [intel-microcode]
    arch: [x86_64]
    depends_on: [common-tools]

  # Only applies to aarch64 machines
  arm-firmware:
    type: package
    machine: [x86-srv, arm-srv]
    provider: apt
    packages: [linux-firmware]
    arch: [aarch64]
    depends_on: [common-tools]
```

Verify with plan:

```bash
forjar plan -f fleet.yaml --state-dir state/
# x86-srv: common-tools, intel-microcode (2 resources)
# arm-srv: common-tools, arm-firmware (2 resources)
# intel-microcode is skipped on arm, arm-firmware is skipped on x86
```

## Recipe: Log Rotation

A reusable recipe for setting up logrotate:

```yaml
# recipes/logrotate.yaml
recipe:
  name: logrotate
  inputs:
    app_name: { required: true }
    log_path: { required: true }
    rotate_count: { default: "7" }
    max_size: { default: "100M" }

resources:
  logrotate-conf:
    type: file
    path: /etc/logrotate.d/{{inputs.app_name}}
    content: |
      {{inputs.log_path}} {
        daily
        rotate {{inputs.rotate_count}}
        maxsize {{inputs.max_size}}
        compress
        delaycompress
        missingok
        notifempty
        create 0640 root adm
      }
    mode: "0644"
```

Usage:

```yaml
resources:
  nginx-logs:
    type: recipe
    machine: web
    recipe: logrotate
    inputs:
      app_name: nginx
      log_path: "/var/log/nginx/*.log"
      rotate_count: "14"
      max_size: "500M"

  app-logs:
    type: recipe
    machine: web
    recipe: logrotate
    inputs:
      app_name: myapp
      log_path: "/var/log/myapp/*.log"
```

## Recipe: SSH Hardening

```yaml
# recipes/ssh-hardening.yaml
recipe:
  name: ssh-hardening
  inputs:
    port: { default: "22" }
    allow_groups: { default: "ssh-users" }

resources:
  sshd-config:
    type: file
    path: /etc/ssh/sshd_config.d/99-hardening.conf
    content: |
      Port {{inputs.port}}
      PermitRootLogin no
      PasswordAuthentication no
      PubkeyAuthentication yes
      AllowGroups {{inputs.allow_groups}}
      MaxAuthTries 3
      LoginGraceTime 20
      ClientAliveInterval 300
      ClientAliveCountMax 2
    mode: "0644"

  sshd-restart:
    type: service
    name: sshd
    state: running
    enabled: true
    restart_on: [sshd-config]
    depends_on: [sshd-config]
```

## Task: Build Pipeline

Use `type: task` for multi-stage build pipelines with completion checks and timeouts:

```yaml
resources:
  generate-source:
    type: task
    machine: build-host
    command: |
      mkdir -p src
      cat > src/main.c <<'CSRC'
      #include <stdio.h>
      int main() { printf("OK\n"); return 0; }
      CSRC
    working_dir: /opt/pipeline
    completion_check: "test -f /opt/pipeline/src/main.c"
    output_artifacts:
      - /opt/pipeline/src/main.c

  compile:
    type: task
    machine: build-host
    command: "gcc -o artifacts/app src/main.c -Wall"
    working_dir: /opt/pipeline
    timeout: 120
    completion_check: "test -x /opt/pipeline/artifacts/app"
    output_artifacts:
      - /opt/pipeline/artifacts/app
    depends_on: [generate-source]

  run-tests:
    type: task
    machine: build-host
    command: |
      OUTPUT=$(./artifacts/app)
      [ "$OUTPUT" = "OK" ] && echo PASS > artifacts/result.txt
    working_dir: /opt/pipeline
    timeout: 60
    completion_check: "grep -q PASS /opt/pipeline/artifacts/result.txt 2>/dev/null"
    depends_on: [compile]
```

Key patterns:
- `completion_check` makes tasks idempotent — re-apply skips completed stages
- `timeout` prevents hung builds from blocking the pipeline
- `output_artifacts` tracks what each stage produces
- `depends_on` chains stages in order

## Recipe: Nested Composition

Use `type: recipe` to compose reusable sub-recipes with input forwarding:

```yaml
# forjar.yaml — parent config
params:
  app_name: myapp
  app_port: "8080"

resources:
  app-scaffold:
    type: recipe
    machine: target
    recipe: app-scaffold
    inputs:
      app_name: "{{params.app_name}}"

  app-config:
    type: recipe
    machine: target
    recipe: app-config
    inputs:
      app_name: "{{params.app_name}}"
      port: "{{params.app_port}}"
    depends_on: [app-scaffold]
```

```yaml
# recipes/app-scaffold.yaml — child recipe
recipe:
  name: app-scaffold
  inputs:
    app_name: { type: string }

resources:
  config-dir:
    type: file
    path: "/etc/apps/{{inputs.app_name}}"
    state: directory
    owner: root
    mode: "0755"

  log-dir:
    type: file
    path: "/var/log/apps/{{inputs.app_name}}"
    state: directory
    owner: root
    mode: "0755"
```

After expansion, resources are namespaced: `app-scaffold/config-dir`, `app-scaffold/log-dir`, `app-config/app-conf`, etc. Internal `depends_on` references are rewritten to use the namespaced IDs.

## Pepita: Kernel Sandbox

Use `type: pepita` for bare-metal kernel isolation without Docker:

```yaml
resources:
  build-sandbox:
    type: pepita
    machine: build-host
    name: build-sandbox
    state: present
    chroot_dir: /var/lib/forjar/rootfs/jammy
    overlay_lower: /var/lib/forjar/pepita/lower
    overlay_upper: /var/lib/forjar/pepita/upper
    overlay_merged: /var/lib/forjar/pepita/merged
    memory_limit: 2048
    cpuset: "0-3"
    netns: true
```

This creates:
- A cgroups v2 group at `/sys/fs/cgroup/forjar-build-sandbox` with 2 GiB memory limit
- CPU affinity bound to cores 0-3
- An isolated network namespace `forjar-build-sandbox`
- An overlayfs mount with copy-on-write writes to the upper layer

## Model: ML Artifact Management

Use `type: model` for ML model downloads with BLAKE3 integrity:

```yaml
resources:
  tinyllama:
    type: model
    machine: gpu-box
    name: tinyllama
    source: "TheBloke/TinyLlama-1.1B-GGUF"
    path: /opt/models/tinyllama.gguf
    cache_dir: /var/cache/apr
    state: present

  custom-model:
    type: model
    machine: gpu-box
    name: custom-v1
    source: "https://example.com/models/custom-v1.bin"
    path: /opt/models/custom-v1.bin
    checksum: "abc123..."
    state: present
```

Sources: HuggingFace repo ID (uses `apr pull`), HTTP URL (uses `curl`), or local path (uses `cp`). When `checksum` is set, drift detection verifies BLAKE3 hashes to catch unauthorized model swaps.

## Pattern: Staged Rollout

Apply changes to canary machines first, then the fleet:

```yaml
machines:
  canary:
    hostname: web-01
    addr: 10.0.1.1
    roles: [web, canary]
  web-02:
    hostname: web-02
    addr: 10.0.1.2
    roles: [web]
  web-03:
    hostname: web-03
    addr: 10.0.1.3
    roles: [web]
```

```bash
# Step 1: Apply to canary only
forjar apply -f forjar.yaml --state-dir state/ -m canary

# Step 2: Verify canary
forjar drift -f forjar.yaml --state-dir state/ -m canary --tripwire

# Step 3: Apply to remaining machines
forjar apply -f forjar.yaml --state-dir state/ -m web-02
forjar apply -f forjar.yaml --state-dir state/ -m web-03
```

## Pattern: Config Templating Per Environment

Use params to template environment-specific values:

```yaml
params:
  env: "{{params.env}}"  # Override with -p env=production
  log_level: info

resources:
  app-config:
    type: file
    machine: web
    path: /etc/myapp/config.yaml
    content: |
      environment: {{params.env}}
      log_level: {{params.log_level}}
      database:
        host: db-{{params.env}}.internal
        pool_size: 10
    mode: "0640"
    owner: app
    group: app
```

```bash
# Staging
forjar apply -f forjar.yaml --state-dir state-staging/ -p env=staging -p log_level=debug

# Production
forjar apply -f forjar.yaml --state-dir state-prod/ -p env=production -p log_level=warn
```

## Disaster Recovery

When a machine is lost or rebuilt, forjar recovers by re-applying from the config:

```bash
# 1. Machine is dead — delete its stale lock (optional; apply will overwrite)
rm -rf state/web1/

# 2. Re-provision from scratch (all resources converge fresh)
forjar apply -f forjar.yaml --state-dir state/ -m web1

# 3. Verify clean state after recovery
forjar drift -f forjar.yaml --state-dir state/ -m web1

# 4. Review the event log — confirm all resources converged
forjar history --state-dir state/ -m web1 -n 10
```

For fleet-wide recovery (e.g., after a datacenter migration):

```bash
# Wipe all state (locks are disposable — configs are the source of truth)
rm -rf state/

# Re-apply to all machines
forjar apply -f forjar.yaml --state-dir state/

# Verify entire fleet
forjar drift -f forjar.yaml --state-dir state/ --tripwire
forjar anomaly --state-dir state/ --min-events 1
```

The key insight: `forjar.yaml` in version control is the authoritative source. Lock files are caches that can always be regenerated.

## Secret Management

Forjar supports secrets via environment variables, keeping sensitive values out of config files:

```yaml
params:
  db_host: db.internal
  db_name: myapp

resources:
  db-config:
    type: file
    machine: web
    path: /etc/myapp/database.yml
    content: |
      host: {{params.db_host}}
      name: {{params.db_name}}
      password: {{secrets.DB_PASSWORD}}
    mode: "0600"
    owner: app
```

```bash
# Set secret as environment variable
export FORJAR_SECRET_DB_PASSWORD="s3cur3-p@ssw0rd"

# Apply — secret is interpolated at apply time, never stored in config
forjar apply -f forjar.yaml --state-dir state/
```

For CI/CD, use GitHub Actions secrets:

```yaml
# .github/workflows/deploy.yml
env:
  FORJAR_SECRET_DB_PASSWORD: ${{ secrets.DB_PASSWORD }}
  FORJAR_SECRET_API_KEY: ${{ secrets.API_KEY }}
steps:
  - run: forjar apply -f forjar.yaml --state-dir state/
```

### Age-Encrypted Secrets (FJ-200)

For secrets that should live alongside your config in git, use age encryption:

```bash
# Generate an age keypair
forjar secrets keygen > ~/.forjar/identity.txt
# Output:
# created: 2026-02-26
# public key: age1abc...xyz
# AGE-SECRET-KEY-1...

# Encrypt a secret value
forjar secrets encrypt "s3cur3-p@ssw0rd" -r age1abc...xyz
# Output: ENC[age,YWdlLWVuY3J5cH...]

# Paste the marker directly in your YAML
```

```yaml
resources:
  db-config:
    type: file
    machine: web
    path: /etc/myapp/database.yml
    content: |
      host: {{params.db_host}}
      password: ENC[age,YWdlLWVuY3J5cH...]
    mode: "0600"
```

```bash
# Apply with identity
export FORJAR_AGE_KEY="AGE-SECRET-KEY-1..."
forjar apply -f forjar.yaml

# View decrypted config without applying
forjar secrets view -f forjar.yaml

# Re-encrypt all secrets with a new key
forjar secrets rekey -f forjar.yaml -r age1newkey...
```

Multi-recipient encryption for teams:

```bash
# Encrypt for multiple team members
forjar secrets encrypt "shared-secret" \
  -r age1alice... \
  -r age1bob... \
  -r age1carol...
```

### Secret Rotation (FJ-201)

Rotate all secrets in a config to new recipients:

```bash
# Rotate all secrets to a new key (requires --re-encrypt safety flag)
forjar secrets rotate -f forjar.yaml \
  -r age1newkey... \
  --re-encrypt \
  --state-dir state/
# Output: rotated 5 secret(s) in forjar.yaml to 1 recipient(s)
```

Rotation events are logged to `state/__secrets__/events.jsonl`:

```json
{"ts":"2026-02-26T12:00:00Z","event":"secret_rotated","file":"forjar.yaml","marker_count":5,"new_recipients":["age1newkey..."]}
```

Best practices:
- Use `ENC[age,...]` markers for secrets committed to git
- Use `{{secrets.*}}` env vars for CI/CD secrets (GitHub Actions, etc.)
- Keep identity files out of git (`.gitignore`)
- Use multi-recipient encryption for team access
- Use `forjar secrets rotate --re-encrypt` when rotating keys or adding team members
- Rotation events are logged to events.jsonl for audit compliance
- Use `forjar lint` to detect hardcoded secrets (checks for common patterns)

## Performance Monitoring

Use `forjar bench` for quick inline performance checks:

```bash
# Quick benchmark (default 10 iterations)
forjar bench

# High-precision run (100 iterations)
forjar bench --iterations 100

# JSON output for CI tracking
forjar bench --json | jq '{validate: .results[0].avg_ms, plan: .results[1].avg_ms}'
```

Track performance over time in CI:

```yaml
# .github/workflows/perf.yml
name: Performance Tracking
on:
  push:
    branches: [main]

jobs:
  bench:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run benchmarks
        run: |
          cargo run -- bench --json --iterations 50 > bench.json
          # Assert targets
          VALIDATE=$(jq '.results[0].avg_ms' bench.json)
          if (( $(echo "$VALIDATE > 10" | bc -l) )); then
            echo "FAIL: validate ${VALIDATE}ms > 10ms target"
            exit 1
          fi
```

For Criterion benchmarks (deeper analysis with confidence intervals):

```bash
# Full Criterion suite
cargo bench

# Compare against baseline
cargo bench -- --save-baseline before
# ... make changes ...
cargo bench -- --baseline before
```

## Trace and Provenance Auditing

After any apply, trace spans record exactly what happened and when:

```bash
# View traces grouped by trace_id
forjar trace --state-dir state/

# Filter to a specific machine
forjar trace --state-dir state/ -m web1

# JSON output for SIEM integration
forjar trace --state-dir state/ --json | jq '.traces[].spans[] | {resource, duration_ms}'
```

Combine trace + anomaly for root cause analysis:

```bash
# 1. Anomaly flags "high churn" on nginx-config
forjar anomaly --state-dir state/ --min-events 3

# 2. Trace shows the timeline — config keeps re-converging
forjar trace --state-dir state/ -m web1 --json | \
  jq '.traces[].spans[] | select(.resource == "nginx-config")'

# 3. Check if another resource triggers it via restart_on
forjar show -f forjar.yaml -r nginx-svc
# restart_on: [nginx-config] — every config change restarts nginx
```

## Resource Tagging

Use tags to apply subsets of resources:

```yaml
resources:
  web-config:
    type: file
    machine: web
    path: /etc/nginx/nginx.conf
    content: "..."
    tags: [web, config]

  db-config:
    type: file
    machine: db
    path: /etc/postgresql/pg_hba.conf
    content: "..."
    tags: [database, config]

  monitoring:
    type: package
    machine: [web, db]
    provider: apt
    packages: [prometheus-node-exporter]
    tags: [monitoring]
```

```bash
# Apply only config-tagged resources
forjar apply -f forjar.yaml --state-dir state/ --tags config

# Apply only monitoring
forjar apply -f forjar.yaml --state-dir state/ --tags monitoring

# Plan for database resources only
forjar plan -f forjar.yaml --tags database
```

## Template Functions

Use `{{func(args)}}` for string transformations without shell escapes.

```yaml
version: "1.0"
name: template-funcs-demo

params:
  hostname: "  My-Server  "
  env: production
  tags: "web,api,gpu"

machines:
  web:
    hostname: web-prod
    addr: 10.0.0.1
    user: deploy

resources:
  # Normalize hostname: trim whitespace, then uppercase
  banner:
    type: file
    machine: web
    path: /etc/banner
    content: "{{upper(trim(params.hostname))}}"
    # Result: "MY-SERVER"

  # Environment tag, lowercased
  env-tag:
    type: file
    machine: web
    path: /etc/env.tag
    content: "env={{lower(params.env)}}"
    # Result: "env=production"

  # Replace separator in string
  slug:
    type: file
    machine: web
    path: /etc/slug
    content: "{{replace(params.hostname, \" \", \"-\")}}"

  # Fallback value for optional param
  config:
    type: file
    machine: web
    path: /etc/config
    content: "region={{default(params.region, \"us-east-1\")}}"

  # BLAKE3 content hash
  checksum:
    type: file
    machine: web
    path: /etc/checksum
    content: "{{b3sum(params.tags)}}"

  # Join comma-separated tags with pipe
  tags-file:
    type: file
    machine: web
    path: /etc/tags
    content: "{{join(params.tags, \"|\")}}"
    # Result: "web|api|gpu"

  # Machine ref inside a function
  upper-host:
    type: file
    machine: web
    path: /etc/upper-host
    content: "{{upper(machine.web.hostname)}}"
    # Result: "WEB-PROD"

  # Nested: chain three functions
  normalized:
    type: file
    machine: web
    path: /etc/normalized
    content: "{{upper(replace(lower(params.hostname), \" \", \"_\"))}}"
```

Available functions: `upper`, `lower`, `trim`, `default`, `replace`, `env`, `b3sum`, `join`, `split`. All support nested calls and `params.*`/`machine.*` argument references.

## Sovereign AI Cookbook

The [sovereign-ai-cookbook](https://github.com/paiml/sovereign-ai-cookbook) repo provides complete forjar deployment configs for the PAIML/APR sovereign AI stack. All stacks use docker container targets for testing — swap to SSH for production.

### Available Stacks

| Stack | Components | Description |
|-------|-----------|-------------|
| `01-inference` | realizar | Single-machine GPU model serving |
| `02-training` | entrenar | LoRA fine-tuning pipeline |
| `03-rag` | trueno-db, trueno-rag, realizar | Retrieval-augmented generation |
| `04-speech` | whisper-apr | Automatic speech recognition |
| `05-distributed-inference` | repartir, realizar | Multi-node inference |
| `06-full-stack` | all components | Complete sovereign AI lab |
| `07-data-pipeline` | alimentar, entrenar, realizar | Ingest → train → serve |
| `08-observability` | renacer, Jaeger, Grafana | Monitoring and tracing |

### Quick Start

```bash
git clone https://github.com/paiml/sovereign-ai-cookbook
cd sovereign-ai-cookbook

# Validate all stacks
make validate

# Plan a single stack
forjar plan -f stacks/01-inference/forjar.yaml

# Apply (deploys to docker container)
forjar apply -f stacks/01-inference/forjar.yaml
```

### Architecture

```
┌──────────────────┐
│   monitor-box    │  renacer + Grafana + Jaeger + pacha
└────────┬─────────┘
         │
    ┌────┼────┐
    │    │    │
┌───▼┐ ┌▼──┐ ┌▼────┐
│gpu │ │rag│ │work-│  realizar + entrenar | trueno-db + trueno-rag + whisper-apr | repartir
│box │ │box│ │ er  │
└────┘ └───┘ └─────┘
```

Each stack composes reusable recipes from `recipes/`. Recipes are machine-agnostic — the stack's `forjar.yaml` binds them to specific machines.

### Writing Custom Stacks

Compose recipes in a new `stacks/` directory:

```yaml
version: "1.0"
name: my-custom-stack

machines:
  box:
    hostname: box
    addr: container
    transport: container
    container:
      runtime: docker
      image: forjar-test-target
      ephemeral: true
      init: true

resources:
  serving:
    type: recipe
    machine: box
    recipe: realizar-serve
    inputs:
      model_path: /opt/models/my-model.gguf
      port: 8080
      user: realizar

  registry:
    type: recipe
    machine: box
    recipe: pacha-registry
    inputs:
      port: 8070
      user: pacha
```

For production, change `addr: container` to a real IP and add `ssh_key`:

```yaml
machines:
  box:
    hostname: gpu-prod-01
    addr: 10.0.1.10
    user: deploy
    ssh_key: ~/.ssh/deploy_key
```

## Pre-flight Checks with Doctor

Run `forjar doctor` before apply to verify system prerequisites:

```bash
# Basic system check
forjar doctor

# Check against a specific config (validates SSH/docker/age as needed)
forjar doctor -f forjar.yaml

# JSON output for CI pipelines
forjar doctor -f forjar.yaml --json
```

Example output:

```
[pass] bash: bash 5.1.16
[pass] ssh: OpenSSH_8.9p1
[pass] docker: Docker version 29.1.4
[pass] state-dir: state writable
[warn] git: 3 uncommitted changes

5 checks: 4 pass, 1 warn, 0 fail
```

Checks are context-aware — SSH is only checked if your config has remote machines, Docker only if you have container machines, and age identity only if your config contains `ENC[age,...]` markers.

## Security Scanning and Policy Gates

Forjar includes a static IaC security scanner (`forjar security-scan`) that detects security smells in your configs before deployment.

### Scan a Config

```bash
# Scan for all security issues
forjar security-scan -f infra.yaml

# JSON output for CI integration
forjar security-scan -f infra.yaml --json

# Fail CI on critical/high findings
forjar security-scan -f infra.yaml --fail-on high
```

### Security Rules

| Rule | Category | Severity | Description |
|------|----------|----------|-------------|
| SS-1 | Hard-coded secrets | Critical | Passwords, tokens, API keys in plain text |
| SS-2 | HTTP without TLS | High | Unencrypted HTTP URLs (use HTTPS) |
| SS-3 | World-accessible | High | File permissions allowing world access |
| SS-4 | Missing integrity check | Medium | External files without BLAKE3 verification |
| SS-5 | Privileged container | Critical | Docker running with elevated permissions |
| SS-6 | No resource limits | Low | Containers without CPU/memory bounds |
| SS-7 | Weak cryptography | High | MD5, SHA1, DES, RC4, SSLv3, TLSv1.0 |
| SS-8 | Insecure protocol | High | telnet://, ftp://, rsh:// in configs |
| SS-9 | Unrestricted network | Medium | Binding to 0.0.0.0 (all interfaces) |
| SS-10 | Sensitive data | Critical | PII patterns (SSN, credit card numbers) |

### Pre-Apply Security Gate

Add `security_gate` to your policy to block applies with security findings:

```yaml
version: "1.0"
name: secure-infra

policy:
  security_gate: high  # Block on critical or high findings

resources:
  web-config:
    type: file
    machine: web-server
    target: /etc/nginx/nginx.conf
    source: https://example.com/nginx.conf  # SS-4: needs integrity check
```

With `security_gate: high`, `forjar apply` will refuse to run if any critical or high severity findings exist. Severity thresholds: `critical`, `high`, `medium`, `low`.

## Cookbook Recipe Index

The `examples/cookbook/` directory contains validated recipes covering common infrastructure patterns:

| # | Recipe | Resources | Key Types | Phase |
|---|--------|-----------|-----------|-------|
| 01 | Developer Workstation | 7 | package, user, file | Core |
| 02 | Web Server (Nginx) | 8 | package, file, service, network | Core |
| 03 | PostgreSQL | 8 | package, file, service, cron | Core |
| 04 | Monitoring Stack | 8 | docker, file, network | Core |
| 05 | Redis Cache | 4 | docker, file, network | Core |
| 06 | CI Runner | 9 | package, file, user, docker | GPU/HW |
| 07 | ROCm GPU | 4 | gpu, user, package, cron | GPU/HW |
| 08 | NVIDIA GPU | 4 | gpu, user, package, cron | GPU/HW |
| 09 | Secure Baseline | 7 | package, file, service, network | Core |
| 10 | NFS Server | 5 | package, file, service, mount | GPU/HW |
| 11 | Dev Shell | 4 | package, file | Nix-style |
| 12 | Toolchain Pin | 6 | file, cron | Nix-style |
| 13 | Build Sandbox | 4 | package, file | Nix-style |
| 14 | System Profile | 9 | package, user, file | Nix-style |
| 15 | Multi-Project Workspace | 9 | package, file | Nix-style |
| 16 | Rust Release Build | 4 | package, file | Rust Build |
| 17 | Static Musl Build | 5 | package, file | Rust Build |
| 19 | Cross-Compilation | 7 | package, file | Rust Build |
| 20 | Sovereign Stack Release | 7 | package, file | Rust Build |
| 21 | APR Model Pipeline | 9 | file, cron | Rust Build |
| 22 | Secrets Lifecycle | 7 | file, cron | Ops |
| 23 | TLS Certificates | 7 | file, cron | Ops |
| 24 | Fleet Provisioning | 8 | package, file, cron | Ops |
| 25 | APT Repository | 5 | file | Packages |
| 26 | .deb Package Build | 7 | package, file | Packages |
| 27 | Private APT Repo | 8 | package, file, service, network | Packages |
| 28 | RPM Build | 7 | package, file | Packages |
| 29 | Distribution Pipeline | 8 | package, file | Packages |
| 30 | Saved Plan Files | 5 | file | OpenTofu |
| 31 | JSON Plan Format | 4 | package, file | OpenTofu |
| 32 | Check Blocks | 4 | package, file | OpenTofu |
| 33 | Lifecycle Protection | 5 | file | OpenTofu |
| 34 | Moved Blocks | 3 | package, file | OpenTofu |
| 35 | Refresh-Only Mode | 4 | file | OpenTofu |
| 36 | Resource Targeting | 5 | package, file | OpenTofu |
| 37 | Testing DSL | 4 | package, file | OpenTofu |
| 38 | State Encryption | 6 | file | OpenTofu |
| 39 | Cross-Config Outputs | 4 | file | OpenTofu |
| 40 | Scheduled Tasks | 8 | cron, file | Linux Admin |
| 41 | User Provisioning | 7 | user, file | Linux Admin |
| 42 | Kernel Tuning | 5 | file | Linux Admin |
| 43 | Log Management | 7 | package, file, cron | Linux Admin |
| 44 | Time Sync (Chrony) | 5 | package, file, service | Linux Admin |
| 45 | Systemd Units | 6 | file, cron | Linux Admin |
| 46 | Resource Limits | 4 | file | Linux Admin |
| 47 | Automated Patching | 6 | package, file, cron | Linux Admin |
| 48 | Hostname & Locale | 6 | file | Linux Admin |
| 49 | Swap & Memory | 4 | file, cron | Linux Admin |
| 50 | Failure: Partial Apply | 4 | package, file | Failure Mode |
| 51 | Failure: State Recovery | 5 | file | Failure Mode |
| 52 | Failure: Crash Resilience | 4 | file | Failure Mode |
| 53 | Stack: Dev Server | 7 | package, file, user | Composability |
| 54 | Stack: Web Production | 8 | package, file | Composability |
| 55 | Stack: GPU Lab | 7 | package, file, user | Composability |
| 56 | Stack: Build Farm | 8 | package, file | Composability |
| 57 | Stack: Package Pipeline | 8 | package, file | Composability |
| 58 | Stack: ML Inference | 8 | file | Composability |
| 59 | Stack: CI Infrastructure | 7 | package, file, user | Composability |
| 60 | Stack: Sovereign AI | 11 | package, file, user | Composability |
| 61 | Stack: Fleet Baseline | 7 | package, file | Composability |
| 62 | Stack: Cross-Distro Release | 8 | package, file | Composability |
| 63 | Store: Version-Pinned | 5 | file | Store |
| 64 | Store: Cargo Sandbox | 5 | file | Store |
| 65 | Store: SSH Cache | 4 | file | Store |
| 66 | Store: Repro CI Gate | 5 | file | Store |
| 67 | Store: Profile Rollback | 5 | file | Store |
| 68 | Mount: NFS/Bind/Tmpfs | 7 | mount, file, cron | Resource Type |
| 69 | Pepita: Kernel Sandbox | 8 | pepita, file | Resource Type |
| 70 | Model: ML Download | 7 | model, file | Resource Type |
| 71 | Task: Build Pipeline | 7 | task, file | Resource Type |
| 72 | Recipe: Composition | 6 | recipe, file | Resource Type |

| 73 | pforge MCP Server | 6 | package, file, service, task | Agent Infrastructure |
| 74 | Agent Deployment | 7 | package, file, task | Agent Infrastructure |

Score all recipes programmatically:

```bash
cargo run --example score_cookbook
```

## Agent Infrastructure

Recipes 73-74 deploy LLM agent infrastructure using forjar primitives.

### pforge MCP Server (Recipe 73)

Deploys the pforge MCP server for agent-accessible infrastructure management:

```yaml
data:
  pforge:
    type: forjar-state
    state_dir: ../mcp-server/state
    outputs:
      - pforge_endpoint
    max_staleness: 1h
```

Resources: forjar binary (cargo), config directory, server config YAML, state directory, systemd service, health check task.

### Agent Deployment (Recipe 74)

Composable recipe for deploying an LLM agent with model, GPU, and MCP configuration:

```bash
# Deploy with custom parameters
forjar apply -f 74-agent-deployment.yaml \
  --set agent_name=code-assistant \
  --set model_name=llama-3.1-70b \
  --set gpu_backend=nvidia \
  --set mcp_port=8800
```

Layers: base packages, model cache directory, agent config, MCP tools config, health check. All parameterized for any model/GPU combination.

## Sudo Elevation and SBOM Generation

### Per-Resource Sudo Elevation

Use the `sudo: true` field on any resource to run its apply script with elevated privileges:

```yaml
resources:
  system-packages:
    type: package
    machine: web
    provider: apt
    packages: [nginx, curl, htop]
    sudo: true    # Runs apt-get with sudo when non-root

  nginx-config:
    type: file
    machine: web
    path: /etc/nginx/nginx.conf
    source: configs/nginx.conf
    sudo: true    # Needs sudo for /etc/ writes

  app-config:
    type: file
    machine: web
    path: /home/app/config.yaml
    source: configs/app.yaml
    # No sudo needed — user-writable path
```

When `sudo: true`, forjar wraps the generated script:
- If already root (`id -u == 0`): runs script as-is
- If non-root: wraps with `sudo bash -c '...'`

### SBOM Generation

Generate a Software Bill of Materials for all managed infrastructure:

```bash
# Text table output
forjar sbom -f forjar.yaml

# SPDX 2.3 JSON output (machine-readable)
forjar sbom -f forjar.yaml --json

# With state directory for BLAKE3 hashes
forjar sbom -f forjar.yaml --state-dir state --json > sbom.spdx.json
```

The SBOM includes:
- **Package resources**: Each package with provider and version
- **Docker images**: Image name, tag, and content hash
- **Model artifacts**: Source URL, version, and BLAKE3 checksum
- **File resources with sources**: Downloaded files with state hashes

## Debug Trace Mode

Use `--trace` on apply to print generated scripts before execution:

```bash
forjar apply --trace

# Output includes:
# [TRACE] base-packages script:
# set -euo pipefail
# ...apt-get install...
# [TRACE] nginx-config script:
# set -euo pipefail
# ...base64 -d...
```

Trace mode implies `--verbose` and shows the full bash script that will be sent to each transport (local, SSH, container).

## Cryptographic Bill of Materials (CBOM)

Generate a cryptographic inventory of all algorithms used in your infrastructure:

```bash
# Text table output
forjar cbom

# JSON output
forjar cbom --json
```

CBOM automatically detects:
- **BLAKE3** — State hashing and resource integrity
- **X25519/age** — Secrets encryption
- **Ed25519/RSA** — SSH transport keys
- **X.509/TLS** — Certificate management
- **SHA-256** — Docker image digests

## Convergence Proof

Prove that your configuration will converge from any reachable state:

```bash
# Prove convergence for all resources
forjar prove

# Prove for a specific machine
forjar prove --machine web-01

# JSON output for CI integration
forjar prove --json
```

The convergence proof validates five properties:
1. **Codegen completeness** — All resources produce check/apply/state_query scripts
2. **DAG acyclicity** — No circular dependencies
3. **State coverage** — Resources have corresponding state entries
4. **Hash determinism** — Same config produces identical state_query scripts
5. **Idempotency structure** — Apply scripts use `set -euo pipefail`

## Least-Privilege Analysis

Analyze the minimum permissions required per resource:

```bash
# Text output — shows which resources need root
forjar privilege-analysis

# Filter to a specific machine
forjar privilege-analysis --machine web-01

# JSON output for CI integration
forjar privilege-analysis --json
```

Reports privilege levels: `unprivileged`, `system-write`, `package-manager`, `service-control`, `network-config`, `sudo`.

## SLSA Provenance Attestation

Generate in-toto-style SLSA Level 3 provenance attestations:

```bash
# Generate provenance attestation
forjar provenance

# JSON output (in-toto v0.1 format)
forjar provenance --json

# Scoped to one machine
forjar provenance --machine web-01
```

Links config BLAKE3 hash -> plan hash -> state hashes in a tamper-evident chain.

## Merkle DAG Lineage

Visualize the Merkle tree over your dependency graph:

```bash
# Show Merkle hashes for each resource
forjar lineage

# JSON output with merkle_root
forjar lineage --json
```

Each node's hash incorporates its dependencies' hashes, so any change propagates up the Merkle tree — enabling tamper detection of the full dependency chain.

## Recipe Bundles

Package your config with all dependencies for air-gap transfer:

```bash
# Dry-run: show bundle manifest
forjar bundle

# Include state files
forjar bundle --include-state

# Write to file
forjar bundle --output my-stack.tar
```

Each file gets a BLAKE3 hash for integrity verification during transfer.

## Model Card Generation

Generate model cards documenting ML resources in your stack:

```bash
# Text output
forjar model-card

# JSON output
forjar model-card --json
```

Detects model resources by type, tags (`ml`, `model`), or resource group (`models`).

## Agent SBOM

Generate an agent-specific bill of materials:

```bash
# Text output
forjar agent-sbom

# JSON output
forjar agent-sbom --json
```

Detects: model resources, GPU runtimes, MCP/pforge-tagged services, agent containers, inference services.

## SVG Graph Export

Export your dependency graph as a standalone SVG image:

```bash
forjar graph --format svg > graph.svg
```

The SVG output includes color-coded nodes by resource type, arrow markers for dependencies, and a grid layout — no external renderer required.

## Training Reproducibility Proof

Generate a cryptographic reproducibility certificate for ML training runs:

```bash
# Generate reproducibility proof
forjar repro-proof -f training.yaml --state-dir state

# JSON output for CI integration
forjar repro-proof -f training.yaml --state-dir state --json
```

The certificate includes: config BLAKE3 hash, git SHA, store artifact hashes, state hash, and a composite certificate hash. Use this to prove identical training outputs given identical inputs.

## Bundle Integrity Verification

Verify the integrity of a bundle's constituent files after air-gap transfer:

```bash
# Verify all files against BLAKE3 hashes
forjar bundle -f forjar.yaml --verify
```

Re-hashes every file (config, store, state) and reports pass/fail per file — detects corruption or tampering during physical media transfer.

## Data Freshness Monitoring

Monitor data artifact freshness with configurable SLA thresholds:

```bash
# Check all artifacts are fresh (default 24h SLA)
forjar data-freshness -f forjar.yaml

# Custom SLA: 8 hours max age
forjar data-freshness -f forjar.yaml --max-age 8

# JSON for CI pipelines
forjar data-freshness -f forjar.yaml --json
```

Reports stale/fresh/missing status per artifact (output_artifacts, store files, state locks). Returns non-zero if any artifact exceeds the SLA.

## Data Validation

Validate data integrity across your infrastructure:

```bash
# Validate all resources
forjar data-validate -f forjar.yaml

# Validate specific resource
forjar data-validate -f forjar.yaml --resource data-loader
```

Checks: file existence, non-empty, BLAKE3 integrity hashes, store content-addressing consistency.

## Training Checkpoint Management

Track and manage ML training checkpoints:

```bash
# List all checkpoints (sorted newest-first)
forjar checkpoint -f training.yaml

# Garbage collect, keep latest 3
forjar checkpoint -f training.yaml --gc --keep 3

# Filter by machine
forjar checkpoint -f training.yaml --machine gpu-box
```

Detects checkpoint resources by type (model), tags (checkpoint/training/ml), or resource_group (checkpoints).

## Dataset Lineage

Track data pipeline lineage with Merkle-hashed dependency graphs:

```bash
# Show dataset lineage graph
forjar dataset-lineage -f pipeline.yaml

# JSON output for tooling
forjar dataset-lineage -f pipeline.yaml --json
```

Builds a lineage graph from data-tagged resources, tracking source → transform → output dependencies with BLAKE3 content hashes.

## Data Sovereignty

Audit data sovereignty compliance across your infrastructure:

```bash
# Show sovereignty report
forjar sovereignty -f forjar.yaml

# JSON for compliance tooling
forjar sovereignty -f forjar.yaml --json
```

Tag resources with `jurisdiction:EU`, `classification:PII`, `residency:eu-west-1` to track data governance. Reports tagged vs untagged resources and state file hashes.

## Cost Estimation

Static cost analysis before applying:

```bash
forjar cost-estimate -f forjar.yaml
forjar cost-estimate -f forjar.yaml --json
```

Estimates execution time per resource by type (package: ~30s, file: ~2s, model: ~300s, GPU: ~60s). Reports total sequential time and complexity classification.

## Model Evaluation Pipeline

Gate model promotion with evaluation checks:

```bash
forjar model-eval -f training.yaml
forjar model-eval -f training.yaml --resource eval-run --json
```

Evaluates model/ml/eval-tagged resources. Checks that `completion_check` is defined and `output_artifacts` exist. Returns non-zero if evaluations fail.

## Agent Infrastructure Recipes

### Single MCP Server

Deploy a pforge MCP server with health monitoring:

```bash
forjar apply -f examples/pforge-mcp-server.yaml
```

4-phase pipeline: install pforge binary → write config → start service → health check.

### Full Agent Deployment

Composable agent recipe: GPU + model + config + MCP + health:

```bash
forjar apply -f examples/agent-deployment.yaml
```

5-phase pipeline with template parameters for model, GPU driver, and MCP port.

### Multi-Agent Fleet

Deploy across GPU fleet with load balancing and tool permission policies:

```bash
forjar apply -f examples/multi-agent-fleet.yaml
```

3-machine deployment with nginx upstream load balancer, per-agent tool-policy.yaml enforcement, and fleet health checks.
