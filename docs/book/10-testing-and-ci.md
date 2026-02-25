# Testing & CI/CD Integration

This chapter covers testing strategies for forjar configs, from local validation to full CI/CD pipeline integration.

## Validation Pyramid

Test your infrastructure configs at three levels:

```
     ┌─────────┐
     │  Apply   │  Integration: apply to containers/staging
     ├─────────┤
     │  Check   │  Pre-flight: run check scripts on live machines
     ├─────────┤
     │ Validate │  Static: parse, validate, lint, plan, fmt
     └─────────┘
```

## Level 1: Static Validation

Static validation catches most errors without connecting to any machine.

### Validate

```bash
forjar validate -f forjar.yaml
```

Checks:
- YAML syntax
- Version is "1.0"
- Name is non-empty
- Resources reference valid machines
- Dependencies reference valid resources
- No circular dependencies
- File state is valid (file, directory, symlink, absent)
- Service state is valid (running, stopped, enabled, disabled)
- Mount state is valid (mounted, unmounted, absent)
- Docker state is valid (running, stopped, absent)
- Network protocol (tcp, udp) and action (allow, deny, reject) are valid
- Cron schedule has exactly 5 fields
- Symlink resources have a target field

### Lint

```bash
forjar lint -f forjar.yaml
```

Detects best-practice violations:
- Unused machines
- Resources without tags (when many resources exist)
- Duplicate content across file resources
- Dependencies on non-existent resources
- Empty package lists

### Format Check

```bash
forjar fmt -f forjar.yaml --check
```

Exits non-zero if the config is not in canonical format. Use `forjar fmt` to fix.

### Plan (Dry Run)

```bash
forjar plan -f forjar.yaml
```

Shows what would change without connecting to machines. Useful for code review.

### Graph

```bash
forjar graph -f forjar.yaml
```

Generates a Mermaid or DOT dependency graph. Paste into GitHub PRs for visual review.

## Level 2: Pre-flight Checks

Run check scripts against live machines to verify preconditions.

```bash
# Check all resources
forjar check -f forjar.yaml

# Check a specific tag
forjar check -f forjar.yaml --tag critical

# JSON output for CI
forjar check -f forjar.yaml --json
```

Check scripts verify current state without modifying anything:
- Package: is the package installed?
- File: does the file exist?
- Service: is the service active/enabled?
- Mount: is the mount point active?

## Level 3: Integration Testing with Containers

For full integration testing, use container transport to apply configs in an isolated environment.

### Container Config

```yaml
version: "1.0"
name: integration-test

machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      name: forjar-test
      ephemeral: true
      init: true

resources:
  base-packages:
    type: package
    machine: test-box
    provider: apt
    packages: [curl, jq]
```

### Test Workflow

```bash
# Validate first
forjar validate -f test-config.yaml

# Plan
forjar plan -f test-config.yaml

# Apply to ephemeral container
forjar apply -f test-config.yaml --state-dir /tmp/test-state

# Check for drift (should be zero)
forjar drift -f test-config.yaml --state-dir /tmp/test-state --tripwire

# Apply again (should be idempotent)
forjar apply -f test-config.yaml --state-dir /tmp/test-state
```

## CI/CD Pipeline

### GitHub Actions

```yaml
name: Infrastructure Tests

on:
  pull_request:
    paths: ['forjar.yaml', 'recipes/**']

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install forjar
        run: cargo install forjar

      - name: Validate config
        run: forjar validate -f forjar.yaml

      - name: Lint config
        run: forjar lint -f forjar.yaml

      - name: Check formatting
        run: forjar fmt -f forjar.yaml --check

      - name: Show plan
        run: forjar plan -f forjar.yaml

      - name: Generate graph
        run: |
          forjar graph -f forjar.yaml > graph.md
          echo "## Dependency Graph" >> $GITHUB_STEP_SUMMARY
          echo '```mermaid' >> $GITHUB_STEP_SUMMARY
          cat graph.md >> $GITHUB_STEP_SUMMARY
          echo '```' >> $GITHUB_STEP_SUMMARY

  integration:
    runs-on: ubuntu-latest
    needs: validate
    steps:
      - uses: actions/checkout@v4

      - name: Install forjar
        run: cargo install forjar

      - name: Apply to container
        run: |
          forjar apply -f test-config.yaml \
            --state-dir /tmp/test-state

      - name: Verify idempotency
        run: |
          OUTPUT=$(forjar apply -f test-config.yaml \
            --state-dir /tmp/test-state 2>&1)
          echo "$OUTPUT"
          echo "$OUTPUT" | grep -q "0 to add"

      - name: Check drift
        run: |
          forjar drift -f test-config.yaml \
            --state-dir /tmp/test-state --tripwire
```

### Pre-Deploy Checklist

Before deploying to production, run this sequence:

```bash
# 1. Static checks
forjar validate -f forjar.yaml
forjar lint -f forjar.yaml
forjar fmt -f forjar.yaml --check

# 2. Preview changes
forjar plan -f forjar.yaml

# 3. Review generated scripts
forjar plan -f forjar.yaml --output-dir /tmp/review-scripts
ls /tmp/review-scripts/

# 4. Apply with auto-commit
forjar apply -f forjar.yaml --auto-commit

# 5. Post-apply drift check
forjar drift -f forjar.yaml --tripwire
```

## Monitoring After Deployment

### Scheduled Drift Detection

```bash
# Cron: check drift every 15 minutes
*/15 * * * * forjar drift -f /opt/infra/forjar.yaml \
  --tripwire --alert-cmd "/opt/scripts/alert.sh" \
  >> /var/log/forjar-drift.log 2>&1
```

### Anomaly Detection

```bash
# Weekly anomaly scan
forjar anomaly --state-dir state --json | jq '.anomalies'
```

### History Audit

```bash
# Last 20 applies
forjar history -n 20

# JSON for dashboards
forjar history --json | jq '.events[] | {ts: .ts, event: .event}'
```

## Script Auditing

Before running generated scripts on production machines, review them:

```bash
# Export all scripts to a directory
forjar plan -f forjar.yaml --output-dir /tmp/audit-scripts

# Review structure
tree /tmp/audit-scripts/
# /tmp/audit-scripts/
# ├── intel/
# │   ├── bash-aliases.sh
# │   ├── cargo-tools.sh
# │   └── nfs-mount.sh
# └── web-server/
#     ├── nginx-config.sh
#     └── ssl-cert.sh

# Audit a specific script
cat /tmp/audit-scripts/web-server/nginx-config.sh
```

Every script starts with `set -euo pipefail` for safety. File resources use heredocs with single-quoted delimiters to prevent variable expansion. Service resources include systemd detection guards.

### Reviewing Templates

Templates are resolved before script generation. To verify resolution:

```bash
# Show resolved config (templates expanded)
forjar show -f forjar.yaml --json | jq '.resources["nginx-config"].content'

# Compare raw config vs resolved
diff <(grep content forjar.yaml) <(forjar show -f forjar.yaml --json | jq -r '.resources["nginx-config"].content')
```

## Testing Strategies

### Canary Deploys

Use tags to test changes on a subset of machines first:

```yaml
resources:
  nginx-config:
    type: file
    machine: all-webservers
    path: /etc/nginx/nginx.conf
    content: |
      worker_processes auto;
    tags: [web, canary]

  nginx-service:
    type: service
    machine: all-webservers
    name: nginx
    restart_on: [nginx-config]
    tags: [web, canary]
```

```bash
# Deploy to canary first
forjar apply -f forjar.yaml --tag canary -m canary-web1

# Verify
forjar drift -f forjar.yaml --tripwire -m canary-web1

# If good, deploy to all
forjar apply -f forjar.yaml --tag web
```

### Idempotency Testing

Verify that applying twice produces no changes:

```bash
# First apply
forjar apply -f forjar.yaml --state-dir /tmp/idem-test

# Second apply — should show 0 changes
OUTPUT=$(forjar apply -f forjar.yaml --state-dir /tmp/idem-test 2>&1)
echo "$OUTPUT"

# Verify
if echo "$OUTPUT" | grep -q "0 changed"; then
  echo "PASS: Idempotent"
else
  echo "FAIL: Not idempotent"
  exit 1
fi
```

### Drift Testing

Intentionally introduce drift and verify detection:

```bash
# Apply config
forjar apply -f forjar.yaml

# Introduce drift (modify a managed file)
ssh web-server "echo 'rogue change' >> /etc/nginx/nginx.conf"

# Detect drift
forjar drift -f forjar.yaml -m web-server

# Auto-remediate
forjar drift -f forjar.yaml --auto-remediate -m web-server

# Verify drift is resolved
forjar drift -f forjar.yaml --tripwire -m web-server
```

## GitOps Workflow

### PR-Based Infrastructure Changes

```
Developer                    CI                          Production
    │                        │                               │
    ├── edit forjar.yaml ──► │                               │
    │                        ├── validate ──► ✓               │
    │                        ├── lint ──► ✓                   │
    │                        ├── plan ──► PR comment          │
    │                        ├── graph ──► PR summary         │
    │   ◄── review plan ─── │                               │
    │                        │                               │
    ├── merge PR ──────────► │                               │
    │                        ├── apply ──────────────────────► │
    │                        ├── drift --tripwire ──────────► │
    │                        ├── commit state ──► git          │
    │                        │                               │
```

### Post-Merge CI Job

```yaml
name: Deploy Infrastructure

on:
  push:
    branches: [main]
    paths: ['forjar.yaml', 'recipes/**']

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install forjar
        run: cargo install forjar

      - name: Apply changes
        run: |
          forjar apply -f forjar.yaml \
            --state-dir state/ \
            --auto-commit
        env:
          SSH_PRIVATE_KEY: ${{ secrets.DEPLOY_KEY }}

      - name: Post-apply drift check
        run: |
          forjar drift -f forjar.yaml \
            --state-dir state/ --tripwire

      - name: Push state
        run: |
          git push origin main
```

## Testing Best Practices

1. **Validate on every PR** — `forjar validate && forjar lint && forjar fmt --check` in CI.

2. **Review plans, not diffs** — `forjar plan` shows the actual impact. YAML diffs miss template resolution and dependency effects.

3. **Test idempotency** — Apply twice; the second apply should show zero changes.

4. **Use ephemeral containers** — Integration tests in containers are cheap and fast. Never test on production.

5. **Monitor drift continuously** — `--tripwire` in cron catches unauthorized changes early.

6. **Audit scripts before apply** — `forjar plan --output-dir` lets you review the exact shell scripts before they run on your machines.

7. **Use tags for staged rollouts** — `forjar apply --tag canary` applies only to canary resources first.

8. **Keep state in git** — Commit state after every apply. This gives you rollback, audit trail, and diff for free.

9. **Test templates separately** — Use `forjar show --json` to verify template resolution before applying.

10. **Set up scheduled drift checks** — A cron job running `forjar drift --tripwire` catches unauthorized changes within minutes.
