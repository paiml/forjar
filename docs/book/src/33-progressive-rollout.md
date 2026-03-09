# Progressive Rollout

Forjar's progressive rollout system (FJ-3507) gives operators fine-grained
control over how changes propagate across a fleet. Rather than applying to
every machine at once, rollouts proceed in waves with health checks between
each step. If a health check fails, Forjar automatically rolls back the
affected machines to their previous state.

## Rollout Strategies

Three strategies are available in the `rollout` block of an environment's
`promotion` config:

| Strategy | Behaviour |
|----------|-----------|
| `canary` | Apply to a small fixed number of machines first, verify health, then proceed to the rest |
| `percentage` | Apply in percentage-based waves (e.g. 25%, 50%, 100%) with health checks between each step |
| `all-at-once` | Apply to every machine simultaneously (the default when no `rollout` block is present) |

## RolloutConfig YAML Format

The `rollout` block lives inside an environment's `promotion` section:

```yaml
environments:
  prod:
    description: "Production"
    params:
      replicas: 4
    machines:
      web:
        addr: prod-web.internal
      api:
        addr: prod-api.internal
    promotion:
      from: staging
      auto_approve: false
      gates:
        - validate: { deep: true }
        - policy: { strict: true }
      rollout:
        strategy: canary
        canary_count: 1
        health_check: "curl -sf http://{{ machine.addr }}:8080/health"
        health_timeout: 30s
        cooldown: 60s
        percentage_steps: [25, 50, 100]
```

### Field Reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `strategy` | string | `all-at-once` | One of `canary`, `percentage`, `all-at-once` |
| `canary_count` | integer | `1` | Number of machines in the canary wave (only used with `canary` strategy) |
| `health_check` | string | none | Shell command to verify machine health; must exit 0 to pass |
| `health_timeout` | duration | `30s` | Maximum time to wait for a health check to succeed |
| `cooldown` | duration | `0s` | Pause between waves to allow metrics to stabilise |
| `percentage_steps` | list of integers | `[100]` | Percentage milestones for the `percentage` strategy |

The `health_check` field supports `{{ machine.addr }}` and
`{{ machine.hostname }}` template variables, resolved per-machine at
execution time.

## Canary Strategy

The canary strategy is the most conservative option. It splits the fleet
into two waves:

1. **Canary wave** -- the first `canary_count` machines receive the change.
2. **Remainder wave** -- all other machines receive the change after the
   canary wave passes health checks.

```yaml
rollout:
  strategy: canary
  canary_count: 2
  health_check: "curl -sf http://{{ machine.addr }}:8080/health"
  health_timeout: 30s
  cooldown: 120s
```

Execution flow:

```
Wave 1: apply to 2 canary machines
  ├── health check machine-01 ... OK
  ├── health check machine-02 ... OK
  └── cooldown 120s
Wave 2: apply to remaining machines
  ├── health check machine-03 ... OK
  ├── health check machine-04 ... OK
  └── health check machine-05 ... OK
Rollout complete.
```

## Percentage Strategy

The percentage strategy applies changes in graduated waves defined by
`percentage_steps`. Each step is a percentage of the total fleet:

```yaml
rollout:
  strategy: percentage
  percentage_steps: [10, 25, 50, 100]
  health_check: "systemctl is-active --quiet myapp"
  health_timeout: 15s
  cooldown: 60s
```

For a fleet of 20 machines the waves would be:

| Step | Percentage | Machines Applied |
|------|-----------|-----------------|
| 1 | 10% | 2 |
| 2 | 25% | 5 (cumulative) |
| 3 | 50% | 10 (cumulative) |
| 4 | 100% | 20 (all) |

Each step runs health checks on every machine that received the change in
that wave before advancing. The `cooldown` delay applies between steps.

## Health Check Integration

Health checks run after each wave completes. A health check is a shell
command executed on the Forjar controller (not on the target machine). Use
SSH, curl, or any tool that can reach the target:

```yaml
health_check: "curl -sf http://{{ machine.addr }}:8080/health"
```

### Health check lifecycle

1. After applying a wave, Forjar waits 5 seconds for services to start.
2. The `health_check` command runs for each machine in the wave.
3. If the command exits 0 within `health_timeout`, the machine passes.
4. If the command exits non-zero or times out, the machine fails.
5. Any single failure in a wave triggers auto-rollback for the entire wave.

### Multiple health checks

Combine multiple checks in a single command:

```yaml
health_check: |
  curl -sf http://{{ machine.addr }}:8080/health &&
  curl -sf http://{{ machine.addr }}:8080/ready &&
  ssh {{ machine.addr }} 'systemctl is-active myapp'
```

## Auto-Rollback on Failure

When a health check fails, Forjar automatically rolls back every machine
in the failed wave to its previous state. Machines from earlier waves that
already passed are left untouched.

```
Wave 1: apply to canary (machine-01)
  └── health check machine-01 ... OK
Wave 2: apply to remaining (machine-02, machine-03)
  ├── health check machine-02 ... OK
  └── health check machine-03 ... FAILED
Auto-rollback: reverting machine-02, machine-03
  ├── rollback machine-02 ... OK
  └── rollback machine-03 ... OK
Rollout FAILED. Canary machines (machine-01) remain on new version.
```

The rollback restores the previous state lock for each affected machine
and re-applies the prior resource configuration. An event is recorded in
the environment's `events.jsonl` with type `rollback`:

```json
{
  "timestamp": "2026-03-09T14:22:01Z",
  "type": "rollback",
  "environment": "prod",
  "wave": 2,
  "machines": ["machine-02", "machine-03"],
  "reason": "health_check_failed",
  "detail": "machine-03: curl exit code 7 (connection refused)"
}
```

## `forjar promote`

The `forjar promote` command drives the full promotion workflow including
gate evaluation and progressive rollout:

```bash
# Promote from staging to prod with configured rollout strategy
forjar promote --env prod -f forjar.yaml

# Promote with explicit approval prompt (default when auto_approve: false)
forjar promote --env prod -f forjar.yaml --approve
```

### Promote output

```
Promoting staging → prod
Gates:
  [PASS] validate (deep=true)
  [PASS] policy (strict=true)

Rollout strategy: canary (1 canary, then remainder)
Wave 1/2: applying to 1 canary machine(s)
  [apply] prod-web-01 ... ok
  [health] prod-web-01 ... ok (204ms)
  cooldown 60s ...
Wave 2/2: applying to 3 remaining machine(s)
  [apply] prod-web-02 ... ok
  [apply] prod-web-03 ... ok
  [apply] prod-web-04 ... ok
  [health] prod-web-02 ... ok (187ms)
  [health] prod-web-03 ... ok (192ms)
  [health] prod-web-04 ... ok (201ms)

Promotion complete. 4 machine(s) updated.
```

### Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--env` | required | Target environment to promote into |
| `-f`, `--file` | `forjar.yaml` | Path to config file |
| `--approve` | `false` | Pre-approve the promotion (skip interactive prompt) |
| `--dry-run` | `false` | Simulate the rollout without applying changes |
| `--json` | `false` | Output results as JSON |

## Dry-Run Mode

Use `--dry-run` to simulate a rollout without making any changes. Forjar
evaluates all gates, computes the wave plan, and prints what would happen:

```bash
forjar promote --env prod -f forjar.yaml --dry-run
```

```
[DRY RUN] Promoting staging → prod
Gates:
  [PASS] validate (deep=true)
  [PASS] policy (strict=true)

Rollout plan (canary strategy):
  Wave 1: 1 machine(s) [prod-web-01]
  Wave 2: 3 machine(s) [prod-web-02, prod-web-03, prod-web-04]
  Health check: curl -sf http://{{ machine.addr }}:8080/health
  Health timeout: 30s
  Cooldown: 60s

No changes applied (dry-run mode).
```

Dry-run is useful in CI pipelines to verify that the promotion plan looks
correct before an operator approves it:

```bash
# CI step: verify promotion plan
forjar promote --env prod -f forjar.yaml --dry-run --json | jq -e '.gates | all(.passed)'
```

## Complete Example

A full `forjar.yaml` with three environments and progressive rollout on
the prod promotion:

```yaml
version: "1.0"
name: web-cluster

machines:
  web-01:
    hostname: web-01
    addr: 10.0.0.1
  web-02:
    hostname: web-02
    addr: 10.0.0.2
  web-03:
    hostname: web-03
    addr: 10.0.0.3

params:
  app_version: "2.1.0"
  log_level: debug

resources:
  app:
    type: package
    machine: web-01
    packages: [myapp]
  config:
    type: file
    machine: web-01
    path: /etc/myapp/config.yaml
    content: |
      version: {{ params.app_version }}
      log_level: {{ params.log_level }}

environments:
  dev:
    description: "Development"
    params:
      log_level: debug
    machines:
      web-01:
        addr: dev-web-01.internal

  staging:
    description: "Staging"
    params:
      log_level: info
    machines:
      web-01:
        addr: staging-web-01.internal
      web-02:
        addr: staging-web-02.internal
    promotion:
      from: dev
      auto_approve: true
      gates:
        - validate: { deep: true }

  prod:
    description: "Production"
    params:
      log_level: warn
    machines:
      web-01:
        addr: prod-web-01.internal
      web-02:
        addr: prod-web-02.internal
      web-03:
        addr: prod-web-03.internal
    promotion:
      from: staging
      auto_approve: false
      gates:
        - validate: { deep: true, exhaustive: true }
        - policy: { strict: true }
        - script: "curl -sf http://staging-web-01.internal:8080/health"
      rollout:
        strategy: canary
        canary_count: 1
        health_check: "curl -sf http://{{ machine.addr }}:8080/health"
        health_timeout: 30s
        cooldown: 60s
```

Promote to production:

```bash
forjar promote --env prod -f forjar.yaml
```
