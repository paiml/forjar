# Event-Driven Runtime

## Rulebook Runtime Evaluator (FJ-3106)

The runtime evaluator processes infrastructure events against configured
rulebooks with cooldown deduplication and pattern matching.

### Architecture

```
[Events] → [Pattern Match] → [Cooldown Check] → [Action Dispatch]
   ↑                              ↑
   └── file_changed              └── CooldownTracker
       process_exit                   (per-rulebook)
       cron_fired
       webhook_received
       metric_threshold
       manual
```

### Cooldown Deduplication

Each rulebook has a `cooldown_secs` setting. After firing, the rulebook
is blocked for that duration — preventing rapid-fire triggering from
flapping resources.

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
          subset: [nginx-config]
    cooldown_secs: 30     # Block for 30s after firing
    max_retries: 3
```

### Evaluation Flow

1. **Event arrives** (file change, cron, webhook, etc.)
2. **Pattern matching**: event type + payload fields matched against each rulebook
3. **Enabled check**: disabled rulebooks are skipped
4. **Cooldown check**: recently-fired rulebooks are blocked
5. **Action dispatch**: matched + unfired rulebooks return their action lists

### Payload Matching

Events can carry key-value payloads. Rulebook patterns can require specific
payload values:

```yaml
events:
  - type: file_changed
    match:
      path: /etc/nginx/nginx.conf    # Only this specific file
      operation: modify              # Only modifications, not creates
```

All match fields must be present in the event payload with matching values.

### Action Types

| Type | Description |
|------|-------------|
| `apply` | Run `forjar apply` on a subset of resources |
| `destroy` | Remove specific resources |
| `script` | Execute a shell script |
| `notify` | Send a webhook notification |

### Runtime Summary

```bash
# Check runtime state
forjar rules coverage -f forjar-rules.yaml

# Evaluate events
forjar trigger config-repair  # Manual trigger
```

## Promotion Event Logging (FJ-3509)

All promotion operations are logged to `events.jsonl` with structured
provenance events.

### Event Types

**PromotionCompleted**: Logged when a promotion succeeds or fails.

```json
{
  "ts": "2026-03-09T12:00:00Z",
  "event": "promotion_completed",
  "source": "dev",
  "target": "staging",
  "success": true,
  "gates_passed": 3,
  "gates_total": 3,
  "rollout_strategy": "canary"
}
```

**RollbackTriggered**: Logged when health checks fail during rollout.

```json
{
  "ts": "2026-03-09T12:05:00Z",
  "event": "rollback_triggered",
  "environment": "prod",
  "failed_step": 2,
  "reason": "canary health check failed: 503"
}
```

### Event Log Location

Events are stored per-environment:

```
state/
  staging/events.jsonl    # Staging promotion events
  prod/events.jsonl       # Production promotion + rollback events
```

Each line is a self-contained JSON object with ISO 8601 timestamp,
enabling `jq` queries for promotion auditing:

```bash
# Find all failed promotions to prod
jq 'select(.event == "promotion_completed" and .success == false)' \
  state/prod/events.jsonl

# Find rollbacks in the last 24 hours
jq 'select(.event == "rollback_triggered")' \
  state/prod/events.jsonl
```
