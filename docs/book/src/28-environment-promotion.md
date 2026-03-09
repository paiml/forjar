# Environment Promotion Pipelines

Forjar's environment system (FJ-3500) provides first-class dev/staging/prod
abstractions with quality gates, diff analysis, and progressive rollout.

## Defining Environments

Add an `environments:` block to your `forjar.yaml`. Each environment
overrides params and machine addresses from the base config:

```yaml
version: "1.0"
name: my-app
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
params:
  log_level: debug
  replicas: 1
resources:
  nginx:
    type: package
    machine: web
    packages: [nginx]

environments:
  dev:
    description: "Development"
    params:
      log_level: debug
      replicas: 1
    machines:
      web:
        addr: dev-web.internal

  staging:
    description: "Staging"
    params:
      log_level: info
      replicas: 2
    machines:
      web:
        addr: staging-web.internal
    promotion:
      from: dev
      auto_approve: true
      gates:
        - validate: { deep: true }
        - policy: { strict: true }

  prod:
    description: "Production"
    params:
      log_level: warn
      replicas: 4
    machines:
      web:
        addr: prod-web.internal
    promotion:
      from: staging
      auto_approve: false
      gates:
        - validate: { deep: true, exhaustive: true }
        - policy: { strict: true }
        - coverage: { min: 95 }
        - script: "curl -sf http://staging.internal/health"
      rollout:
        strategy: canary
        canary_count: 1
        percentage_steps: [25, 50, 100]
```

## How Resolution Works

Each environment inherits the base config and applies overrides:

1. **Params**: Environment params replace base params with matching keys.
   Unoverridden params inherit from the base.
2. **Machines**: Only the `addr` field is overridden per environment.
   All other machine fields (user, arch, ssh_key, etc.) inherit.
3. **Resources**: Shared across all environments (DRY by design).

## State Isolation

Each environment gets its own state directory:

```
.forjar/state/
├── dev/
│   ├── state.lock.yaml
│   └── events.jsonl
├── staging/
│   ├── state.lock.yaml
│   └── events.jsonl
└── prod/
    ├── state.lock.yaml
    └── events.jsonl
```

## Promotion Gates

Quality gates must pass before promoting between environments:

| Gate | Description |
|------|-------------|
| `validate` | Run `forjar validate` with `deep` and `exhaustive` options |
| `policy` | Run policy-as-code checks (strict mode treats warnings as errors) |
| `coverage` | Verify test coverage meets minimum threshold |
| `script` | Run custom shell command (must exit 0) |

## Progressive Rollout

For production promotions, configure progressive rollout:

```yaml
rollout:
  strategy: canary        # canary, percentage, all-at-once
  canary_count: 1          # machines in canary wave
  health_check: "curl -sf http://{{ machine.addr }}:8080/health"
  health_timeout: 30s
  percentage_steps: [25, 50, 100]
```

## Environment Diff

Compare effective configurations between environments to understand
the blast radius before promotion:

```
--- Diff: dev → prod ---
Total differences: 4
  param [replicas]: 1 → 4
  param [log_level]: "debug" → "warn"
  machine [web]: dev-web.internal → prod-web.internal
  machine [db]: dev-db.internal → prod-db.internal
```

## Example

```bash
cargo run --example environments
```
