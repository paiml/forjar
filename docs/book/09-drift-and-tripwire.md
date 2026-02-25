# Drift Detection & Tripwire

Drift detection is forjar's mechanism for discovering unauthorized changes — modifications made to managed infrastructure outside of `forjar apply`. This chapter covers how drift detection works, how to use it, and how to integrate it into your workflow.

## How Drift Detection Works

Every `forjar apply` records a BLAKE3 hash of each resource's state in the lock file (`state/{machine}/state.lock.yaml`). Drift detection re-checks the live state and compares it to the recorded hash.

### File Resources

For file resources, forjar computes a BLAKE3 hash of the file contents on disk:

```
# Lock file entry (after apply)
config-file:
  type: file
  status: converged
  hash: "blake3:7f83b1657ff1fc53b..."
  details:
    path: /etc/nginx/nginx.conf
    content_hash: "blake3:7f83b1657ff1fc53b..."
```

During drift detection, forjar re-reads the file and computes `blake3(file_contents)`. If the hash differs, drift is reported.

- **Files**: BLAKE3 of file contents
- **Directories**: BLAKE3 of sorted `(relative_path, file_hash)` pairs
- **Symlinks**: Skipped during directory hashing

### Non-File Resources

For packages, services, mounts, users, cron jobs, docker containers, and network rules, forjar re-runs the resource's `state_query_script` via transport and compares the BLAKE3 hash of the output to the `live_hash` stored at apply time.

```
# Lock file entry with live_hash
nginx-service:
  type: service
  status: converged
  hash: "blake3:a1b2c3..."
  details:
    service_name: nginx
    live_hash: "blake3:d4e5f6..."
```

The `state_query_script` captures the current state:

```bash
# Service state query
systemctl show 'nginx' --property=ActiveState,SubState,UnitFileState 2>/dev/null || echo 'MISSING'

# Package state query
dpkg-query -W -f='${Package}=${Version}\n' curl htop 2>/dev/null || echo 'MISSING'

# User state query
id 'deploy' >/dev/null 2>&1 && {
  echo "user=deploy"
  echo "uid=$(id -u 'deploy')"
  echo "shell=$(getent passwd 'deploy' | cut -d: -f7)"
} || echo 'user=MISSING'
```

## Using Drift Detection

### Basic Drift Check

```bash
# Check all machines for drift
forjar drift -f forjar.yaml

# Check a specific machine
forjar drift -f forjar.yaml -m web-server

# Dry run — list what would be checked without connecting
forjar drift -f forjar.yaml --dry-run
```

Output shows each finding with expected vs actual hash:

```
DRIFT DETECTED: 2 finding(s)

  web-server/config-file:
    expected: blake3:7f83b1657ff1fc53b...
    actual:   blake3:9a4e2d1c3b5f7a8e0...
    detail:   /etc/nginx/nginx.conf content changed

  web-server/nginx-service:
    expected: blake3:d4e5f6789abc012...
    actual:   blake3:1a2b3c4d5e6f789...
    detail:   service state changed
```

### JSON Output

For scripting and CI integration:

```bash
forjar drift -f forjar.yaml --json
```

```json
{
  "drift_count": 2,
  "findings": [
    {
      "machine": "web-server",
      "resource": "config-file",
      "expected": "blake3:7f83b...",
      "actual": "blake3:9a4e2...",
      "detail": "/etc/nginx/nginx.conf content changed"
    }
  ]
}
```

### Tripwire Mode

Use `--tripwire` to exit non-zero on any drift — ideal for CI/cron:

```bash
# Exit code 1 if any drift detected
forjar drift -f forjar.yaml --tripwire

# Use in CI pipeline
forjar drift -f forjar.yaml --tripwire || notify-slack "Drift detected!"
```

### Alert Commands

Run a custom command when drift is detected:

```bash
forjar drift -f forjar.yaml --alert-cmd "slack-notify.sh"
```

The alert command receives `$FORJAR_DRIFT_COUNT` as an environment variable.

### Auto-Remediation

Automatically re-apply all resources to restore desired state:

```bash
forjar drift -f forjar.yaml --auto-remediate
```

This is equivalent to detecting drift and then running `forjar apply --force`. Use with caution — verify the config is still correct before auto-remediating.

## Scheduled Drift Detection

### Cron-Based Monitoring

Add a cron job to check for drift periodically:

```bash
# Check every 15 minutes, alert on drift
*/15 * * * * cd /opt/infra && forjar drift -f forjar.yaml --tripwire --alert-cmd "/opt/scripts/drift-alert.sh" >> /var/log/forjar-drift.log 2>&1
```

### Systemd Timer

For systemd-based scheduling:

```ini
# /etc/systemd/system/forjar-drift.service
[Unit]
Description=Forjar drift detection

[Service]
Type=oneshot
WorkingDirectory=/opt/infra
ExecStart=/usr/local/bin/forjar drift -f forjar.yaml --tripwire --alert-cmd "/opt/scripts/alert.sh"

# /etc/systemd/system/forjar-drift.timer
[Unit]
Description=Run forjar drift check every 15 minutes

[Timer]
OnBootSec=5min
OnUnitActiveSec=15min

[Install]
WantedBy=timers.target
```

## Drift Investigation Workflow

When drift is detected, follow this workflow to diagnose and resolve:

```bash
# 1. Detect drift
forjar drift -f forjar.yaml --json > /tmp/drift-report.json

# 2. Inspect findings
cat /tmp/drift-report.json | jq '.findings[] | {resource: .resource, detail: .detail}'

# 3. Check event history for the drifted resource
forjar history -m web-server -n 20 | grep config-file

# 4. Compare expected vs actual
forjar show -f forjar.yaml -r config-file --json | jq '.content'
ssh web-server "cat /etc/nginx/nginx.conf"

# 5. Decide: remediate or accept
# Option A: Restore desired state
forjar apply -f forjar.yaml --force -r config-file

# Option B: Update config to match live state
# Edit forjar.yaml, then apply
```

### Root Cause Analysis

Common drift causes and their signatures:

| Pattern | Likely Cause | Resolution |
|---------|-------------|------------|
| Single file drifts repeatedly | Manual edits by operators | Add comment "Managed by forjar — do not edit" |
| Package version changes | Auto-updates (unattended-upgrades) | Pin version in config |
| Service state toggles | External monitoring restarts | Coordinate with monitoring team |
| Multiple resources drift together | Ansible/puppet overlap | Remove competing tool |
| Drift only on one machine | SSH'd in and made changes | Audit SSH access logs |

## Anomaly Detection

Beyond simple drift, `forjar anomaly` analyzes event history to find resources with suspicious patterns:

```bash
forjar anomaly --state-dir state
```

### What It Detects

**High churn** — Resources that converge abnormally often (z-score > 1.5). This suggests a resource is being externally modified and re-converged repeatedly.

The z-score calculation:
1. Count `resource_converged` events per resource across all machines
2. Compute mean and standard deviation of converge counts
3. Flag resources where `(count - mean) / stddev > 1.5`

**High failure rate** — Resources with more than 20% failure rate (minimum 2 failures). Indicates a persistent configuration problem.

**Drift events** — Any resource that has had drift detected in its history.

```
ANOMALIES: 2 finding(s)

  web-server/app-config:
    type: high_churn
    converge_count: 47
    z_score: 3.92

  db-server/mysql-config:
    type: high_failure_rate
    total_events: 10
    failures: 4
    rate: 40%
```

### JSON Output for Monitoring

```bash
# All anomalies
forjar anomaly --json | jq '.anomalies'

# Only high-churn resources
forjar anomaly --json | jq '.anomalies[] | select(.type == "high_churn")'

# Resources with failure rate > 30%
forjar anomaly --json | jq '.anomalies[] | select(.type == "high_failure_rate" and .rate > 30)'
```

### Responding to Anomalies

| Anomaly Type | Investigation | Fix |
|-------------|--------------|-----|
| High churn | Check who/what is modifying the resource between applies | Lock down SSH, add file immutability, or increase check interval |
| High failure rate | Check resource error logs in events.jsonl | Fix underlying issue (package repo, permissions, network) |
| Drift detected | Compare expected vs actual hash | Re-apply or update config to match reality |

## Event Log

Every `forjar apply` and drift check writes to the event log at `state/{machine}/events.jsonl`. This is an append-only audit trail.

### Event Types

| Event | Description |
|-------|-------------|
| `apply_started` | Apply run begins (includes run_id, forjar version) |
| `resource_started` | Individual resource apply begins |
| `resource_converged` | Resource successfully converged (includes hash, duration) |
| `resource_failed` | Resource apply failed (includes error message) |
| `apply_completed` | Apply run finishes (includes summary counts) |

### Reading Event Logs

```bash
# Show last 10 events
forjar history --state-dir state

# Show events for a specific machine
forjar history -m web-server -n 20

# JSON output for parsing
forjar history --json | jq '.events[] | select(.event == "resource_failed")'
```

### Manual Log Inspection

```bash
# Raw event log
cat state/web-server/events.jsonl | jq .

# Find all failures
cat state/web-server/events.jsonl | jq 'select(.event == "resource_failed")'

# Count events by type
cat state/web-server/events.jsonl | jq -r '.event' | sort | uniq -c
```

## BLAKE3 Hashing

Forjar uses BLAKE3 for all content hashing. BLAKE3 is:

- **Fast**: ~4x faster than SHA-256 on modern hardware
- **Deterministic**: Same input always produces the same hash
- **Streaming**: Handles files of any size with constant memory (64KB buffer)

Hash format: `blake3:{64 hex chars}` (71 characters total).

### Hash Verification

```bash
# Verify a file's BLAKE3 hash matches the lock
b3sum /etc/nginx/nginx.conf
# Compare to: state/web-server/state.lock.yaml → config-file.details.content_hash
```

## Best Practices

1. **Run drift checks before every apply** — `forjar drift` before `forjar apply` shows what changed since the last apply.

2. **Use `--tripwire` in CI** — Catches unauthorized changes before they accumulate.

3. **Monitor anomalies weekly** — `forjar anomaly` identifies resources that need attention.

4. **Keep state in git** — `forjar apply --auto-commit` commits state after each apply, giving you a full history of infrastructure changes.

5. **Use `--alert-cmd` for production** — Don't just log drift; alert your team.

6. **Review before remediating** — `forjar drift --json` lets you review what changed before running `--auto-remediate`.

## Drift Detection Internals

### Hash Comparison Pipeline

The drift detection pipeline follows this sequence for each resource in the lock file:

```
For each resource in state.lock.yaml:
  1. Skip if status != Converged (failed/drifted resources are excluded)
  2. Determine resource type:
     ├── File resource:
     │   a. Read details.path from lock
     │   b. Read details.content_hash from lock
     │   c. Compute blake3(file_contents) on disk
     │   d. Compare: lock hash vs live hash
     │   e. If different → DriftFinding { resource_id, detail: "content changed" }
     │
     └── Non-file resource (service, package, user, etc.):
         a. Read details.live_hash from lock
         b. Re-run state_query_script via transport (local/SSH/container)
         c. Compute blake3(script_stdout)
         d. Compare: lock live_hash vs current output hash
         e. If different → DriftFinding { resource_id, detail: "state changed" }
```

### Directory Drift Detection

Directory resources use composite hashing — all files in the directory are included:

```
hash_directory(path):
  1. Walk directory recursively (sorted by path)
  2. Skip symlinks
  3. For each regular file: compute blake3(contents)
  4. Concatenate: "relative_path\0hash\0" for all files
  5. Final hash = blake3(concatenated string)
```

This means adding, removing, or modifying any file inside a managed directory triggers drift detection.

### Skip Conditions

Drift detection skips resources in these cases:

| Condition | Reason |
|-----------|--------|
| `status == Failed` | Resource never converged — no baseline to compare |
| `status == Unknown` | No previous state recorded |
| Missing `path` (file resources) | Cannot hash without a path |
| Missing `content_hash` (file resources) | No baseline hash to compare |
| Missing `live_hash` (non-file resources) | No baseline to compare |
| Non-string hash values | Corrupt or malformed lock entry |

### Full vs Local Drift Detection

Forjar provides two drift detection modes:

```bash
# Local-only (fast): checks files on the local filesystem
forjar drift -f forjar.yaml --state-dir ./state

# Full (via transport): re-runs state_query_script on remote machines
forjar drift -f forjar.yaml --state-dir ./state --full
```

Local drift checks only file resources (because they're on disk). Full drift uses the transport layer to re-query all resource types via their state_query scripts.

## Operational Workflows

### Pre-Deploy Drift Check

Before applying new changes, check for unauthorized modifications:

```bash
#!/bin/bash
# pre-deploy.sh — run before forjar apply

echo "=== Checking for drift ==="
drift_output=$(forjar drift -f forjar.yaml --state-dir ./state --json 2>&1)
drift_count=$(echo "$drift_output" | jq '.findings | length')

if [ "$drift_count" -gt 0 ]; then
    echo "WARNING: $drift_count resources have drifted:"
    echo "$drift_output" | jq -r '.findings[] | "  - \(.resource_id): \(.detail)"'
    echo ""
    echo "Options:"
    echo "  1. Review changes: forjar drift --json | jq '.findings'"
    echo "  2. Accept drift: forjar apply --force"
    echo "  3. Remediate: forjar drift --auto-remediate"
    exit 1
fi

echo "No drift detected. Safe to apply."
forjar apply -f forjar.yaml --state-dir ./state
```

### CI Drift Monitoring

Add a scheduled CI job to detect drift between deploys:

```yaml
# .github/workflows/drift-monitor.yml
name: Drift Monitor
on:
  schedule:
    - cron: '0 */6 * * *'  # Every 6 hours

jobs:
  check-drift:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install forjar
        run: cargo install --path .
      - name: Check drift
        run: |
          forjar drift -f forjar.yaml --state-dir ./state --json > drift.json
          FINDINGS=$(jq '.findings | length' drift.json)
          if [ "$FINDINGS" -gt 0 ]; then
            echo "::warning::$FINDINGS resources have drifted"
            jq -r '.findings[] | "::warning::\(.resource_id): \(.detail)"' drift.json
          fi
```

### Multi-Machine Drift Report

For fleets with many machines, generate a summary report:

```bash
# drift-report.sh — weekly drift summary
echo "=== Fleet Drift Report $(date -I) ==="

for machine_dir in state/*/; do
    machine=$(basename "$machine_dir")
    lock_file="$machine_dir/state.lock.yaml"

    if [ ! -f "$lock_file" ]; then
        echo "  $machine: no lock file (never applied)"
        continue
    fi

    # Count resources by status
    converged=$(grep -c "status: converged" "$lock_file" 2>/dev/null || echo 0)
    failed=$(grep -c "status: failed" "$lock_file" 2>/dev/null || echo 0)

    echo "  $machine: $converged converged, $failed failed"
done

echo ""
echo "=== Anomaly Detection ==="
forjar anomaly --state-dir state --json | jq -r '.anomalies[] | "  ⚠ \(.resource): \(.type) (z-score: \(.z_score))"'
```
