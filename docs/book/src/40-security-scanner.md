# Security Scanner & Access Control

Forjar includes a static security scanner that detects IaC security smells in recipe configurations, plus path deny policies and operator authorization for runtime enforcement.

## Security Scanner

The scanner implements 10 detection rules based on the IaC security smell taxonomy:

```rust
use forjar::core::security_scanner::{scan, severity_counts};
use forjar::core::types::ForjarConfig;

let config: ForjarConfig = /* parse recipe */;
let findings = scan(&config);
let (critical, high, medium, low) = severity_counts(&findings);

if critical > 0 {
    eprintln!("BLOCKED: {} critical security findings", critical);
    std::process::exit(1);
}
```

### Detection Rules

| Rule | Category | Severity | Detects |
|------|----------|----------|---------|
| SS-1 | Hard-coded secret | Critical | `password=`, `token=`, `api_key=`, `secret=`, `AWS_SECRET` in content |
| SS-2 | HTTP without TLS | High | `http://` URLs (except localhost) in source/target/content |
| SS-3 | World-accessible | High | File mode where last digit ≥ 4 (world read/write/execute) |
| SS-4 | Missing integrity | Medium | External file source with no integrity check |
| SS-5 | Privileged container | Critical/Medium | Docker `privileged=true` or running as root |
| SS-6 | No resource limits | Low | Docker without `MEMORY_LIMIT`/`CPU_LIMIT` |
| SS-7 | Weak crypto | High | References to md5, sha1, des, rc4, sslv3, tlsv1.0 |
| SS-8 | Insecure protocol | High | `telnet://`, `ftp://`, `rsh://` in content |
| SS-9 | Unrestricted network | Medium | Binding to `0.0.0.0` or `bind_address: *` |
| SS-10 | Sensitive data | Critical | PII patterns (`ssn=`, `credit_card=`) in content |

### Safe Pattern

```yaml
resources:
  config:
    type: file
    path: /etc/app/config.yaml
    mode: '0640'           # Not world-readable
    owner: app             # Not root
    content: |
      db_host: {{ secrets.db_host }}   # Template, not plaintext
      api_url: https://api.example.com # HTTPS, not HTTP
```

## Path Deny Policy

Block resource operations on sensitive filesystem paths:

```yaml
policy:
  deny_paths:
    - /etc/shadow
    - /etc/sudoers
    - /root/.ssh/*          # Glob match
    - /etc/sudoers.d/*
```

```rust
use forjar::core::types::PathPolicy;

let policy = PathPolicy {
    deny_paths: vec!["/etc/shadow".into(), "/root/.ssh/*".into()],
};
assert!(policy.is_denied("/etc/shadow"));        // Exact match
assert!(policy.is_denied("/root/.ssh/id_rsa"));  // Glob match
assert!(!policy.is_denied("/etc/nginx.conf"));   // Not denied
```

## Operator Authorization

Restrict which operators can apply to specific machines:

```yaml
machines:
  production-db:
    hostname: db-01.prod
    addr: 10.0.1.5
    user: deploy
    allowed_operators:
      - deploy-bot
      - senior-sre@company.com
```

Identity resolution order:
1. `--operator` CLI flag → `CliFlag`
2. `$USER@$(hostname)` → `Environment`
3. Git config → `GitConfig`

## Secret Management

Four provider backends for `{{ secrets.* }}` templates:

| Provider | Resolution | Config |
|----------|-----------|--------|
| `env` | `$FORJAR_SECRET_<NAME>` | Default |
| `file` | Read from secrets directory | `path: /run/secrets/` |
| `sops` | `sops -d` decryption | `file: secrets.enc.yaml` |
| `op` | 1Password CLI `op read` | — |

### Secret Audit Trail

All secret access is logged to `secret-audit.jsonl` with BLAKE3 hashes (never plaintext):

```json
{"timestamp":"2026-03-09T12:00:00Z","event_type":"resolve","key":"db_password","provider":"env","value_hash":"blake3:abc..."}
```

## Falsification

```bash
cargo run --example platform_security_falsification
```

Key invariants verified:
- SS-1 detects hardcoded passwords → Critical severity
- SS-3 detects mode 0777 → High severity
- SS-2 detects HTTP without TLS (but not localhost)
- Clean configs produce zero findings for SS-1/SS-3
- Path deny policy blocks exact and glob matches
- Empty policy denies nothing
- Operator flag overrides environment resolution
