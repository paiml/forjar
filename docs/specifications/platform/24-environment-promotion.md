# Environment Promotion Pipelines

> First-class dev/staging/prod environments with quality gates, diff analysis, and progressive rollout.

**Status**: Proposed | **Date**: 2026-03-09 | **Spec IDs**: FJ-3500 through FJ-3509

---

## Motivation

Forjar manages infrastructure but has no concept of environments. Users deploying to dev/staging/prod must maintain separate YAML files with duplicated config. Spacelift, Env0, and ArgoCD all provide environment promotion with quality gates. Forjar needs native environment support with built-in quality gates — using the sovereign stack.

### Chain of Thought: Sovereign Stack Implementation

```
Problem: No environment abstraction. No promotion flow. No quality gates.

STEP 1 — Environment Definition (forjar core types)
  Add `environments:` block to forjar.yaml.
  Each environment: name, machine overrides, param overrides, state directory.
  Single source YAML, multiple environments — DRY by construction.
  Environments inherit from base config. Overrides are explicit.

STEP 2 — Environment-Scoped State (forjar state layer)
  State directories: state/dev/, state/staging/, state/prod/
  Each environment has independent state.lock.yaml, events.jsonl.
  `forjar apply -e staging` targets the staging environment.
  Cross-environment drift: `forjar environments diff dev staging`.

STEP 3 — Quality Gates (certeza + forjar validate)
  Promotion requires passing quality gates:
  - forjar validate --deep (structural correctness)
  - certeza coverage check (test coverage ≥ 95%)
  - forjar policy check (policy-as-code compliance, FJ-3200)
  - Custom assertions (user-defined shell checks, bashrs-validated)
  Gates are composable: each environment can define its own gate list.

STEP 4 — Promotion Engine (batuta pipeline model)
  batuta's pipeline transpilation model handles multi-stage promotion.
  `forjar promote dev staging` executes:
    1. Snapshot dev state (generation)
    2. Run quality gates
    3. Apply to staging with param overrides
    4. Run post-apply verification
    5. Record promotion event in events.jsonl
  Approval gates: manual (interactive prompt) or automatic (all gates pass).

STEP 5 — Progressive Rollout (forjar executor extension)
  For prod promotion: canary → percentage → full rollout.
  Canary: apply to 1 machine, health check, proceed or rollback.
  Percentage: apply to N% of fleet, health check at each step.
  Auto-rollback: if health check fails, revert to previous generation.

STEP 6 — Diff Analysis (pmat integration)
  pmat analyzes config diff between environments:
  - Which resources changed?
  - What's the blast radius?
  - Are there breaking changes (removed resources, changed types)?
  Report shown before promotion, requires confirmation.

Conclusion: Environment promotion uses forjar state for isolation,
certeza for quality gates, batuta for pipeline orchestration,
bashrs for assertion script validation, pmat for diff analysis.
Zero external CI/CD platform dependency.
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                 forjar environments                       │
│  list | diff | promote | rollback | status                │
└──────────┬────────────────────────────────────────────┘
           │
┌──────────▼────────────────────────────────────────────┐
│              Environment Resolution                     │
│                                                         │
│  Base config (forjar.yaml)                              │
│    + environment overrides (params, machines)            │
│    = resolved config for target environment              │
└──────────┬────────────────────────────────────────────┘
           │
┌──────────▼────────────────────────────────────────────┐
│              Promotion Pipeline                         │
│                                                         │
│  1. Snapshot source environment                         │
│  2. Run quality gates (certeza, validate, policy)       │
│  3. Diff analysis (blast radius report)                 │
│  4. Approval gate (manual or auto)                      │
│  5. Apply to target environment                         │
│  6. Progressive rollout (canary → percentage → full)    │
│  7. Post-apply verification                             │
│  8. Record promotion event                              │
└──────────┬────────────────────────────────────────────┘
           │
┌──────────▼────────────────────────────────────────────┐
│    state/dev/          state/staging/     state/prod/   │
│    ├── state.lock.yaml ├── state.lock... ├── state...  │
│    ├── events.jsonl    ├── events.jsonl  ├── events... │
│    └── generations/    └── generations/  └── gen.../   │
└────────────────────────────────────────────────────────┘
```

### Configuration

```yaml
# forjar.yaml
environments:
  dev:
    description: "Development environment"
    machines:
      web:
        addr: "dev-web-01.internal"
    params:
      log_level: "debug"
      replicas: 1

  staging:
    description: "Staging environment"
    machines:
      web:
        addr: "staging-web-01.internal"
    params:
      log_level: "info"
      replicas: 2
    promotion:
      from: dev
      gates:
        - validate: { deep: true }
        - policy: { strict: true }
      auto_approve: true

  prod:
    description: "Production environment"
    machines:
      web:
        addr: "prod-web-{01..04}.internal"
    params:
      log_level: "warn"
      replicas: 4
    promotion:
      from: staging
      gates:
        - validate: { deep: true, exhaustive: true }
        - policy: { strict: true }
        - coverage: { min: 95 }
        - script: "curl -sf https://staging.internal/health"
      auto_approve: false    # require manual approval
      rollout:
        strategy: canary
        canary_count: 1
        health_check: "curl -sf http://{{ machine.addr }}:8080/health"
        health_timeout: 30s
        percentage_steps: [25, 50, 100]
```

### CLI

```bash
# List environments
forjar environments list -f forjar.yaml

# Show diff between environments
forjar environments diff dev staging -f forjar.yaml

# Promote dev → staging (runs quality gates)
forjar promote dev staging -f forjar.yaml

# Promote staging → prod (with progressive rollout)
forjar promote staging prod -f forjar.yaml

# Apply to specific environment
forjar apply -f forjar.yaml -e staging

# Rollback prod to previous generation
forjar environments rollback prod -f forjar.yaml

# Show promotion history
forjar environments history prod -f forjar.yaml --json
```

---

## Spec IDs

| ID | Deliverable | Depends On |
|----|-------------|-----------|
| FJ-3500 | `environments:` block in config schema | — |
| FJ-3501 | Environment resolution (base + override merging) | FJ-3500 |
| FJ-3502 | Environment-scoped state directories | FJ-3500 |
| FJ-3503 | `forjar apply -e <env>` environment targeting | FJ-3501, FJ-3502 |
| FJ-3504 | `forjar environments list/diff` commands | FJ-3501 |
| FJ-3505 | Quality gate framework (validate, policy, coverage, script) | FJ-3501 |
| FJ-3506 | `forjar promote <src> <dst>` with gate evaluation | FJ-3505 |
| FJ-3507 | Progressive rollout (canary → percentage → full) | FJ-3506 |
| FJ-3508 | Auto-rollback on health check failure | FJ-3507 |
| FJ-3509 | Promotion event logging and history | FJ-3506 |

---

## Performance Targets

| Operation | Target | Mechanism |
|-----------|--------|-----------|
| Environment resolution | < 5ms | In-memory config merge |
| Quality gate evaluation | < 10s | Parallel gate execution |
| Environment diff | < 100ms | In-memory config comparison |
| Canary health check | < 30s | HTTP health probe with timeout |
| Promotion event recording | < 5ms | Append to events.jsonl |

---

## Batuta Oracle Advice

**Recommendation**: batuta for pipeline orchestration and promotion sequencing.
**Compute**: Scalar — promotion is orchestration, not compute.
**Supporting**: certeza for quality gate evaluation, pmat for diff analysis.

## arXiv References

- [AI-Augmented CI/CD Pipelines (2508.11867)](https://arxiv.org/abs/2508.11867) — Reactive pipeline triggers and progressive delivery
- [MLOps: Continuous Delivery Pipelines (2011.01984)](https://arxiv.org/abs/2011.01984) — Multi-stage promotion patterns
- [Continuous Integration and Delivery for ML (2209.09125)](https://arxiv.org/abs/2209.09125) — CI/CD adapted for complex pipelines
- [Continuous Deployment at Facebook (2110.04008)](https://arxiv.org/abs/2110.04008) — Progressive rollout at scale

---

## Falsification Criteria

| ID | Claim | Rejection Test |
|----|-------|---------------|
| F-3500-1 | Environment isolation | Apply to dev; REJECT if staging state modified |
| F-3500-2 | Quality gates block promotion | Introduce policy violation; REJECT if promotion succeeds |
| F-3500-3 | Progressive rollout respects canary | Apply to prod with 4 machines; REJECT if more than canary_count updated first |
| F-3500-4 | Auto-rollback on health failure | Fail health check during canary; REJECT if rollback doesn't trigger |
| F-3500-5 | Environment diff is accurate | Change one param; REJECT if diff doesn't show exactly one change |
| F-3500-6 | Promotion history is append-only | Promote twice; REJECT if first promotion event overwritten |
| F-3500-7 | No external CI/CD dependency | Audit Cargo.toml; REJECT if any CI/CD platform SDK imported |
| F-3500-8 | Config DRY: single YAML | Count resource definitions; REJECT if any resource duplicated across environments |
