# Ephemeral Values and State Encryption

> Secrets used during apply but never persisted. At-rest encryption for state files.

**Status**: Implemented | **Date**: 2026-03-09 | **Spec IDs**: FJ-3300 through FJ-3309

---

## Motivation

Forjar stores all state in plaintext YAML/JSONL. Secrets referenced in templates (database passwords, API keys, TLS certs) are resolved and persisted in `state.lock.yaml`. Terraform 1.10 introduced ephemeral resources. Pulumi encrypts state by default. Forjar needs both: ephemeral values that never touch disk, and encrypted state for values that must persist.

### Chain of Thought: Sovereign Stack Implementation

```
Problem: Secrets persist in plaintext state. No ephemeral value support.

STEP 1 — Ephemeral Field Marker (forjar core types)
  Add `ephemeral: true` to resource fields and params.
  During apply: resolve template → execute → discard value before state write.
  State stores BLAKE3 hash of ephemeral value (for drift detection) but NOT the value.
  This means: drift detection works, but state replay requires re-fetching the secret.

STEP 2 — State Encryption (age, sovereign)
  age (https://age-encryption.org) is the encryption primitive.
  Why age over GPG: simpler, auditable, no web-of-trust complexity.
  Why age over cloud KMS: sovereign — no AWS/GCP/Azure dependency.
  State files encrypted with age identity (passphrase or key file).
  BLAKE3 HMAC over ciphertext for integrity verification.

STEP 3 — Secret Injection Isolation (pepita namespaces)
  When `ephemeral: true`, secret value injected via pepita namespace.
  Process environment isolated — secret exists only in namespace process tree.
  After apply completes, namespace torn down → secret gone from memory.
  No /proc leak, no environment inheritance to child processes.

STEP 4 — Script Leak Detection (bashrs)
  bashrs lints generated scripts for secret leakage patterns:
  - echo/printf of variables containing secret names
  - Redirection of secret values to files
  - curl/wget with inline credentials
  Severity: error (blocks apply).

STEP 5 — Audit Trail (renacer)
  renacer traces which processes accessed secret values.
  Audit log: who read the secret, when, from which namespace.
  Append to events.jsonl with event_type: "secret_access".

Conclusion: Secrets never persist unencrypted. Ephemeral values never persist
at all. All encryption uses age (sovereign). Secret handling isolated via
pepita namespaces. No cloud KMS, no Vault, no external secret manager required.
(Optional cloud KMS integration as provider, not dependency.)
```

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│              Secret Resolution                    │
│                                                   │
│  {{ secrets.db_password }}  →  Provider lookup     │
│  {{ ephemeral.api_key }}   →  Provider lookup      │
└──────────┬──────────────────────────────────────┘
           │
┌──────────▼──────────────────────────────────────┐
│              Secret Providers                     │
│                                                   │
│  env:     $FORJAR_SECRET_*                        │
│  file:    /run/secrets/<name>                     │
│  age:     secrets.age (encrypted file)            │
│  exec:    `vault kv get -field=...` (optional)    │
└──────────┬──────────────────────────────────────┘
           │
┌──────────▼──────────────────────────────────────┐
│         Ephemeral / Persistent Split              │
│                                                   │
│  ephemeral: true  →  use in apply, hash for state │
│  ephemeral: false →  encrypt with age, write state│
└──────────┬──────────────────────────────────────┘
           │
┌──────────▼──────────────────────────────────────┐
│         State Encryption Layer                    │
│                                                   │
│  state.lock.yaml  →  state.lock.yaml.age          │
│  events.jsonl     →  events.jsonl.age             │
│  BLAKE3 HMAC over ciphertext                      │
│  Key: age identity file or passphrase             │
└─────────────────────────────────────────────────┘
```

### Configuration

```yaml
# forjar.yaml
encryption:
  enabled: true
  provider: age
  identity: ~/.config/forjar/key.age    # age identity file
  recipients:                            # additional recipients
    - age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p
  encrypt_state: true                    # encrypt state.lock.yaml
  encrypt_events: true                   # encrypt events.jsonl

params:
  db_password:
    value: "{{ secrets.db_password }}"
    ephemeral: true                      # never persisted to state

secrets:
  db_password:
    provider: env
    key: DB_PASSWORD
  tls_cert:
    provider: file
    path: /run/secrets/tls.crt
  vault_token:
    provider: exec
    command: "vault kv get -field=token secret/forjar"
    ephemeral: true                      # always ephemeral
```

---

## Spec IDs

| ID | Deliverable | Depends On |
|----|-------------|-----------|
| FJ-3300 | `ephemeral: true` field on params and resources | — |
| FJ-3301 | Secret provider trait (env, file, age, exec) | FJ-3300 |
| FJ-3302 | Ephemeral resolution: use → hash → discard before state write | FJ-3300 |
| FJ-3303 | age encryption for state files | — |
| FJ-3304 | `forjar state encrypt` / `forjar state decrypt` commands | FJ-3303 |
| FJ-3305 | BLAKE3 HMAC integrity verification for encrypted state | FJ-3303 |
| FJ-3306 | pepita namespace isolation for ephemeral secret injection | FJ-3302 |
| FJ-3307 | bashrs secret leakage detection in generated scripts | FJ-3301 |
| FJ-3308 | renacer audit trail for secret access | FJ-3306 |
| FJ-3309 | Key rotation: `forjar state rekey --new-identity` | FJ-3303 |

---

## Performance Targets

| Operation | Target | Mechanism |
|-----------|--------|-----------|
| Secret resolution (env provider) | < 1ms | Direct env lookup |
| Secret resolution (file provider) | < 5ms | Single file read |
| State encryption (100 resources) | < 50ms | age stream encryption |
| State decryption (100 resources) | < 50ms | age stream decryption |
| BLAKE3 HMAC verification | < 1ms | Single-pass HMAC |

---

## Batuta Oracle Advice

**Recommendation**: batuta for orchestrating secret lifecycle across machines.
**Compute**: Scalar — encryption is CPU-bound, no GPU.
**Key insight**: age encryption is the sovereign choice — no cloud dependency.

## arXiv References

- [Terraform Ephemeral Resources (Hashicorp, 2024)](https://securityboulevard.com/2025/10/terraform-secrets-management-best-practices-secret-managers-and-ephemeral-resources/) — Industry precedent for ephemeral values
- [ADA: Ephemeral Infrastructure-Native Rotation (arXiv 2025)](https://arxiv.org/list/cs.CR/2025-05) — Moving target defense via ephemeral infrastructure
- [Post-Quantum Cryptography Survey (2510.10436)](https://arxiv.org/abs/2510.10436) — Future-proofing encryption choices

---

## Falsification Criteria

| ID | Claim | Rejection Test |
|----|-------|---------------|
| F-3300-1 | Ephemeral values never in state | Set ephemeral param, apply, grep state files; REJECT if cleartext found |
| F-3300-2 | Drift detection works on ephemeral | Change ephemeral secret, run drift; REJECT if drift not detected via hash |
| F-3300-3 | Encrypted state round-trips | Encrypt → decrypt → diff; REJECT if any byte differs |
| F-3300-4 | BLAKE3 HMAC catches tampering | Flip one bit in encrypted state; REJECT if HMAC verification passes |
| F-3300-5 | pepita namespace isolates secrets | Read /proc/PID/environ from outside namespace; REJECT if secret visible |
| F-3300-6 | bashrs catches secret echo | Generate script with `echo $DB_PASSWORD`; REJECT if bashrs doesn't flag |
| F-3300-7 | Key rotation preserves state | Rekey with new identity; REJECT if decrypted state differs from original |
| F-3300-8 | No cloud KMS in default path | Audit Cargo.toml; REJECT if aws-sdk, gcp, or azure crates are non-optional |
