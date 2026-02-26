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
