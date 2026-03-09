# Docker Migration & Webhook Events

Falsification coverage for FJ-044 and FJ-3104.

## Docker → Pepita Migration (FJ-044)

Converts Docker container resources to pepita kernel isolation resources.

### Field Mapping

| Docker | Pepita | Notes |
|--------|--------|-------|
| `image` | (cleared) | Extract rootfs for `overlay_lower` |
| `ports` | `netns: true` | Manual iptables/nftables needed |
| `volumes` | (cleared) | Use bind mounts or overlay dirs |
| `environment` | (cleared) | Set in chroot `/etc/environment` |
| `restart` | (cleared) | Use systemd unit instead |
| `state: running` | `state: present` | |
| `state: stopped` | `state: absent` | Warning emitted |
| `state: absent` | `state: absent` | |
| `name` | `name` | Preserved |
| `depends_on` | `depends_on` | Preserved |

### Single Resource

```rust
use forjar::core::migrate::docker_to_pepita;

let result = docker_to_pepita("web", &docker_resource);
assert_eq!(result.resource.resource_type, ResourceType::Pepita);
for w in &result.warnings {
    eprintln!("  {w}");
}
```

### Full Config

```rust
use forjar::core::migrate::migrate_config;

let (migrated, warnings) = migrate_config(&config);
// Only Docker resources are converted; others unchanged
```

## Webhook Event Source (FJ-3104)

### Request Validation

```rust
use forjar::core::webhook_source::{validate_request, WebhookConfig};

let config = WebhookConfig {
    secret: Some("deploy-secret".into()),
    allowed_paths: vec!["/hooks/deploy".into()],
    ..WebhookConfig::default()
};
let result = validate_request(&config, &request);
assert!(result.is_valid());
```

Validation checks (in order):
1. Method must be POST
2. Body size within `max_body_bytes`
3. Path in `allowed_paths`
4. HMAC-SHA256 signature (if `secret` configured)

### HMAC Signatures

Uses BLAKE3 keyed hash for request authentication:

```rust
use forjar::core::webhook_source::compute_hmac_hex;

let sig = compute_hmac_hex("secret", body);
// Set as x-forjar-signature header
```

### Payload Parsing

```rust
use forjar::core::webhook_source::{parse_json_payload, request_to_event};

let payload = parse_json_payload(body)?;
let event = request_to_event(&request)?;
assert_eq!(event.event_type, EventType::WebhookReceived);
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_migrate_webhook.rs` | 29 | 458 |
