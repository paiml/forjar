# Shell Providers & Script Security

## Shell Provider Bridge (FJ-3405)

Shell providers let you extend forjar with bash scripts. Each provider defines
three scripts — check, apply, destroy — validated by bashrs and scanned for
secret leakage before execution.

### Provider Structure

```
providers/
  nginx/
    provider.yaml
    check.sh
    apply.sh
    destroy.sh
```

### Manifest Format

```yaml
name: nginx
version: "1.0.0"
description: "Nginx configuration provider"
check: check.sh
apply: apply.sh
destroy: destroy.sh
```

### Usage in Resources

```yaml
resources:
  web-config:
    type: "shell:nginx"
    params:
      config_path: /etc/nginx/nginx.conf
      worker_connections: 1024
```

### Provider Scripts

Scripts receive parameters as environment variables prefixed with `FORJAR_`:

```bash
#!/bin/bash
set -euo pipefail

# check.sh — return 0 if converged, 1 if needs apply
if [ -f "$FORJAR_config_path" ]; then
    exit 0
fi
exit 1
```

## Script Secret Leakage Detection (FJ-3307)

All shell provider scripts are scanned for 14 secret leakage patterns before
execution. This catches common mistakes that would expose credentials.

### Detected Patterns

| Pattern | Example |
|---------|---------|
| `echo_secret_var` | `echo $PASSWORD` |
| `export_secret_inline` | `export TOKEN=abc` |
| `curl_inline_creds` | `curl -u admin:pass url` |
| `wget_inline_password` | `wget --password=x url` |
| `redirect_secret_to_file` | `$SECRET > file` |
| `sshpass_inline` | `sshpass -p pass ssh host` |
| `db_inline_password` | `mysql -ppassword` |
| `aws_key_in_script` | `AKIA...` hardcoded |
| `hardcoded_token` | `ghp_...` GitHub token |
| `hardcoded_stripe` | `sk_live_...` Stripe key |
| `private_key_inline` | `-----BEGIN RSA PRIVATE KEY-----` |
| `hex_secret_assign` | `SECRET=abcdef01234...` |
| `db_url_embedded_pass` | `postgres://user:pass@host` |

### Safe Patterns

Comments are skipped — `# echo $PASSWORD` does not trigger detection.

Use environment variable injection from forjar's ephemeral values instead
of hardcoding secrets:

```bash
#!/bin/bash
set -euo pipefail
# SAFE: use forjar-injected env vars
curl -H "Authorization: Bearer ${FORJAR_API_TOKEN}" https://api.example.com
```

## State File Rekey (FJ-3309)

Re-encrypt all state files with a new passphrase without exposing plaintext
to disk. BLAKE3 integrity verification at each step prevents data loss.

```bash
# Encrypt state files
forjar state-encrypt --passphrase "team-pass-2024"

# Rotate to new passphrase (annual key rotation)
forjar state-rekey \
  --old-passphrase "team-pass-2024" \
  --new-passphrase "team-pass-2025"

# Verify decryption with new key
forjar state-decrypt --passphrase "team-pass-2025"
```

### Integrity Guarantees

1. Old ciphertext HMAC verified before decryption
2. Plaintext hash compared after decryption
3. New ciphertext written atomically
4. New HMAC metadata stored alongside
5. Wrong old passphrase → error, no data modification
