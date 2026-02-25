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

## Testing Patterns

### Unit Testing Configs

Test individual config files without applying them:

```bash
#!/bin/bash
# test-configs.sh — validate all config variants

set -euo pipefail

for config in configs/*.yaml; do
    echo "Testing: $config"
    forjar validate -f "$config" || { echo "FAIL: $config"; exit 1; }
    forjar lint -f "$config" || { echo "LINT FAIL: $config"; exit 1; }
    forjar plan -f "$config" --state-dir /dev/null 2>/dev/null || true
    echo "  OK"
done

echo "All configs valid."
```

### Snapshot Testing

Compare plan output against a known-good baseline:

```bash
#!/bin/bash
# snapshot-test.sh

# Generate current plan
forjar plan -f forjar.yaml --state-dir state/ > /tmp/current-plan.txt

# Compare to baseline
if diff -q snapshots/expected-plan.txt /tmp/current-plan.txt > /dev/null 2>&1; then
    echo "Plan matches snapshot."
else
    echo "Plan changed!"
    diff snapshots/expected-plan.txt /tmp/current-plan.txt
    echo ""
    echo "If this is expected, update the snapshot:"
    echo "  cp /tmp/current-plan.txt snapshots/expected-plan.txt"
    exit 1
fi
```

### Container Integration Test Matrix

Test all resource types in containers:

```yaml
# test-matrix.yaml
version: "1.0"
name: integration-tests

machines:
  test:
    hostname: test
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      ephemeral: true
      privileged: true    # Needed for service tests

resources:
  # Package install
  test-packages:
    type: package
    machine: test
    provider: apt
    packages: [curl, jq]

  # File creation
  test-file:
    type: file
    machine: test
    path: /etc/test/config.yaml
    content: "test: true"
    mode: "0644"
    depends_on: [test-packages]

  # Directory creation
  test-dir:
    type: file
    machine: test
    state: directory
    path: /opt/test-app
    mode: "0755"

  # User creation
  test-user:
    type: user
    machine: test
    name: testuser
    shell: /bin/bash
    home: /home/testuser
```

Run the test matrix:

```bash
# Apply
forjar apply -f test-matrix.yaml --state-dir /tmp/test-state/

# Verify idempotency
forjar apply -f test-matrix.yaml --state-dir /tmp/test-state/
# Should show: 0 converged, N unchanged, 0 failed

# Check for drift
forjar drift -f test-matrix.yaml --state-dir /tmp/test-state/

# Clean up (ephemeral: true handles container cleanup)
```

### Testing Recipes in Isolation

Test recipes independently before using them in production configs:

```yaml
# test-recipe.yaml
version: "1.0"
name: recipe-test

machines:
  test:
    hostname: test
    addr: container
    transport: container
    container:
      image: ubuntu:22.04
      ephemeral: true

resources:
  web:
    type: recipe
    machine: test
    recipe: web-server
    inputs:
      domain: test.example.com
      port: 8080
      log_level: debug
```

```bash
# Validate recipe expansion
forjar validate -f test-recipe.yaml

# View expanded resources
forjar graph -f test-recipe.yaml

# Apply in container
forjar apply -f test-recipe.yaml --state-dir /tmp/recipe-test/
```

## Monitoring and Alerting

### Prometheus Metrics

Export forjar drift status as Prometheus metrics:

```bash
#!/bin/bash
# forjar-exporter.sh — run as a cron job, write to textfile collector

METRICS_FILE="/var/lib/prometheus/node-exporter/forjar.prom"

drift_json=$(forjar drift -f /opt/infra/forjar.yaml --state-dir /opt/infra/state/ --json 2>/dev/null)
drift_count=$(echo "$drift_json" | jq '.findings | length' 2>/dev/null || echo 0)

cat > "$METRICS_FILE" <<EOF
# HELP forjar_drift_count Number of resources with detected drift
# TYPE forjar_drift_count gauge
forjar_drift_count $drift_count
EOF
```

### Slack/Discord Notifications

Alert when drift is detected:

```bash
#!/bin/bash
# drift-alert.sh — called by forjar drift --alert-cmd

WEBHOOK_URL="https://hooks.slack.com/services/..."
DRIFT_COUNT="${FORJAR_DRIFT_COUNT:-0}"

if [ "$DRIFT_COUNT" -gt 0 ]; then
    curl -s -X POST "$WEBHOOK_URL" \
        -H 'Content-type: application/json' \
        -d "{\"text\": \"Forjar drift alert: $DRIFT_COUNT resources drifted\"}"
fi
```

Use with forjar:

```bash
forjar drift -f forjar.yaml --state-dir state/ --alert-cmd "./drift-alert.sh"
```

## GitOps Workflow

### Pull Request Validation

Run forjar in CI on every pull request:

```yaml
# .github/workflows/validate.yml
name: Validate Infrastructure
on: [pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install forjar
        run: cargo install forjar

      - name: Validate
        run: forjar validate -f forjar.yaml

      - name: Lint
        run: forjar lint -f forjar.yaml

      - name: Format check
        run: forjar fmt -f forjar.yaml --check

      - name: Preview plan
        run: |
          forjar plan -f forjar.yaml --state-dir state/
        # Plan output appears in CI logs for review
```

### Auto-Deploy on Merge

Deploy automatically when changes merge to main:

```yaml
# .github/workflows/deploy.yml
name: Deploy Infrastructure
on:
  push:
    branches: [main]
    paths: ['forjar.yaml', 'recipes/**']

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: production
    steps:
      - uses: actions/checkout@v4

      - name: Setup SSH
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.DEPLOY_KEY }}" > ~/.ssh/id_ed25519
          chmod 600 ~/.ssh/id_ed25519

      - name: Install and apply
        run: |
          cargo install forjar
          forjar drift -f forjar.yaml --state-dir state/ --tripwire || true
          forjar apply -f forjar.yaml --state-dir state/
          forjar drift -f forjar.yaml --state-dir state/ --tripwire

      - name: Commit state
        run: |
          git add state/
          git commit -m "forjar: deploy $(date -I)" || true
          git push
```
