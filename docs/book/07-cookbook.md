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

Files are base64-encoded locally and decoded on the remote machine, supporting binary files and all transports.

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
