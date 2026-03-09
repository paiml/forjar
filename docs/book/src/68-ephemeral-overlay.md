# Ephemeral Secrets & Overlay Layers

Falsification coverage for FJ-3300 and FJ-2103.

## Ephemeral Value Redaction (FJ-3300)

### Hash Redaction

Secret values are replaced with BLAKE3 hash markers before writing to state:

```rust
use forjar::core::state::ephemeral::{redact_to_hash, is_ephemeral_marker};

let marker = redact_to_hash("db-password-2026");
// → "EPHEMERAL[blake3:6fc1ecbf38b9...3d57]"
assert!(is_ephemeral_marker(&marker));
```

### Drift Detection

Re-resolve the secret and compare hashes to detect drift without storing cleartext:

```rust
use forjar::core::state::ephemeral::{redact_to_hash, verify_drift};

let marker = redact_to_hash("old-password");
assert!(verify_drift("old-password", &marker));   // no drift
assert!(!verify_drift("new-password", &marker));   // drift detected
```

### Output Redaction

Two modes — heuristic (detects secret-looking key names) and forced (all values):

```rust
use forjar::core::state::ephemeral::redact_outputs;

let mut outputs = indexmap::IndexMap::new();
outputs.insert("db_password".into(), "s3cret".into());
outputs.insert("data_dir".into(), "/var/data".into());

// Heuristic: only secret-looking keys redacted
let redacted = redact_outputs(&outputs, false);
// db_password → EPHEMERAL[blake3:...], data_dir → "/var/data"

// Force all: everything redacted
let redacted = redact_outputs(&outputs, true);
```

Secret key heuristic matches: `password`, `token`, `secret`, `key`, `credential`.

### Keyed Hashing (State Integrity)

```rust
use forjar::core::state::ephemeral::{derive_key, keyed_hash, verify_keyed_hash};

let key = derive_key("passphrase");
let hmac = keyed_hash(b"encrypted-state", &key);
assert!(verify_keyed_hash(b"encrypted-state", &key, &hmac));
```

## Overlay Layer Conversion (FJ-2103)

### Overlay Scanning

`scan_overlay_upper()` walks an overlayfs upper directory, producing layer entries and detecting whiteouts:

| Entry Type | Detection |
|-----------|-----------|
| Regular file | Normal file in upper dir |
| File deletion | `.wh.<name>` marker |
| Opaque directory | `.wh..wh..opq` marker |

### Whiteout → OCI Layer Entries

```rust
use forjar::core::store::overlay_export::whiteouts_to_entries;
use forjar::core::types::WhiteoutEntry;

let whiteouts = vec![
    WhiteoutEntry::FileDelete { path: "etc/old.conf".into() },
    WhiteoutEntry::OpaqueDir { path: "var/cache".into() },
];
let entries = whiteouts_to_entries(&whiteouts);
// → [".wh.old.conf", ".wh..wh..opq"]
```

`merge_overlay_entries()` combines regular entries with whiteout entries into a single layer set.

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_ephemeral_overlay.rs` | 39 | 401 |
