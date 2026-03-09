# Plugin Hot-Reload & Webhook Events

## Plugin Hot-Reload (FJ-3406)

Forjar detects when WASM plugin files change on disk and automatically reloads them. This enables a fast development workflow where you can modify a plugin and see changes on the next `forjar apply` without restarting.

### How It Works

Every plugin invocation checks the BLAKE3 hash of the `.wasm` file against the cached hash from the last load. If the hash differs, the plugin is reloaded from disk.

```
┌─────────────────────────┐
│   forjar apply          │
│   type: plugin:my-app   │
└──────────┬──────────────┘
           │
           ▼
┌──────────────────────────┐
│   PluginCache            │
│   BLAKE3 hash check      │
│                          │
│   Cached hash == disk?   │
│   YES → use cached       │
│   NO  → reload + verify  │
└──────────────────────────┘
```

### ReloadCheck States

| State | Meaning |
|-------|---------|
| `UpToDate` | Plugin unchanged, use cached version |
| `Changed` | WASM file modified, reload needed |
| `NotCached` | Plugin not yet loaded |
| `FileGone` | WASM file deleted from disk |

### Example

```bash
cargo run --example plugin_hot_reload
```

The example demonstrates caching plugins, detecting file modifications, and reloading.

## Webhook Event Source (FJ-3104)

The webhook event source receives HTTP POST requests and converts them to `InfraEvent` values for the rules engine. This enables external systems (CI/CD, monitoring, ChatOps) to trigger forjar automation.

### Configuration

```yaml
# forjar-rules.yaml
event_sources:
  webhook:
    port: 8484
    secret: "hmac-shared-secret"  # optional HMAC-SHA256 validation
    max_body_bytes: 65536
    allowed_paths:
      - /webhook
      - /hooks/deploy
```

### Request Validation

All incoming requests pass through validation:

1. **Method check**: Only POST accepted
2. **Body size**: Rejects payloads exceeding `max_body_bytes`
3. **Path allowlist**: Only configured paths accepted
4. **HMAC signature**: If `secret` is set, requires valid `x-forjar-signature` header

### Sending a Webhook

```bash
# Simple trigger
curl -X POST http://localhost:8484/webhook \
  -H "Content-Type: application/json" \
  -d '{"action":"deploy","env":"production"}'

# With HMAC signature
SIG=$(echo -n '{"action":"deploy"}' | blake3 --keyed "$(echo -n 'secret' | blake3)")
curl -X POST http://localhost:8484/webhook \
  -H "Content-Type: application/json" \
  -H "x-forjar-signature: $SIG" \
  -d '{"action":"deploy"}'
```

### Event Payload

Webhook JSON body is flattened to key-value pairs in the `InfraEvent.payload`:

```json
{"action": "deploy", "env": "production"}
```

Becomes:

```
event_type: WebhookReceived
payload:
  action: deploy
  env: production
  _path: /webhook
  _source_ip: 10.0.0.1
```

### Example

```bash
cargo run --example webhook_source
```

## CIS Ubuntu 22.04 Compliance Pack (FJ-3206)

Forjar includes a built-in CIS Ubuntu 22.04 LTS compliance pack with 24 rules covering 6 CIS benchmark sections.

### Sections Covered

| Section | Rules | Examples |
|---------|-------|---------|
| 1. Filesystem | 4 | /tmp partition, file permissions |
| 2. Services | 5 | No telnet/xinetd, NFS disabled |
| 3. Network | 3 | IP forwarding, firewall, SYN cookies |
| 4. Access Controls | 4 | auditd, rsyslog, environment tags |
| 5. Authentication | 5 | SSH hardening, no world-writable files |
| 6. System Maintenance | 3 | File ownership, no duplicate users |

### Using the Pack

```bash
# Evaluate via API
cargo run --example cis_compliance

# Export pack as YAML
forjar policy export --pack cis-ubuntu-22.04
```

### Cross-Framework Mappings

Some CIS rules cross-map to other frameworks:

- **CIS-5.2.1** (SSH root login) maps to **STIG V-238196**
- All rules include CIS control IDs for audit trail

### Severity Distribution

- **14 error** rules (block apply with `--policy-check`)
- **9 warning** rules (logged, non-blocking)
- **1 info** rule (advisory only)
