# 10: Security Model

> Authorization, path restrictions, secret management, and privilege boundaries.

**Spec ID**: FJ-2300 | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Threat Model

Forjar executes scripts on remote machines via SSH, runs commands in privileged namespaces (pepita), and stores state files containing resource details. The attack surface:

| Threat | Vector | Impact |
|--------|--------|--------|
| Malicious config | PR adds `type: file, path: /etc/cron.d/backdoor, content: "..."` | Arbitrary file write on target machine |
| Secret leakage | `content:` field with credentials → stored in lock files, events, FTS5 | Credential exposure to anyone with state directory access |
| Privilege escalation | pepita uses `unshare(2)` which requires `CAP_SYS_ADMIN` or user namespace support | Root-equivalent on host |
| State tampering | Attacker modifies `state.lock.yaml` to fake convergence | Planner skips re-apply, machine stays misconfigured |
| Transport MITM | SSH ControlMaster reuses connections — if initial auth is compromised, all subsequent commands are | Full machine compromise |

---

## Authorization

### Machine-Level Access Control

```yaml
# In forjar config YAML
machines:
  production-db:
    hostname: db.internal
    addr: 10.0.1.5
    allowed_operators:
      - "noah"
      - "ci-bot"
    # If unset: any operator can apply (current behavior)
```

```
fn check_authorization(machine, operator):
    if machine.allowed_operators.is_empty():
        return Ok(())  // no restriction (backward compatible)
    if operator not in machine.allowed_operators:
        return Err("operator {operator} not authorized for machine {machine}")
```

Operator identity: `$USER` on local, or `--operator` flag for CI. Stored in generation metadata for audit trail.

### Resource Path Restrictions

Prevent resources from writing to sensitive system paths:

```yaml
# In policy section of config
policy:
  deny_paths:
    - /etc/shadow
    - /etc/sudoers
    - /etc/sudoers.d/*
    - /root/.ssh/authorized_keys
  # Default: no restrictions (current behavior)
```

```
fn check_path_policy(resource, policy):
    if let Some(path) = resource.path:
        for denied in policy.deny_paths:
            if glob_match(denied, path):
                return Err("path {path} is denied by policy")
    Ok(())
```

Path policy is **advisory** — it's checked at parse time (during `validate_deny_paths()` in `format_validation.rs`), not enforced by the OS. A malicious config could disable the policy. For defense-in-depth, combine with OS-level mandatory access control (AppArmor/SELinux profiles for the forjar process).

---

## Secret Management

### The Problem

Resources with `content:` fields may contain secrets:
```yaml
resources:
  db-config:
    type: file
    path: /etc/app/db.yaml
    content: |
      host: db.internal
      password: s3cret_p4ssw0rd  # BAD: plaintext in config
```

This password appears in:
1. The forjar config YAML (checked into git)
2. `state/<machine>/state.lock.yaml` (resource hash includes content)
3. `state/<machine>/events.jsonl` (apply event may log resource details)
4. `state.db` FTS5 index (`content_preview` column)
5. `forjar query "db"` output

### Secret References

Replace inline secrets with references to external secret stores:

```yaml
resources:
  db-config:
    type: file
    path: /etc/app/db.yaml
    content: |
      host: db.internal
      password: {{ secrets.db_password }}
```

Resolution at apply time:

```
fn resolve_secrets(content, secret_provider):
    for match in regex("\\{\\{\\s*secrets\\.(\\w+)\\s*\\}\\}"):
        let value = secret_provider.get(match.group(1))?
        content = content.replace(match.full, value)
    content
```

### Secret Providers

| Provider | Config | Resolution | Status |
|---------|--------|------------|--------|
| Environment variable | `secrets.provider: env` | `$FORJAR_SECRET_<name>` | Implemented |
| File | `secrets.provider: file`, `secrets.path: /run/secrets/` | Read `/run/secrets/<name>` | Implemented |
| SOPS | `secrets.provider: sops`, `secrets.file: secrets.enc.yaml` | `sops -d secrets.enc.yaml` | Implemented |
| 1Password CLI | `secrets.provider: op` | `op read "op://vault/item/field"` | Implemented |
| Age encryption | `secrets.provider: age` | Age-encrypted values in config | Implemented |

> **Current status**: All five providers are implemented. Age encryption in `secrets.rs`; Env, File, Sops, Op dispatched via `SecretProvider` enum in `core/resolver/template.rs`. Template expansion of `{{ secrets.* }}` references resolves at apply time.

### Redaction

Secrets are **never** stored in state files:

```
fn hash_desired_state_with_secrets(resource):
    // Hash the TEMPLATE, not the resolved value
    // "{{ secrets.db_password }}" is hashed, not "s3cret_p4ssw0rd"
    // This means: changing the secret value does NOT change the hash
    // → forjar apply doesn't detect secret rotation
    // → use `forjar apply --force` or `forjar drift` for secret rotation

fn log_event(resource, event):
    // Redact any resolved secret values from event details
    for secret_name in resource.secret_refs:
        event.details = event.details.replace(secret_value, "***")
```

### Secret Rotation

Changing a secret value doesn't change `hash_desired_state` (the template is hashed, not the resolved value). To rotate secrets:

```bash
# Option 1: Force re-apply (re-resolves all secrets)
forjar apply --force --resource db-config

# Option 2: Drift detection compares live file content
forjar drift --machine production-db
# Detects: db-config content hash differs (secret was rotated externally)
```

### Limitation: Secrets in transit

When forjar applies a file resource with secret content, the resolved value is sent to the target machine via the transport layer (SSH, pepita). The secret is in the bash script piped to the remote shell. This is no worse than `scp` or `ansible-vault`, but the script may appear in process listings (`/proc/*/cmdline`).

**Mitigation**: For highly sensitive secrets, use `source:` pointing to an encrypted file that the target machine decrypts locally, rather than `content:` with inline secret references.

---

## Pepita Privilege Boundary

pepita uses `unshare(2)` which requires either:
- Root access, or
- `CAP_SYS_ADMIN` capability, or
- User namespace support (`/proc/sys/kernel/unprivileged_userns_clone = 1`)

### Privilege Levels

| Transport | Required Privilege | Isolation |
|----------|-------------------|-----------|
| `local` | Current user | None |
| `ssh` | SSH key auth | Remote user's permissions |
| `container` | Docker socket access | Container namespace |
| `pepita` | `CAP_SYS_ADMIN` or userns | PID + mount + optional net namespace |

pepita with `CAP_SYS_ADMIN` is root-equivalent on the host. A malicious resource executed in a pepita namespace can escape to the host by:
- Mounting `/` as writable overlay
- Writing to `/etc/cron.d/` via the overlay upper

**Mitigation**: pepita is designed for trusted workloads on trusted machines (developer workstations, CI runners, GPU compute nodes). It is NOT a security boundary. For untrusted workloads, use container transport with a hardened runtime, or Firecracker.

---

## State Integrity

### Existing Protections

- BLAKE3 sidecar files verify lock file integrity (`state/integrity.rs`)
- Tamper-evident event chain (`tripwire/chain.rs`): each event's hash includes the previous event's hash
- `forjar drift` detects unauthorized modifications to managed files

### Missing Protection: State Directory Access Control

The state directory (`state/`) has no access control beyond filesystem permissions. On a shared machine, other users can:
- Read lock files (see resource details)
- Modify lock files to fake convergence (bypass the sidecar check by recomputing it)
- Read `events.jsonl` (see full operation history)

**Recommendation**: Set `state/` to `0700` (owner-only). For shared environments, use a dedicated service account for forjar operations. The BLAKE3 sidecar detects accidental corruption but NOT malicious tampering (the attacker can recompute the sidecar).

---

## Implementation

### Phase 17: Security Model (FJ-2300) -- IMPLEMENTED
- [x] Machine-level `allowed_operators` field on Machine struct
- [x] `is_operator_allowed()` authorization check (empty = no restriction, backward compatible)
- [x] Known field detection and JSON Schema updated for `allowed_operators`
- [x] `--operator` flag: `OperatorIdentity` with `from_flag()`, `from_env()`, `resolve()` and `OperatorSource` enum
- [x] `policy.deny_paths` for resource path restrictions
- [x] `secrets.provider` with env and file backends
- [x] Secret redaction via `redact_secrets()` utility
- [x] `{{ secrets.* }}` template resolution at apply time
- [x] Hash template (not resolved value) in `hash_desired_state`
- [x] Security types: `SecretProvider`, `SecretRef`, `SecretConfig`, `PathPolicy`, `AuthzResult`
- [x] Secret scan types: `SecretScanResult`, `SecretScanFinding` with structured output
- [x] `forjar apply --force` for secret rotation — bypasses planner hash comparison, re-applies all resources (empty locks passed to planner)
- [x] Document pepita privilege boundary honestly (see "Pepita Privilege Boundary" section above)
- **Deliverable**: Secrets never appear in state files; path policy prevents accidental writes to sensitive paths
