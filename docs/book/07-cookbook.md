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
