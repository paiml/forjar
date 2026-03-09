# Cron Source, Ephemeral Secrets & State Encryption

Forjar provides in-process cron scheduling, ephemeral value pipelines with hash-and-discard semantics, state file encryption with BLAKE3 integrity, and script secret leakage detection.

## Cron Event Source (FJ-3103)

In-process cron expression parser for the `forjar watch` daemon. No system crontab dependency — purely in-process evaluation.

### Cron Expression Format

Standard 5-field format: `minute hour dom month dow`

| Syntax | Meaning | Example |
|--------|---------|---------|
| `*` | Every value | `* * * * *` (every minute) |
| `N` | Exact value | `30 12 * * *` (12:30 daily) |
| `N-M` | Range | `9-17` (9am to 5pm) |
| `*/N` | Step | `*/15` (every 15 units) |
| `A,B,C` | List | `1,15` (1st and 15th) |

### Usage

```rust
use forjar::core::cron_source::{parse_cron, matches, CronTime};

let schedule = parse_cron("*/15 9-17 * * 1-5").unwrap();
let now = CronTime { minute: 30, hour: 10, day: 15, month: 3, weekday: 1 };
if matches(&schedule, &now) {
    println!("Schedule fires now");
}
```

### Field Ranges

| Field | Min | Max |
|-------|-----|-----|
| Minute | 0 | 59 |
| Hour | 0 | 23 |
| Day of month | 1 | 31 |
| Month | 1 | 12 |
| Day of week | 0 (Sun) | 6 (Sat) |

## Script Secret Lint (FJ-3307)

Scans shell scripts for secret leakage patterns before execution.

### Detected Patterns

| Pattern | Example |
|---------|---------|
| `echo_secret_var` | `echo $PASSWORD` |
| `export_secret_inline` | `export TOKEN=abc` |
| `curl_inline_creds` | `curl -u admin:pass url` |
| `wget_inline_password` | `wget --password=x url` |
| `redirect_secret_to_file` | `$SECRET > file` |
| `sshpass_inline` | `sshpass -p pass ssh host` |
| `db_inline_password` | `mysql -psecret` |
| `aws_key_in_script` | `AKIAIOSFODNN7EXAMPLE` |
| `hardcoded_token` | `ghp_ABCDEF...` |
| `hardcoded_stripe` | `sk_live_...` |
| `private_key_inline` | `-----BEGIN RSA PRIVATE KEY-----` |
| `hex_secret_assign` | `SECRET=abcdef012345...` |
| `db_url_embedded_pass` | `postgres://user:pass@host` |

### Usage

```rust
use forjar::core::script_secret_lint::{scan_script, validate_no_leaks};

let result = scan_script("echo $PASSWORD\ncurl -u admin:pass url\n");
if !result.clean() {
    for finding in &result.findings {
        println!("line {}: [{}] {}", finding.line, finding.pattern_name, finding.matched_text);
    }
}

// Or use the validation helper
validate_no_leaks(script)?;
```

Comment lines (starting with `#`) are skipped. Matched text is redacted (truncated to 12 chars).

## State Encryption (FJ-3303)

BLAKE3-based encryption metadata and integrity verification for state files. Sovereign — no cloud KMS dependency.

### Key Derivation

```rust
use forjar::core::state_encryption::{derive_key, hash_data, keyed_hash, verify_keyed_hash};

let key = derive_key("my-passphrase"); // 32-byte BLAKE3 derived key
let hash = hash_data(b"state content"); // BLAKE3 hash (64 hex chars)
let hmac = keyed_hash(b"ciphertext", &key); // Keyed HMAC
assert!(verify_keyed_hash(b"ciphertext", &key, &hmac));
```

### Encryption Metadata

Sidecar `.enc.meta.json` files store:

- `version`: Schema version (currently 1)
- `plaintext_hash`: BLAKE3 hash of original state
- `ciphertext_hmac`: Keyed HMAC of encrypted data
- `encrypted_at`: ISO 8601 timestamp

```rust
use forjar::core::state_encryption::{create_metadata, verify_metadata, write_metadata};

let meta = create_metadata(plaintext, ciphertext, &key);
write_metadata(&state_path, &meta)?;
assert!(verify_metadata(&meta, ciphertext, &key));
```

## Ephemeral Values (FJ-3302)

Resolve → use → discard pipeline for secrets. Only BLAKE3 hashes are stored in state for drift detection.

### Pipeline

1. **Resolve**: Fetch secret from provider chain (env, file, exec)
2. **Substitute**: Replace `{{ephemeral.KEY}}` in templates
3. **Hash**: Compute BLAKE3 hash of plaintext
4. **Discard**: Store only `EphemeralRecord { key, hash }` — no plaintext

### Drift Detection

```rust
use forjar::core::ephemeral::{check_drift, DriftStatus};

let drift = check_drift(&current_resolved, &stored_records);
for d in &drift {
    match d.status {
        DriftStatus::Unchanged => println!("{}: no change", d.key),
        DriftStatus::Changed => println!("{}: SECRET ROTATED", d.key),
        DriftStatus::New => println!("{}: new secret", d.key),
    }
}
```

## Falsification

```bash
cargo test --test falsification_cron_source
cargo test --test falsification_script_secret_lint
cargo test --test falsification_state_encryption
cargo test --test falsification_ephemeral_secrets
```

Key invariants verified:
- Cron parsing: all syntax variants, boundary values, error rejection
- Cron matching: all 5 fields checked, step hit/miss, weekday ranges
- Secret lint: all 13 patterns detected, comments skipped, redaction applied
- BLAKE3: deterministic hashing, keyed HMAC tamper detection
- Key derivation: deterministic, different passphrases produce different keys
- Metadata: version 1, plaintext hash, ciphertext HMAC, sidecar I/O
- Ephemeral: hash-only records, drift detection (unchanged/changed/new)
- Template substitution: single/multiple/repeated/no-match cases
