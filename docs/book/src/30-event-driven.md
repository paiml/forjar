# Event-Driven Automation

Forjar's event-driven engine (FJ-3100) enables reactive infrastructure
convergence — rulebooks that automatically respond to file changes,
process exits, cron schedules, webhooks, and metric thresholds.

## Event Types

| Event | Description | Example |
|-------|-------------|---------|
| `file_changed` | File system change (inotify/fanotify) | Config file modified |
| `process_exit` | Process exited (waitpid) | Service crashed |
| `cron_fired` | Cron schedule fired | Nightly cleanup |
| `webhook_received` | HTTP webhook received | GitHub push event |
| `metric_threshold` | Metric threshold crossed | CPU > 90% |
| `manual` | Manual trigger (`forjar trigger`) | Operator action |

## Rulebooks

A rulebook maps events to actions. When an event matches a rulebook's
patterns, the configured actions execute automatically.

```yaml
name: config-repair
description: "Auto-repair nginx config drift"
events:
  - type: file_changed
    match:
      path: /etc/nginx/nginx.conf
actions:
  - apply:
      file: forjar.yaml
      tags: [config]
cooldown_secs: 30
max_retries: 5
```

### Rulebook Fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | Yes | — | Unique identifier |
| `description` | No | — | Human-readable description |
| `events` | Yes | — | Event patterns to match |
| `actions` | Yes | — | Actions to execute |
| `conditions` | No | `[]` | Template expressions that must be true |
| `cooldown_secs` | No | `30` | Minimum seconds between activations |
| `max_retries` | No | `3` | Maximum retry attempts |
| `enabled` | No | `true` | Whether the rulebook is active |

## Event Patterns

Patterns match events by type and optional payload fields:

```yaml
events:
  # Match any file change
  - type: file_changed

  # Match specific file path
  - type: file_changed
    match:
      path: /etc/nginx/nginx.conf

  # Match process exit with specific code
  - type: process_exit
    match:
      exit_code: "137"
      process: nginx
```

All `match` fields must be present in the event's payload with matching
values. If no `match` fields are specified, any event of the given type
matches.

## Actions

Four action types are supported:

### Apply

Run `forjar apply` on a subset of resources:

```yaml
actions:
  - apply:
      file: forjar.yaml
      tags: [config]
      subset: [nginx-config]
      machine: web-01
```

### Destroy

Remove resources:

```yaml
actions:
  - destroy:
      file: forjar.yaml
      resources: [temp-cache]
```

### Script

Run a shell command:

```yaml
actions:
  - script: "forjar apply -f cleanup.yaml --tags temp"
```

### Notify

Send a notification:

```yaml
actions:
  - notify:
      channel: "https://hooks.slack.com/services/abc"
      message: "Config drift detected on {{machine}}"
```

## Cooldown Deduplication

Cooldowns prevent rapid re-triggering. After a rulebook fires, it won't
fire again until the cooldown period expires:

```yaml
cooldown_secs: 60  # At most once per minute
```

Each rulebook tracks its cooldown independently. A cooldown of `0`
disables deduplication entirely.

## Multi-Rulebook Configuration

Define multiple rulebooks in a single config:

```yaml
rulebooks:
  - name: config-repair
    events:
      - type: file_changed
        match:
          path: /etc/nginx/nginx.conf
    actions:
      - apply:
          file: forjar.yaml
          tags: [config]

  - name: deploy-notify
    events:
      - type: manual
    actions:
      - notify:
          channel: "https://hooks.slack.com/abc"
          message: "Manual deployment triggered"

  - name: cleanup-cron
    events:
      - type: cron_fired
    actions:
      - script: "forjar apply -f cleanup.yaml"
```

## Example

```bash
cargo run --example event_rulebook
```
