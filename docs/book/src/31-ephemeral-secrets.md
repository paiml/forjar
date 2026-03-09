# Ephemeral Secrets

Forjar's ephemeral secrets system (FJ-3300) provides a secure pipeline for
using secrets during apply without persisting plaintext to state files. Secrets
are resolved from a provider chain, substituted into templates, and then
discarded -- only a BLAKE3 hash is stored, enabling drift detection without
cleartext exposure.

## Ephemeral Parameter Declaration

Declare ephemeral parameters in your configuration. Each parameter maps a
logical key to a secret provider key:

```yaml
version: "1.0"
name: secure-deploy
secrets:
  provider: env
  ephemeral: true

machines:
  web:
    hostname: web-01
    addr: 10.0.0.1

resources:
  db-config:
    type: file
    machine: web
    path: /etc/app/db.conf
    content: |
      host={{ params.db_host }}
      password={{ephemeral.db_password}}
      api_key={{ephemeral.api_key}}
```

When `ephemeral: true` is set, all resolved secret values are hashed before
writing to state. The plaintext value only exists in memory during the apply
operation.

## Secret Provider Chain

Ephemeral parameters resolve secrets through the same provider chain used by
`{{ secrets.* }}` templates. Providers are tried in registration order until
one resolves the key.

### env (default)

Reads from environment variables prefixed with `FORJAR_SECRET_`:

```bash
export FORJAR_SECRET_DB_PASSWORD=s3cret
export FORJAR_SECRET_API_KEY=sk-live-abc123
forjar apply -f forjar.yaml
```

The key `db_password` maps to `FORJAR_SECRET_DB_PASSWORD` (uppercase, hyphens
become underscores).

### file

Reads from files in a directory (default: `/run/secrets/`):

```yaml
secrets:
  provider: file
  path: /run/secrets
  ephemeral: true
```

Each secret is a file named after the key:

```
/run/secrets/db_password    -> contains "s3cret"
/run/secrets/api_key        -> contains "sk-live-abc123"
```

This works with Docker/Kubernetes secrets mounted as volumes, or systemd
`LoadCredential=`.

### exec

Runs a command to resolve each key:

```yaml
secrets:
  provider: exec
  command: vault kv get -field
  ephemeral: true
```

The command receives the key as an argument. It must print the secret value
to stdout and exit 0.

### Chain fallback

When multiple providers are configured, the chain tries each in order.
The first provider to return a value wins:

```yaml
secrets:
  providers:
    - env
    - file:
        path: /run/secrets
    - exec:
        command: vault kv get -field
  ephemeral: true
```

## BLAKE3 Hash-and-Discard Pattern

The core security property of ephemeral secrets is the hash-and-discard
pattern:

1. **Resolve** -- During `forjar apply`, the provider chain resolves the
   secret key to a plaintext value.
2. **Substitute** -- The plaintext value is substituted into templates and
   applied to target machines.
3. **Hash** -- The plaintext is hashed with BLAKE3, producing a 64-character
   hex string.
4. **Discard** -- The plaintext is dropped from memory. Only the hash is
   retained.

The hash marker format stored in state is:

```
EPHEMERAL[blake3:<64-hex-chars>]
```

BLAKE3 is a cryptographic hash function -- the plaintext cannot be recovered
from the hash. The 32-byte (256-bit) output provides collision resistance
equivalent to SHA-256 with significantly better performance.

## Template Substitution

Ephemeral values are substituted into template strings using the
`{{ephemeral.KEY}}` syntax:

```yaml
resources:
  app-env:
    type: file
    machine: web
    path: /etc/app/.env
    content: |
      DATABASE_URL=postgres://app:{{ephemeral.db_password}}@db:5432/app
      STRIPE_KEY={{ephemeral.stripe_key}}
      APP_PORT=8080
```

During apply, `{{ephemeral.db_password}}` is replaced with the resolved
plaintext value. After the template is rendered and applied, the plaintext
is discarded.

Unresolved `{{ephemeral.*}}` patterns are left as-is (not treated as errors),
allowing templates to be composed incrementally.

## Drift Detection via Hash Comparison

Even though plaintext is never stored, forjar detects when a secret changes
between applies:

1. On the next `forjar apply`, the secret is re-resolved from its provider.
2. The new plaintext is hashed with BLAKE3.
3. The new hash is compared against the stored `EPHEMERAL[blake3:...]` marker.

Three outcomes are possible:

| Status | Meaning |
|--------|---------|
| `Unchanged` | Hash matches -- the secret has not changed |
| `Changed` | Hash differs -- the secret was rotated or modified |
| `New` | No stored hash -- this is the first resolution |

When drift is detected (`Changed`), forjar re-applies the affected resources
with the new secret value.

**Important**: Drift detection requires the secret provider to be available.
If the provider is offline or the secret is removed, the apply will fail with
a resolution error rather than silently using stale data.

## State Records

The state file (`forjar.lock.yaml`) stores ephemeral records as hash-only
entries. No plaintext is ever written:

```yaml
outputs:
  db_password: "EPHEMERAL[blake3:fa08068884e33ec2c96cb4aabb2e14324...]"
  api_key: "EPHEMERAL[blake3:7a2b1c3d4e5f6a7b8c9d0e1f2a3b4c5d...]"
  app_port: "8080"
```

Non-secret outputs (like `app_port`) are stored in cleartext. Secret-looking
keys are also heuristically redacted even without `ephemeral: true` -- keys
containing `password`, `secret`, `token`, `key`, or `credential`
(case-insensitive) are automatically hashed.

The `EphemeralRecord` stored in state contains only two fields:

| Field | Description |
|-------|-------------|
| `key` | Parameter name |
| `hash` | BLAKE3 hex hash of the resolved value |

There is no `value` field -- the plaintext is structurally excluded from the
record type.

## Example YAML and CLI Usage

### Full configuration

```yaml
version: "1.0"
name: production-deploy
secrets:
  provider: env
  ephemeral: true

machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
  db:
    hostname: db-01
    addr: 10.0.0.2

resources:
  app-config:
    type: file
    machine: web
    path: /etc/app/config.yaml
    content: |
      database:
        host: {{ params.db_host }}
        password: {{ephemeral.db_password}}
      redis:
        url: redis://:{{ephemeral.redis_password}}@cache:6379

  tls-cert:
    type: file
    machine: web
    path: /etc/ssl/app.key
    content: "{{ephemeral.tls_private_key}}"
    mode: "0600"

outputs:
  db_password:
    value: "{{ephemeral.db_password}}"
    description: "Database password (ephemeral)"
  app_port:
    value: "8080"
    description: "Application port"
```

### CLI usage

```bash
# Set secrets in the environment
export FORJAR_SECRET_DB_PASSWORD="prod-db-pass-2026"
export FORJAR_SECRET_REDIS_PASSWORD="redis-auth-token"
export FORJAR_SECRET_TLS_PRIVATE_KEY="$(cat /path/to/private.key)"

# Apply -- secrets are used then discarded
forjar apply -f forjar.yaml

# Verify state contains only hashes
cat forjar.lock.yaml | grep EPHEMERAL
# db_password: "EPHEMERAL[blake3:fa0806...]"
# redis_password: "EPHEMERAL[blake3:9c2b17...]"

# Check for drift after rotating a secret
export FORJAR_SECRET_DB_PASSWORD="new-rotated-password"
forjar apply -f forjar.yaml
# Drift detected on db_password -- re-applies affected resources
```

### Encrypted state integrity

For encrypted state files, forjar provides BLAKE3 keyed hash verification
as an additional layer:

```bash
# The .b3hmac sidecar is written alongside encrypted state
ls forjar.lock.yaml.age forjar.lock.yaml.age.b3hmac

# Before decrypting, the sidecar is verified to detect tampering
forjar status -f forjar.yaml
# State integrity: verified (BLAKE3 keyed hash)
```

## API Reference

The ephemeral modules are at `forjar::core::ephemeral` and
`forjar::core::state::ephemeral`:

| Function | Module | Description |
|----------|--------|-------------|
| `resolve_ephemerals(params, chain)` | `core::ephemeral` | Resolve parameters via provider chain |
| `to_records(resolved)` | `core::ephemeral` | Convert to hash-only records for state |
| `check_drift(current, stored)` | `core::ephemeral` | Compare current hashes against stored |
| `substitute_ephemerals(template, resolved)` | `core::ephemeral` | Replace `{{ephemeral.KEY}}` in templates |
| `redact_to_hash(value)` | `core::state::ephemeral` | Replace value with BLAKE3 hash marker |
| `is_ephemeral_marker(s)` | `core::state::ephemeral` | Check if string is a hash marker |
| `extract_hash(marker)` | `core::state::ephemeral` | Get hex hash from marker |
| `verify_drift(value, marker)` | `core::state::ephemeral` | Check if value matches stored hash |
| `redact_outputs(map, force)` | `core::state::ephemeral` | Redact a full output map |
| `keyed_hash(data, key)` | `core::state::ephemeral` | BLAKE3 keyed hash for integrity |
| `derive_key(passphrase)` | `core::state::ephemeral` | Derive 32-byte key from passphrase |
| `write_hmac_sidecar(path, key)` | `core::state::ephemeral` | Write integrity sidecar |
| `verify_hmac_sidecar(path, key)` | `core::state::ephemeral` | Verify integrity sidecar |
