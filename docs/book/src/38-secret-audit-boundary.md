# Secret Audit & Policy Boundary Testing

## Secret Access Audit Trail (FJ-3308)

Every secret access is logged to a JSONL audit trail for compliance and forensic analysis. Events track: who accessed the secret, which provider resolved it, timestamp, and the BLAKE3 hash (never plaintext).

### Event Types

| Event Type | Description |
|-----------|-------------|
| `resolve` | Secret resolved from provider |
| `inject` | Secret injected into namespace |
| `discard` | Secret cleared from memory |
| `rotate` | Secret key rotated to new value |

### Usage

```rust
use forjar::core::secret_audit::{
    append_audit, make_resolve_event, make_inject_event,
    make_discard_event, read_audit, audit_summary,
};

// Log a secret resolution
let event = make_resolve_event("db_pass", "env", &value_hash, Some("web-01"));
append_audit(state_dir, &event).unwrap();

// Log injection into namespace
let event = make_inject_event("db_pass", "env", &value_hash, "ns-forjar-1");
append_audit(state_dir, &event).unwrap();

// Read and analyze
let events = read_audit(state_dir).unwrap();
let summary = audit_summary(&events);
println!("Total: {} events, {} unique keys", summary.total, summary.unique_keys);
```

### Filtering

```rust
use forjar::core::secret_audit::{filter_by_key, filter_by_type, SecretEventType};

let db_events = filter_by_key(&events, "db_password");
let injects = filter_by_type(&events, &SecretEventType::Inject);
```

## Namespace Isolation (FJ-3306)

Secrets are injected into child processes via `env_clear()` — the parent environment is never contaminated. After execution, secrets are automatically discarded.

```rust
use forjar::core::secret_namespace::{NamespaceConfig, execute_isolated};

let config = NamespaceConfig {
    namespace_id: "ns-forjar-apply-1".into(),
    audit_enabled: true,
    state_dir: Some(state_dir.to_path_buf()),
    inherit_env: vec!["PATH".into()],
};

let result = execute_isolated(&config, &secrets, "sh", &["-c", script]).unwrap();
assert!(result.success);
assert_eq!(result.secrets_injected, secrets.len());
```

Key guarantees:
- Parent environment cleared (`env_clear()`)
- Only allowlisted variables inherited
- `FORJAR_NAMESPACE` marker set in child
- Audit events logged for inject and discard

## Policy Boundary Testing (FJ-3209)

Boundary testing verifies that policy rules are non-vacuous: every deny rule rejects at least one config, every assert rule passes on golden configs.

```rust
use forjar::core::policy_boundary::{test_boundaries, format_boundary_results};
use forjar::core::cis_ubuntu_pack::cis_ubuntu_2204_pack;

let pack = cis_ubuntu_2204_pack();
let result = test_boundaries(&pack);
println!("{}", format_boundary_results(&result));
// Result: 48/48 boundary tests passed
```

For each rule, two configs are generated:
- **Golden**: satisfies the rule (expected: pass)
- **Boundary**: violates the rule (expected: fail)

If a boundary config passes when it should fail, the rule is vacuous.

## Running the Examples

```bash
cargo run --example secret_audit
cargo run --example secret_namespace
cargo run --example policy_boundary
```
