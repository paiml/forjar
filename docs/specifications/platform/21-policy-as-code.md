# Policy-as-Code Engine

> User-extensible validation policies with compliance packs — CIS, STIG, SOC2 mappings.

**Status**: Proposed | **Date**: 2026-03-09 | **Spec IDs**: FJ-3200 through FJ-3209

---

## Motivation

Forjar's built-in `--strict` lint provides 6 hardcoded checks. Enterprise IaC requires extensible, auditable, machine-enforceable policies. Terraform has Sentinel/OPA, Pulumi has CrossGuard. Forjar needs a sovereign policy engine that doesn't depend on OPA/Rego — staying within the stack.

### Chain of Thought: Sovereign Stack Implementation

```
Problem: No user-extensible policy framework. Only hardcoded lint rules.

STEP 1 — Policy Language (forjar-native YAML, NOT Rego)
  Rego/OPA is a non-sovereign dependency with its own runtime.
  Instead: policies are YAML with typed assertions (assert/deny/require/limit).
  This is consistent with forjar's YAML-first philosophy.
  Policies reference resource fields via the same {{ template }} syntax.

STEP 2 — Policy Evaluation Engine (certeza integration)
  certeza already evaluates quality thresholds (coverage, mutation).
  Extend certeza's evaluation model: policy = predicate + threshold + severity.
  Three severities: error (blocks apply), warning (logged), info (advisory).
  Evaluation is pure function: Config → PolicySet → Vec<Violation>.

STEP 3 — Compliance Packs (recipe bundles)
  CIS/STIG/SOC2 mappings ship as forjar recipe bundles.
  Each pack is a directory of policy YAML files + metadata.
  `forjar policy install cis-ubuntu-22` fetches from recipe registry.
  Content-addressed with BLAKE3 — tamper-evident compliance.

STEP 4 — Static Analysis (pmat TDG grading)
  pmat grades policy rule complexity.
  Overly complex policies (cyclomatic > 10) flagged as smell.
  Policy coverage tracked: which resources have which policies.

STEP 5 — Boundary Testing (verificar)
  verificar generates synthetic configs at policy boundaries.
  Property: every deny policy must reject at least one generated config.
  Property: every assert policy must pass on the golden config.

STEP 6 — Script Assertions (bashrs)
  Custom policy assertions can include shell commands.
  All assertion scripts purified through bashrs I8 invariant.

Conclusion: Policy engine uses only sovereign components. No OPA binary,
no Rego parser, no Sentinel runtime. Policies are auditable YAML.
```

---

## Architecture

```
┌─────────────────────────────────┐
│     forjar validate --policy     │
│     forjar policy check          │
│     forjar apply --policy-check  │
└──────────┬──────────────────────┘
           │
┌──────────▼──────────────────────┐
│     Policy Evaluation Engine     │
│                                  │
│  Load: policies/*.yaml           │
│  Parse: typed policy rules       │
│  Eval: Config × Policy → Result  │
│  Report: violations + remediation│
└──────────┬──────────────────────┘
           │
┌──────────▼──────────────────────┐
│     Compliance Packs             │
│                                  │
│  cis-ubuntu-22/                  │
│  stig-rhel-9/                    │
│  soc2-baseline/                  │
│  custom/                         │
└─────────────────────────────────┘
```

### Policy Format

```yaml
# policies/security-baseline.yaml
policies:
  - id: SEC-001
    name: "No root-owned files without system tag"
    severity: error
    type: assert
    scope: resources[type=file]
    condition: |
      owner != "root" OR tags CONTAINS "system"
    remediation: "Add 'system' tag or change owner to non-root"
    compliance:
      - cis: "6.1.2"
      - stig: "V-238196"

  - id: SEC-002
    name: "SSH keys required for remote machines"
    severity: error
    type: require
    scope: machines[addr != "localhost" AND addr != "127.0.0.1" AND addr != "container"]
    field: ssh_key
    remediation: "Add ssh_key field to machine definition"

  - id: SEC-003
    name: "No privileged containers"
    severity: warning
    type: deny
    scope: machines[container.privileged = true]
    remediation: "Remove 'privileged: true' or document exception"

  - id: PERF-001
    name: "Package lists under 50 items"
    severity: warning
    type: limit
    scope: resources[type=package]
    field: packages
    max_count: 50
    remediation: "Split into multiple package resources for parallel install"
```

### Policy Types

| Type | Semantics | Example |
|------|-----------|---------|
| `assert` | Condition must be true for matching resources | "All files have owner" |
| `deny` | Matching resources are violations | "No privileged containers" |
| `require` | Field must exist on matching resources | "SSH key on remote machines" |
| `limit` | Field count/value must be within bounds | "Max 50 packages per resource" |
| `script` | Shell command must exit 0 (bashrs-validated) | "openssl verify cert.pem" |

---

## Spec IDs

| ID | Deliverable | Depends On |
|----|-------------|-----------|
| FJ-3200 | Policy YAML schema and parser | — |
| FJ-3201 | Policy evaluation engine (assert/deny/require/limit) | FJ-3200 |
| FJ-3202 | `forjar policy check -f forjar.yaml` CLI | FJ-3201 |
| FJ-3203 | `forjar apply --policy-check` pre-apply gate | FJ-3201 |
| FJ-3204 | Script-type policies with bashrs purification | FJ-3201 |
| FJ-3205 | Compliance pack format and `forjar policy install` | FJ-3200 |
| FJ-3206 | CIS Ubuntu 22.04 compliance pack (20+ rules) | FJ-3205 |
| FJ-3207 | JSON/SARIF output for CI integration | FJ-3202 |
| FJ-3208 | Policy coverage report (`forjar policy coverage`) | FJ-3201 |
| FJ-3209 | Mutation testing: verificar boundary configs | FJ-3201 |

---

## Performance Targets

| Operation | Target | Mechanism |
|-----------|--------|-----------|
| Policy parse (100 rules) | < 5ms | serde_yaml_ng deserialization |
| Policy eval (100 rules × 100 resources) | < 50ms | In-memory predicate evaluation |
| Compliance report generation | < 100ms | Template-based markdown/SARIF |

---

## Batuta Oracle Advice

**Recommendation**: certeza (85% confidence) for validation tasks.
**Pattern**: certeza's threshold evaluation model maps directly to policy assertions.
**Supporting**: pmat for grading policy rule complexity, verificar for boundary testing.

## arXiv References

- [ARPaCCino: Agentic-RAG for Policy-as-Code Compliance (2507.10584)](https://arxiv.org/abs/2507.10584) — LLM + RAG for policy generation and verification
- [TerraFormer: Policy-Guided IaC Verification (2601.08734)](https://arxiv.org/abs/2601.08734) — Neuro-symbolic framework for policy compliance
- [Papadakis et al. (2019) — Mutation Testing Advances](https://arxiv.org/abs/2002.05090) — Mutation testing for policy rule validation
- [Three Decades of Formal Methods in Compliance (2410.10906)](https://arxiv.org/abs/2410.10906) — Survey of formal compliance verification

---

## Falsification Criteria

| ID | Claim | Rejection Test |
|----|-------|---------------|
| F-3200-1 | All 4 policy types evaluate correctly | Generate 100 boundary configs via verificar; REJECT if any misclassification |
| F-3200-2 | Error-severity blocks apply | Create config violating error-policy; REJECT if `forjar apply --policy-check` succeeds |
| F-3200-3 | Policy eval < 50ms | Benchmark 100 rules × 100 resources; REJECT if p95 > 50ms |
| F-3200-4 | bashrs validates script policies | Inject command injection in script policy; REJECT if bashrs doesn't catch it |
| F-3200-5 | Compliance packs are tamper-evident | Modify one byte in installed pack; REJECT if BLAKE3 verification passes |
| F-3200-6 | No OPA/Rego dependency | Audit Cargo.toml and imports; REJECT if any OPA/Rego crate found |
| F-3200-7 | Cross-dimension discrimination | Compute policy scores across 10 configs; REJECT if σ < 5 |
