# Ephemeral Values and State Integrity

Forjar's ephemeral value system (FJ-3300) prevents secrets from persisting
in state files. When enabled, resolved secret values are replaced with BLAKE3
hashes before writing to state — enabling drift detection without cleartext
exposure.

## Enabling Ephemeral Mode

Add `ephemeral: true` to your secrets configuration:

```yaml
version: "1.0"
name: secure-deploy
secrets:
  provider: env
  ephemeral: true   # secrets never written to state in cleartext

machines:
  web:
    hostname: web-01
    addr: 10.0.0.1

outputs:
  db_password:
    value: "{{ secrets.db_password }}"
    description: "Database password (ephemeral)"
  app_port:
    value: "8080"
    description: "Application port"
```

## How It Works

1. **Resolve**: During `forjar apply`, secret templates like
   `{{ secrets.db_password }}` are resolved to cleartext values.
2. **Use**: The cleartext value is used in generated shell scripts and
   applied to target machines.
3. **Hash**: Before writing to state, the value is hashed with BLAKE3:
   `EPHEMERAL[blake3:<64-hex>]`
4. **Discard**: The cleartext value is never written to `forjar.lock.yaml`.

## State File Content

With ephemeral mode enabled, state files contain hash markers instead of
cleartext:

```yaml
outputs:
  db_password: "EPHEMERAL[blake3:fa08068884e33ec2c96cb4aabb2e143247fb6869...]"
  app_port: "EPHEMERAL[blake3:e7be8f38c5bcd72ecf2c3c8...]"
```

## Drift Detection

Even though cleartext isn't stored, forjar can detect when a secret changes:

1. On next apply, the secret is re-resolved from its provider
2. The new value is hashed with BLAKE3
3. If the hash differs from the stored marker, drift is detected

This means: **drift detection works, but replaying state requires the secret
provider to be available.**

## Heuristic Redaction

Even without `ephemeral: true`, forjar heuristically redacts output keys that
look like secrets. Keys containing `password`, `secret`, `token`, `key`, or
`credential` (case-insensitive) are automatically redacted.

## Encrypted State Integrity

For encrypted state files (`.age`), forjar provides BLAKE3 keyed hash
verification:

- After encrypting, a `.b3hmac` sidecar is written alongside the ciphertext
- The keyed hash uses BLAKE3's native keyed hash mode (not HMAC)
- Before decrypting, the sidecar is verified to detect tampering
- The key is derived from the encryption passphrase via BLAKE3

## Example

```bash
cargo run --example ephemeral_secrets
```

Output:
```
Original:  my-database-password-2026
Redacted:  EPHEMERAL[blake3:fa0806...]
Is marker: true

Same secret:    drift=false
Changed secret: drift=true
```

## API Reference

The ephemeral module is at `forjar::core::state::ephemeral`:

| Function | Description |
|----------|-------------|
| `redact_to_hash(value)` | Replace value with BLAKE3 hash marker |
| `is_ephemeral_marker(s)` | Check if string is a hash marker |
| `extract_hash(marker)` | Get hex hash from marker |
| `verify_drift(value, marker)` | Check if value matches stored hash |
| `redact_outputs(map, force)` | Redact a full output map |
| `keyed_hash(data, key)` | BLAKE3 keyed hash for integrity |
| `derive_key(passphrase)` | Derive 32-byte key from passphrase |
| `write_hmac_sidecar(path, key)` | Write integrity sidecar |
| `verify_hmac_sidecar(path, key)` | Verify integrity sidecar |
