# Phase I: Security & Auditability (FJ-1356)

**Status**: ✅ Complete
**Implementation**: `src/core/store/secret_scan.rs`

---

## 1. Motivation

No plaintext secrets should ever appear in forjar configuration. All sensitive values must be encrypted with `ENC[age,...]` before they enter config YAML. The secret scanning framework provides defense-in-depth: even if a user forgets to encrypt, `forjar validate` will catch it.

## 2. Secret Detection

### 2.1 Regex Patterns

15 compiled patterns detect common secret types:

| Pattern | Regex | Example |
|---------|-------|---------|
| `aws_access_key` | `AKIA[0-9A-Z]{16}` | `AKIAIOSFODNN7EXAMPLE` |
| `aws_secret_key` | `(?i)aws_secret_access_key\s*[=:]\s*\S{20,}` | `aws_secret_access_key = wJalrXUt...` |
| `private_key_pem` | `-----BEGIN (RSA\|EC\|DSA\|OPENSSH) PRIVATE KEY-----` | PEM header |
| `github_token` | `gh[ps]_[A-Za-z0-9_]{36,}` | `ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` |
| `generic_api_key` | `(?i)(api[_-]?key\|apikey)\s*[=:]\s*['"]?\S{20,}` | `api_key: sk-proj-...` |
| `generic_secret` | `(?i)(secret\|password\|passwd\|token)\s*[=:]\s*['"]?\S{8,}` | `password: hunter2...` |
| `jwt_token` | `eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}` | `eyJhbGciOiJ...` |
| `slack_webhook` | `https://hooks\.slack\.com/services/T[A-Z0-9]+/B[A-Z0-9]+/[a-zA-Z0-9]+` | Slack webhook URL |
| `gcp_service_key` | `"type":\s*"service_account"` | GCP service account JSON |
| `stripe_key` | `[sr]k_(live\|test)_[A-Za-z0-9]{20,}` | `sk_live_...` |
| `database_url_pass` | `(?i)(mysql\|postgres\|mongodb)://[^:]+:[^@]+@` | `postgres://user:pass@host` |
| `base64_private` | `(?i)private.key.*=\s*[A-Za-z0-9+/]{40,}={0,2}` | Base64-encoded private key |
| `hex_secret_32` | `(?i)(secret\|key)\s*[=:]\s*[0-9a-f]{32,}` | 32+ char hex secret |
| `ssh_password` | `(?i)sshpass\s+-p\s+\S+` | `sshpass -p mypassword` |
| `age_plaintext` | `AGE-SECRET-KEY-1[A-Z0-9]{58}` | age identity key |

### 2.2 Encrypted Value Bypass

Values matching `ENC[age,...]` are excluded from scanning — they are properly encrypted and safe to store in config.

### 2.3 Scan Targets

The scanner walks all string fields in config YAML:
- `params.*` — template parameters
- `resource.*.pre_apply` / `resource.*.post_apply` — hook scripts
- `resource.*.content` — file resource content
- All other string values recursively

## 3. Integration

`forjar validate` calls `scan_config()` — any finding is a validation error. This ensures secrets are caught before they reach version control or the transport layer.

## 4. Redaction

Matched secrets are redacted in output: first 8 characters + `...`. This prevents the scanner itself from leaking secrets in logs.
