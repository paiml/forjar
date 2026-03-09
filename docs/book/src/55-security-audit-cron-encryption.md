# Security Scanner, Audit Trail, Cron & State Encryption

Falsification coverage for FJ-1390, FJ-3307, FJ-3308, FJ-3103, FJ-202, FJ-3303, and FJ-3507.

## Security Scanner (FJ-1390)

Static IaC security scanning with 10 rule categories:

| Rule | Severity | Detects |
|------|----------|---------|
| SS-1 | Critical | Hardcoded secrets (`password=`, `secret=`, etc.) |
| SS-2 | High | HTTP URLs without TLS (localhost exempt) |
| SS-3 | High | World-accessible file modes (last digit >= 4) |
| SS-4 | Medium | External sources without integrity checks |
| SS-5 | Medium | Privileged containers |
| SS-6 | Low | Missing resource limits |
| SS-7 | Medium | Weak cryptographic algorithms |
| SS-8 | High | Insecure protocols (FTP, Telnet) |
| SS-9 | Medium | Unrestricted network binding (0.0.0.0) |
| SS-10 | Medium | Sensitive data in content fields |

```rust
use forjar::core::security_scanner::{scan, severity_counts};

let findings = scan(&config);
let (critical, high, medium, low) = severity_counts(&findings);
```

### Script Secret Lint (FJ-3307)

13 regex patterns detect secret leakage in shell scripts:

```rust
use forjar::core::script_secret_lint::{scan_script, validate_no_leaks};

let result = scan_script("echo $DB_PASSWORD");
assert!(!result.clean());

// Gate: reject scripts with leaks
validate_no_leaks("safe_script.sh").unwrap();
```

Patterns include: `echo_secret_var`, `curl_inline_creds`, `aws_key_in_script`,
`hardcoded_token`, `private_key_inline`, `db_url_embedded_pass`, and more.
Comments (lines starting with `#`) are skipped.

## Secret Audit Trail (FJ-3308)

JSONL-persisted audit log for secret lifecycle events:

```rust
use forjar::core::secret_audit::*;

// Four event types
let e1 = make_resolve_event("db_pass", "env", "blake3:abc", Some("web-01"));
let e2 = make_inject_event("db_pass", "env", "blake3:abc", "ns-apply-1");
let e3 = make_discard_event("db_pass", "blake3:abc");
let e4 = make_rotate_event("api_key", "file", "blake3:old", "blake3:new");

// Persistence roundtrip
append_audit(state_dir, &e1).unwrap();
let events = read_audit(state_dir).unwrap();

// Filtering and aggregation
let db_events = filter_by_key(&events, "db_pass");
let summary = audit_summary(&events);
println!("{}", format_audit_summary(&summary));
```

Rejection criteria tested:
- Event construction preserves all fields (key, provider, value_hash, machine, pid, timestamp)
- Rotate events track `rotated_from:{old_hash}` in namespace
- Discard events have empty provider
- JSONL roundtrip: each line is valid JSON
- `filter_by_key` / `filter_by_type` select correctly
- `audit_summary` counts unique keys and providers (empty provider excluded)

## Cron Source (FJ-3103)

Five-field cron expression parsing and schedule matching:

```rust
use forjar::core::cron_source::{parse_cron, matches, schedule_summary, CronTime};

let schedule = parse_cron("0 9 * * 1-5").unwrap();  // 9am weekdays
let monday_9am = CronTime { minute: 0, hour: 9, day: 10, month: 3, weekday: 1 };
assert!(matches(&schedule, &monday_9am));
```

Supports: `*` (wildcard), `*/N` (step), `A-B` (range), `A,B,C` (list), and combinations.

## Conditional Evaluation (FJ-202)

`when:` expression evaluation for resource conditions:

```rust
use forjar::core::conditions::evaluate_when;

// Supports ==, !=, contains operators
let result = evaluate_when(
    "{{machine.arch}} == x86_64",
    &params, &machine
).unwrap();
```

Machine fields available: `arch`, `hostname`, `addr`, `user`, `roles`.

## State Encryption (FJ-3303)

BLAKE3 hashing and keyed HMAC for state integrity:

```rust
use forjar::core::state_encryption::*;

let key = derive_key("my-passphrase");           // 32-byte key
let hash = hash_data(b"plaintext");              // BLAKE3 hex (64 chars)
let hmac = keyed_hash(b"ciphertext", &key);      // Keyed HMAC
assert!(verify_keyed_hash(b"ciphertext", &key, &hmac));

// Metadata roundtrip with tamper detection
let meta = create_metadata(b"plain", b"cipher", &key);
assert!(verify_metadata(&meta, b"cipher", &key));
assert!(!verify_metadata(&meta, b"tampered", &key));

// Filesystem persistence
write_metadata(&path, &meta).unwrap();
let loaded = read_metadata(&path).unwrap();
```

## Progressive Rollout (FJ-3507)

Three deployment strategies:

| Strategy | Behavior |
|----------|----------|
| `canary` | Deploy to `canary_count` machines first, then percentage steps |
| `percentage` | Deploy in percentage increments (e.g., 25%, 50%, 100%) |
| `all-at-once` | Deploy to all machines in a single step |

```rust
use forjar::core::rollout::plan_rollout;

let config = RolloutConfig {
    strategy: "canary".into(),
    canary_count: 1,
    percentage_steps: vec![50, 100],
    ..Default::default()
};
let steps = plan_rollout(&config, 10);
assert_eq!(steps[0].machine_indices.len(), 1); // canary first
```

## Running the Example

```bash
cargo run --example security_audit_cron
```

Demonstrates all four subsystems end-to-end with assertion-guarded output.

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_cron_conditions_scanner.rs` | 41 | 500 |
| `falsification_audit_encrypt_rollout.rs` | 30 | 403 |
| **Total** | **71** | **903** |
