# Supply Chain Security & Resilience

Forjar provides a comprehensive supply chain security toolkit: SLSA provenance attestation, Merkle DAG lineage, SBOM/CBOM generation, privilege analysis, fault injection testing, and security scanning — all built into the CLI with no external dependencies.

## Convergence Proofs

Verify that your configuration can converge from any state:

```bash
forjar prove -f forjar.yaml
```

Output:
```
Convergence Proof: my-stack
------------------------------------------------------------------------
[PASS] codegen-completeness: all resources produce check/apply/state_query scripts
[PASS] dag-acyclicity: DAG is acyclic (8 resources)
[PASS] state-coverage: 0/8 resources have state entries (0%)
[PASS] hash-determinism: 8 resources: state_query scripts are deterministic
[PASS] idempotency-structure: 8/8 apply scripts use set -euo pipefail (100%)
------------------------------------------------------------------------
5/5 proofs passed
```

The five proof obligations:

| Proof | What It Checks |
|-------|---------------|
| codegen-completeness | Every resource type produces valid check/apply/state_query scripts |
| dag-acyclicity | No cycles in the dependency graph |
| state-coverage | State entries match declared resources |
| hash-determinism | BLAKE3 hashes are deterministic (same resource → same hash) |
| idempotency-structure | Apply scripts use `set -euo pipefail` for safe execution |

## SLSA Provenance Attestation

Generate [SLSA](https://slsa.dev/) provenance attestation for audit trails:

```bash
forjar provenance -f forjar.yaml
```

Output:
```
SLSA Provenance Attestation

  Subject:     my-stack
  Timestamp:   1772640004
  Config hash: blake3:b0f54ff8c40ae094
  Plan hash:   blake3:7951f597ac2f6c00

  State hashes:
    * forjar.lock blake3:83f87e171d97f13c

  Materials (3 resources):
    - nginx-pkg   9a0178c657a6fe76
    - nginx-conf  4f87c4ef88c01663
    - nginx-svc   5c3a03db545d2875
```

Every resource is content-addressed with BLAKE3. The attestation includes config hash, plan hash, and per-resource material hashes — providing a complete chain of custody.

```bash
forjar provenance -f forjar.yaml --json  # Machine-readable for CI
```

## Merkle DAG Lineage

Visualize the content-addressed dependency tree:

```bash
forjar lineage -f forjar.yaml
```

Output:
```
Merkle DAG Lineage

  Config:      my-stack
  Merkle root: c30932ad738e65e2
  Nodes:       3

  * nginx-pkg     9a0178c657a6fe76
  * nginx-conf    4f87c4ef88c01663 <- [nginx-pkg]
  * nginx-svc     5c3a03db545d2875 <- [nginx-conf]
```

Each node's hash incorporates its own content plus its dependencies' hashes, forming a Merkle DAG. Any change to a leaf propagates hash changes upward — making tampering detectable.

## Security Scanning

Static security scan of your configuration:

```bash
forjar security-scan -f forjar.yaml
```

Output:
```
Security Scan Results
------------------------------------------------------------
  [HIGH] SS-3 (nginx-site) — mode 0644 allows world access
  [MED ] SS-4 (nginx-enable) — externally-sourced file has no integrity check
------------------------------------------------------------
Summary: 0 critical, 2 high, 1 medium, 0 low (3 total)
```

Checks include:
- **SS-1**: Root-owned files with write permissions
- **SS-2**: Services running without resource limits
- **SS-3**: World-readable sensitive files
- **SS-4**: External content without integrity verification
- **SS-5**: Unencrypted secrets in config

```bash
forjar security-scan -f forjar.yaml --json  # For CI gatekeeping
```

## Privilege Analysis

Audit minimum privileges required per resource:

```bash
forjar privilege-analysis -f forjar.yaml
```

Output:
```
Privilege Analysis

  8 Requires elevated privileges:
    ! firewall-https (network) — network-config
    ! nginx-service (service) — service-control
    ! nginx-pkg (package) — package-manager
    ! nginx-conf (file) — system-write

  Summary: 4/4 resources need root
```

Categories: `package-manager`, `service-control`, `system-write`, `network-config`, `user-management`.

## SBOM & CBOM Generation

Generate Software Bill of Materials and Cryptographic Bill of Materials:

```bash
forjar sbom -f forjar.yaml -s state/            # Software BOM
forjar cbom -f forjar.yaml                       # Cryptographic BOM
```

SBOM output includes every managed package with version, provider, and machine. CBOM documents all cryptographic algorithms and key materials in the configuration.

Both support `--json` for integration with vulnerability scanners and compliance tools.

## Fault Injection Testing

Simulate infrastructure failures before they happen:

```bash
forjar fault-inject -f forjar.yaml
```

Output:
```
Fault Injection Report
======================
Total: 15 | Passed: 14 | Failed: 1

[PASS] nginx-pkg: network-timeout (transport)
[PASS] nginx-pkg: idempotency-check (convergence)
[PASS] nginx-conf: permission-denied (filesystem)
[PASS] nginx-conf: disk-full (filesystem)
[PASS] nginx-conf: dep-failure-cascade (dependency)
[FAIL] nginx-conf: idempotency-check (convergence)
       Expected: Check script returns 0 on second apply
```

Five fault categories:

| Category | Simulates |
|----------|-----------|
| network-timeout | SSH/transport connection failures |
| permission-denied | File system permission errors |
| disk-full | Out of disk space during write |
| dep-failure-cascade | Upstream dependency failure propagation |
| idempotency-check | Second apply should be a no-op |

## Runtime Invariant Monitors

Verify structural invariants hold for your configuration:

```bash
forjar invariants -f forjar.yaml --json
```

Checks DAG structure, resource naming conventions, dependency completeness, and other structural properties that must hold before apply.

## Cost Estimation

Estimate apply time before committing:

```bash
forjar cost-estimate -f forjar.yaml
```

Output:
```
Cost Estimation

  Stack:     my-stack
  Resources: 4
  Machines:  1
  Est. time: 44s (sequential)

  M nginx-pkg (Package) ~30s [package-management]
  L nginx-conf (File) ~2s [file-management]
  L nginx-enable (File) ~2s [file-management]
  M nginx-svc (Service) ~10s [service-management]
```

Estimates are based on static analysis of resource types and complexity. Actual apply time varies by network and system load.

## Reproducibility Proofs

Generate training and build reproducibility certificates:

```bash
forjar repro-proof -f forjar.yaml -s state/ --json
```

Verifies that the configuration + state + content hashes form a reproducible deployment — the same inputs will always produce the same infrastructure state.

## Secret Management (FJ-2300)

Forjar resolves secrets at apply time without storing them in state files:

```yaml
resources:
  db-config:
    type: file
    path: /etc/app/db.yaml
    content: |
      host: db.internal
      password: {{ secrets.db_password }}
```

Four secret provider backends are supported:

| Provider | Resolution |
|---------|-----------|
| `env` (default) | `$FORJAR_SECRET_<name>` |
| `file` | Read from `/run/secrets/<name>` |
| `sops` | `sops -d secrets.enc.yaml` |
| `op` | 1Password CLI `op read` |

**Key behavior**: `hash_desired_state` hashes the template (`{{ secrets.db_password }}`), not the resolved value. This means secret rotation requires `forjar apply --force`.

Path policies restrict writes to sensitive system paths:

```yaml
policy:
  deny_paths:
    - /etc/shadow
    - /etc/sudoers.d/*
```

`forjar validate --check-secrets` scans for hardcoded credentials in resource content fields.

## CI/CD Integration

All commands support `--json` for pipeline integration:

```bash
# Gate: block deploy if security findings are critical
forjar security-scan -f forjar.yaml --json | jq -e '.findings | map(select(.severity == "critical")) | length == 0'

# Gate: block deploy if blast radius exceeds threshold
forjar impact -f forjar.yaml -r db-pkg --json | jq -e '.risk != "critical"'

# Gate: verify all convergence proofs pass
forjar prove -f forjar.yaml --json | jq -e '.passed == .total'

# Audit: generate provenance for compliance
forjar provenance -f forjar.yaml --json > provenance.json
```
