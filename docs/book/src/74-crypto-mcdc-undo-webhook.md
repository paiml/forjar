# State Encryption, MC/DC, Undo Plans & Webhooks

Falsification coverage for FJ-3303, FJ-051, FJ-2003, and FJ-3104.

## State Encryption (FJ-3303)

BLAKE3-based encryption metadata for state file integrity:

```rust
use forjar::core::state_encryption::*;

let hash = hash_data(b"state yaml content");       // BLAKE3 hex hash
let key = derive_key("passphrase");                 // Key derivation
let hmac = keyed_hash(b"ciphertext", &key);         // Keyed HMAC
assert!(verify_keyed_hash(b"ciphertext", &key, &hmac));

let meta = create_metadata(b"plain", b"cipher", &key);
assert!(verify_metadata(&meta, b"cipher", &key));
```

File operations: `write_metadata`, `read_metadata`, `is_encrypted`, `list_encrypted`.

## MC/DC Analysis (FJ-051)

Modified Condition/Decision Coverage for DO-178C DAL-A:

```rust
use forjar::core::mcdc::*;

let d = build_decision("ready && approved", &["ready", "approved"]);
let report = generate_mcdc_and(&d);
assert_eq!(report.pairs.len(), 2);     // one pair per condition
assert_eq!(report.min_tests_needed, 3); // n+1

let or_report = generate_mcdc_or(&build_decision("a || b", &["a", "b"]));
assert!(or_report.coverage_achievable);
```

## Undo Plans (FJ-2003)

Generation-based rollback with irreversibility tracking:

```rust
use forjar::core::types::*;

let plan = UndoPlan { generation_from: 12, generation_to: 10, /* ... */ };
println!("{}", plan.format_summary());
assert!(!plan.has_irreversible());
```

`UndoProgress` tracks per-resource status (Pending/Completed/Failed) with resume support.

## Webhook Events (FJ-3104)

Request validation with HMAC signatures and event conversion:

```rust
use forjar::core::webhook_source::*;

let config = WebhookConfig::default(); // port 8484, /webhook
let result = validate_request(&config, &request);
assert!(result.is_valid());

let event = request_to_event(&request).unwrap();
// event.event_type == EventType::WebhookReceived
```

Validation checks: method (POST only), body size, allowed paths, HMAC signature.

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_crypto_mcdc_verus.rs` | 33 | ~320 |
| `falsification_undo_webhook.rs` | 35 | ~330 |
