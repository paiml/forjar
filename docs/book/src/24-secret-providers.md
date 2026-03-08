# Secret Providers

Forjar resolves `{{ secrets.* }}` template variables through pluggable secret providers. Configure the provider in your `forjar.yaml`:

```yaml
secrets:
  provider: env   # env | file | sops | op
```

## Provider: `env` (default)

Reads secrets from environment variables prefixed with `FORJAR_SECRET_`:

```yaml
secrets:
  provider: env

resources:
  db-config:
    type: file
    machine: web
    path: /etc/app/db.conf
    content: |
      host={{ params.db_host }}
      password={{ secrets.db_password }}
```

```bash
export FORJAR_SECRET_DB_PASSWORD=s3cret
forjar apply -f forjar.yaml
```

The key `db_password` maps to `FORJAR_SECRET_DB_PASSWORD` (uppercase, hyphens become underscores).

## Provider: `file`

Reads secrets from files in a directory (default: `/run/secrets/`):

```yaml
secrets:
  provider: file
  path: /run/secrets    # optional, this is the default
```

Each secret is a file named after the key:

```
/run/secrets/db_password    → contains "s3cret"
/run/secrets/api_key        → contains "sk-live-abc123"
```

This works well with Docker/Kubernetes secrets mounted as volumes, or systemd `LoadCredential=`.

## Provider: `sops`

Decrypts secrets from a SOPS-encrypted file using Mozilla SOPS:

```yaml
secrets:
  provider: sops
  file: secrets.enc.yaml    # optional, default: secrets.enc.yaml
```

The encrypted file contains your secrets:

```yaml
# secrets.enc.yaml (encrypted with sops)
db_password: ENC[AES256_GCM,...]
api_key: ENC[AES256_GCM,...]
```

Forjar runs `sops -d --extract '["<key>"]' <file>` for each secret reference. SOPS supports AWS KMS, GCP KMS, Azure Key Vault, age, and PGP as key management backends.

### Prerequisites

- Install `sops`: <https://github.com/getsops/sops>
- Configure `.sops.yaml` with your key management backend

## Provider: `op` (1Password)

Resolves secrets via the 1Password CLI:

```yaml
secrets:
  provider: op
  path: my-vault    # optional, default: forjar
```

Forjar runs `op read "op://<vault>/<key>"` for each secret reference. The vault defaults to `forjar` if not specified.

### Prerequisites

- Install `op` CLI: <https://developer.1password.com/docs/cli/>
- Sign in: `eval $(op signin)`

### Example

```yaml
secrets:
  provider: op
  path: production

resources:
  app-env:
    type: file
    machine: web
    path: /etc/app/.env
    content: |
      DATABASE_URL=postgres://app:{{ secrets.db_password }}@db:5432/app
      STRIPE_KEY={{ secrets.stripe_api_key }}
```

## Age Encryption (FJ-200)

Separately from secret providers, forjar supports inline age encryption with `ENC[age,...]` markers:

```bash
# Generate a keypair
forjar secrets keygen

# Encrypt a value
forjar secrets encrypt --recipient age1... "my-secret-value"

# Use in config
resources:
  config:
    type: file
    content: "password=ENC[age,YWdlLWVuY3J5cHRpb24...]"
```

Age-encrypted values are decrypted AFTER template resolution, so you can combine both:

```yaml
secrets:
  provider: env

resources:
  config:
    type: file
    content: |
      env_secret={{ secrets.api_key }}
      encrypted_secret=ENC[age,YWdlLWVuY3J5cHRpb24...]
```

## Secret Scanning

Forjar can detect hardcoded secrets in your configuration:

```bash
forjar validate --check-secrets -f forjar.yaml
```

This scans for patterns like AWS keys, JWT tokens, private keys, and database URLs. Properly templated secrets (`{{ secrets.* }}`) and age-encrypted values (`ENC[age,...]`) are not flagged.

## Redaction

Secret values are automatically redacted from log output. Any resolved secret value is replaced with `***` in apply logs and event output.

## Example

```bash
cargo run --example secret_providers
```
