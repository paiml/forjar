# Store Metadata, Secret Scanning & Garbage Collection

Falsification coverage for FJ-1301 (store metadata), FJ-1356 (secret scanning), and FJ-1325 (garbage collection).

## Store Metadata (FJ-1301)

Every store entry has a `meta.yaml` with provenance tracking:

```rust
use forjar::core::store::meta::{new_meta, write_meta, read_meta, Provenance};

let meta = new_meta("blake3:abc", "blake3:recipe", &["blake3:in1".into()], "x86_64", "apt");
assert_eq!(meta.schema, "1.0");
assert!(meta.generator.starts_with("forjar"));

// Atomic write + read roundtrip
write_meta(&entry_dir, &meta).unwrap();
let read = read_meta(&entry_dir).unwrap();
assert_eq!(meta, read);

// Optional provenance chain
meta.provenance = Some(Provenance {
    origin_provider: "cargo".into(),
    origin_ref: Some("crates.io/serde".into()),
    origin_hash: Some("sha256:abc".into()),
    derived_from: None,
    derivation_depth: 0,
});
meta.references = vec!["blake3:ref1".into()];
```

### StoreMeta Fields

| Field | Type | Description |
|-------|------|-------------|
| `schema` | String | Always "1.0" |
| `store_hash` | String | BLAKE3 hash of store entry |
| `recipe_hash` | String | Hash of build recipe |
| `input_hashes` | Vec | Hashes of all inputs |
| `arch` | String | Target architecture |
| `provider` | String | Package provider (apt, cargo, nix) |
| `created_at` | String | ISO 8601 timestamp |
| `generator` | String | "forjar {version}" |
| `references` | Vec | Other store entries this depends on |
| `provenance` | Option | Origin tracking chain |

## Secret Scanning (FJ-1356)

15 regex patterns detect leaked secrets with age encryption bypass:

```rust
use forjar::core::store::secret_scan::{is_encrypted, scan_text, scan_yaml_str};

// Detects AWS keys, GitHub tokens, private keys, Stripe keys, etc.
assert_eq!(scan_text("AKIAIOSFODNN7EXAMPLE").len(), 1);
assert!(scan_text("ghp_AAAA...").iter().any(|f| f.0 == "github_token"));

// Encrypted values are skipped
assert!(is_encrypted("ENC[age,data]"));
assert!(scan_text("ENC[age,AKIAIOSFODNN7EXAMPLE]").is_empty());

// YAML scanning walks nested structures
let result = scan_yaml_str("db:\n  password: AKIAIOSFODNN7EXAMPLE\n");
assert!(!result.clean);
assert!(result.findings[0].location.contains("db"));
```

### Detected Patterns

| Pattern | Example |
|---------|---------|
| `aws_access_key` | AKIA... (20 chars) |
| `aws_secret_key` | 40-char base64 after key identifier |
| `private_key_pem` | -----BEGIN RSA PRIVATE KEY----- |
| `github_token` | ghp_ + 40 alphanumeric |
| `stripe_key` | sk_live_ + 24 alphanumeric |
| `generic_api_key` | api_key/apikey assignment |
| `jwt_token` | eyJ... bearer tokens |
| `slack_webhook` | hooks.slack.com/services/ URLs |
| `gcp_service_key` | "type": "service_account" |
| `database_url_pass` | ://user:pass@ connection strings |

## Garbage Collection (FJ-1325)

Mark-and-sweep GC from profile and lockfile roots:

```rust
use forjar::core::store::gc::{collect_roots, mark_and_sweep, GcConfig};

// Collect roots from profiles, lockfiles, gc-roots dir
let roots = collect_roots(&profiles, &locks, None);
// Deduplicates across sources

// Mark-and-sweep follows meta.yaml references
let report = mark_and_sweep(&roots, store_dir).unwrap();
assert!(report.live.contains(&"blake3:root_hash".into()));
assert!(report.dead.contains(&"blake3:orphan_hash".into()));

// Default config
let config = GcConfig::default();
assert_eq!(config.keep_generations, 5);
assert!(config.older_than_days.is_none());
```

### GC Algorithm

1. **Collect roots** from active profiles, lockfiles, and `/var/lib/forjar/gc-roots/`
2. **BFS traversal** following `references` in each entry's `meta.yaml`
3. **Mark** all reachable entries as live
4. **Report** unreachable entries as dead (actual deletion is separate)

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_meta_secret_gc.rs` | 23 | ~256 |
