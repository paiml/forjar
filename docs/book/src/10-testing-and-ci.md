# Testing & CI/CD Integration

This chapter covers testing strategies for forjar configs, from local validation to full CI/CD pipeline integration.

## Validation Pyramid

Test your infrastructure configs at three levels:

```
     ┌─────────┐
     │  Apply   │  Integration: apply to containers/staging
     ├─────────┤
     │  Check   │  Pre-flight: run check scripts on live machines
     ├─────────┤
     │ Validate │  Static: parse, validate, lint, plan, fmt
     └─────────┘
```

## Level 1: Static Validation

Static validation catches most errors without connecting to any machine.

### Validate

```bash
forjar validate -f forjar.yaml
```

Checks:
- YAML syntax
- Version is "1.0"
- Name is non-empty
- Resources reference valid machines
- Dependencies reference valid resources
- No circular dependencies
- File state is valid (file, directory, symlink, absent)
- Service state is valid (running, stopped, enabled, disabled)
- Mount state is valid (mounted, unmounted, absent)
- Docker state is valid (running, stopped, absent)
- Network protocol (tcp, udp) and action (allow, deny, reject) are valid
- Cron schedule has exactly 5 fields
- Symlink resources have a target field
- Format validation: octal mode, port range, absolute paths, Unix owner/group names (FJ-2501)
- Unknown field detection with "did you mean?" suggestions (FJ-2500)

### Deep Validation

```bash
forjar validate --deep -f forjar.yaml
```

Runs all deep checks in a single aggregated pass:
- Template variable resolution
- Circular dependency detection (transitive closure)
- Resource overlap detection (same path/port on same machine)
- Hardcoded secret scan
- Naming convention enforcement (kebab-case)
- Drift coverage verification
- Idempotency verification
- Exhaustive cross-reference validation

### Lint

```bash
forjar lint -f forjar.yaml
```

Detects best-practice violations:
- Unused machines
- Resources without tags (when many resources exist)
- Duplicate content across file resources
- Dependencies on non-existent resources
- Empty package lists

### Format Check

```bash
forjar fmt -f forjar.yaml --check
```

Exits non-zero if the config is not in canonical format. Use `forjar fmt` to fix.

### Plan (Dry Run)

```bash
forjar plan -f forjar.yaml
```

Shows what would change without connecting to machines. Useful for code review.

### Graph

```bash
forjar graph -f forjar.yaml
```

Generates a Mermaid or DOT dependency graph. Paste into GitHub PRs for visual review.

## Level 2: Pre-flight Checks

Run check scripts against live machines to verify preconditions.

```bash
# Check all resources
forjar check -f forjar.yaml

# Check a specific tag
forjar check -f forjar.yaml --tag critical

# JSON output for CI
forjar check -f forjar.yaml --json
```

Check scripts verify current state without modifying anything:
- Package: is the package installed?
- File: does the file exist?
- Service: is the service active/enabled?
- Mount: is the mount point active?

## Level 3: Integration Testing with Containers

For full integration testing, use container transport to apply configs in an isolated environment.

### Container Config

```yaml
version: "1.0"
name: integration-test

machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      name: forjar-test
      ephemeral: true
      init: true

resources:
  base-packages:
    type: package
    machine: test-box
    provider: apt
    packages: [curl, jq]
```

### Test Workflow

```bash
# Validate first
forjar validate -f test-config.yaml

# Plan
forjar plan -f test-config.yaml

# Apply to ephemeral container
forjar apply -f test-config.yaml --state-dir /tmp/test-state

# Check for drift (should be zero)
forjar drift -f test-config.yaml --state-dir /tmp/test-state --tripwire

# Apply again (should be idempotent)
forjar apply -f test-config.yaml --state-dir /tmp/test-state
```

## CI/CD Pipeline

### GitHub Actions

```yaml
name: Infrastructure Tests

on:
  pull_request:
    paths: ['forjar.yaml', 'recipes/**']

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install forjar
        run: cargo install forjar

      - name: Validate config
        run: forjar validate -f forjar.yaml

      - name: Lint config
        run: forjar lint -f forjar.yaml

      - name: Check formatting
        run: forjar fmt -f forjar.yaml --check

      - name: Show plan
        run: forjar plan -f forjar.yaml

      - name: Generate graph
        run: |
          forjar graph -f forjar.yaml > graph.md
          echo "## Dependency Graph" >> $GITHUB_STEP_SUMMARY
          echo '```mermaid' >> $GITHUB_STEP_SUMMARY
          cat graph.md >> $GITHUB_STEP_SUMMARY
          echo '```' >> $GITHUB_STEP_SUMMARY

  integration:
    runs-on: ubuntu-latest
    needs: validate
    steps:
      - uses: actions/checkout@v4

      - name: Install forjar
        run: cargo install forjar

      - name: Apply to container
        run: |
          forjar apply -f test-config.yaml \
            --state-dir /tmp/test-state

      - name: Verify idempotency
        run: |
          OUTPUT=$(forjar apply -f test-config.yaml \
            --state-dir /tmp/test-state 2>&1)
          echo "$OUTPUT"
          echo "$OUTPUT" | grep -q "0 to add"

      - name: Check drift
        run: |
          forjar drift -f test-config.yaml \
            --state-dir /tmp/test-state --tripwire
```

### Forjar Repository CI Jobs

The forjar repository runs 5 CI jobs on every push and pull request:

| Job | What it does | Catches |
|-----|-------------|---------|
| **test** | `cargo test --all-targets` + `cargo clippy` | Regressions, type errors, lint warnings |
| **container-test** | Build test-target Docker image + `cargo test --features container-test` | Container transport regressions |
| **gpu-container-test** | `cargo test --features gpu-container-test` (NVIDIA + AMD GPU hosts) | GPU device passthrough, env var propagation, cross-vendor parity |
| **fmt** | `cargo fmt --check` | Style violations |
| **dogfood** | Validate all 30 dogfood configs, run all 20 examples, verify MCP schema | Codegen regressions, parser changes, example breakage |
| **bench** | `cargo bench --no-run` + `forjar bench --iterations 10 --json` | Compile errors in benchmarks, smoke-test performance |

The dogfood job is particularly valuable — it validates that every resource type's codegen produces parseable configs and that all examples demonstrate working code paths. Any change to parser validation, resource handlers, or template resolution that breaks a dogfood config or example will fail this job.

### Pre-Deploy Checklist

Before deploying to production, run this sequence:

```bash
# 1. Static checks
forjar validate -f forjar.yaml
forjar lint -f forjar.yaml
forjar fmt -f forjar.yaml --check

# 2. Preview changes
forjar plan -f forjar.yaml

# 3. Review generated scripts
forjar plan -f forjar.yaml --output-dir /tmp/review-scripts
ls /tmp/review-scripts/

# 4. Apply with auto-commit
forjar apply -f forjar.yaml --auto-commit

# 5. Post-apply drift check
forjar drift -f forjar.yaml --tripwire
```

## Monitoring After Deployment

### Scheduled Drift Detection

```bash
# Cron: check drift every 15 minutes
*/15 * * * * forjar drift -f /opt/infra/forjar.yaml \
  --tripwire --alert-cmd "/opt/scripts/alert.sh" \
  >> /var/log/forjar-drift.log 2>&1
```

### Anomaly Detection

```bash
# Weekly anomaly scan
forjar anomaly --state-dir state --json | jq '.anomalies'
```

### History Audit

```bash
# Last 20 applies
forjar history -n 20

# JSON for dashboards
forjar history --json | jq '.events[] | {ts: .ts, event: .event}'
```

## Script Auditing

Before running generated scripts on production machines, review them:

```bash
# Export all scripts to a directory
forjar plan -f forjar.yaml --output-dir /tmp/audit-scripts

# Review structure
tree /tmp/audit-scripts/
# /tmp/audit-scripts/
# ├── intel/
# │   ├── bash-aliases.sh
# │   ├── cargo-tools.sh
# │   └── nfs-mount.sh
# └── web-server/
#     ├── nginx-config.sh
#     └── ssl-cert.sh

# Audit a specific script
cat /tmp/audit-scripts/web-server/nginx-config.sh
```

Every script starts with `set -euo pipefail` for safety. File resources use heredocs with single-quoted delimiters to prevent variable expansion. Service resources include systemd detection guards.

### Reviewing Templates

Templates are resolved before script generation. To verify resolution:

```bash
# Show resolved config (templates expanded)
forjar show -f forjar.yaml --json | jq '.resources["nginx-config"].content'

# Compare raw config vs resolved
diff <(grep content forjar.yaml) <(forjar show -f forjar.yaml --json | jq -r '.resources["nginx-config"].content')
```

## Testing Strategies

### Canary Deploys

Use tags to test changes on a subset of machines first:

```yaml
resources:
  nginx-config:
    type: file
    machine: all-webservers
    path: /etc/nginx/nginx.conf
    content: |
      worker_processes auto;
    tags: [web, canary]

  nginx-service:
    type: service
    machine: all-webservers
    name: nginx
    restart_on: [nginx-config]
    tags: [web, canary]
```

```bash
# Deploy to canary first
forjar apply -f forjar.yaml --tag canary -m canary-web1

# Verify
forjar drift -f forjar.yaml --tripwire -m canary-web1

# If good, deploy to all
forjar apply -f forjar.yaml --tag web
```

### Idempotency Testing

Verify that applying twice produces no changes:

```bash
# First apply
forjar apply -f forjar.yaml --state-dir /tmp/idem-test

# Second apply — should show 0 changes
OUTPUT=$(forjar apply -f forjar.yaml --state-dir /tmp/idem-test 2>&1)
echo "$OUTPUT"

# Verify
if echo "$OUTPUT" | grep -q "0 changed"; then
  echo "PASS: Idempotent"
else
  echo "FAIL: Not idempotent"
  exit 1
fi
```

### Drift Testing

Intentionally introduce drift and verify detection:

```bash
# Apply config
forjar apply -f forjar.yaml

# Introduce drift (modify a managed file)
ssh web-server "echo 'rogue change' >> /etc/nginx/nginx.conf"

# Detect drift
forjar drift -f forjar.yaml -m web-server

# Auto-remediate
forjar drift -f forjar.yaml --auto-remediate -m web-server

# Verify drift is resolved
forjar drift -f forjar.yaml --tripwire -m web-server
```

## GitOps Workflow

### PR-Based Infrastructure Changes

```
Developer                    CI                          Production
    │                        │                               │
    ├── edit forjar.yaml ──► │                               │
    │                        ├── validate ──► ✓               │
    │                        ├── lint ──► ✓                   │
    │                        ├── plan ──► PR comment          │
    │                        ├── graph ──► PR summary         │
    │   ◄── review plan ─── │                               │
    │                        │                               │
    ├── merge PR ──────────► │                               │
    │                        ├── apply ──────────────────────► │
    │                        ├── drift --tripwire ──────────► │
    │                        ├── commit state ──► git          │
    │                        │                               │
```

### Post-Merge CI Job

```yaml
name: Deploy Infrastructure

on:
  push:
    branches: [main]
    paths: ['forjar.yaml', 'recipes/**']

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install forjar
        run: cargo install forjar

      - name: Apply changes
        run: |
          forjar apply -f forjar.yaml \
            --state-dir state/ \
            --auto-commit
        env:
          SSH_PRIVATE_KEY: ${{ secrets.DEPLOY_KEY }}

      - name: Post-apply drift check
        run: |
          forjar drift -f forjar.yaml \
            --state-dir state/ --tripwire

      - name: Push state
        run: |
          git push origin main
```

## Testing Best Practices

1. **Validate on every PR** — `forjar validate && forjar lint && forjar fmt --check` in CI.

2. **Review plans, not diffs** — `forjar plan` shows the actual impact. YAML diffs miss template resolution and dependency effects.

3. **Test idempotency** — Apply twice; the second apply should show zero changes.

4. **Use ephemeral containers** — Integration tests in containers are cheap and fast. Never test on production.

5. **Monitor drift continuously** — `--tripwire` in cron catches unauthorized changes early.

6. **Audit scripts before apply** — `forjar plan --output-dir` lets you review the exact shell scripts before they run on your machines.

7. **Use tags for staged rollouts** — `forjar apply --tag canary` applies only to canary resources first.

8. **Keep state in git** — Commit state after every apply. This gives you rollback, audit trail, and diff for free.

9. **Test templates separately** — Use `forjar show --json` to verify template resolution before applying.

10. **Set up scheduled drift checks** — A cron job running `forjar drift --tripwire` catches unauthorized changes within minutes.

## Testing Patterns

### Unit Testing Configs

Test individual config files without applying them:

```bash
#!/bin/bash
# test-configs.sh — validate all config variants

set -euo pipefail

for config in configs/*.yaml; do
    echo "Testing: $config"
    forjar validate -f "$config" || { echo "FAIL: $config"; exit 1; }
    forjar lint -f "$config" || { echo "LINT FAIL: $config"; exit 1; }
    forjar plan -f "$config" --state-dir /dev/null 2>/dev/null || true
    echo "  OK"
done

echo "All configs valid."
```

### Snapshot Testing

Compare plan output against a known-good baseline:

```bash
#!/bin/bash
# snapshot-test.sh

# Generate current plan
forjar plan -f forjar.yaml --state-dir state/ > /tmp/current-plan.txt

# Compare to baseline
if diff -q snapshots/expected-plan.txt /tmp/current-plan.txt > /dev/null 2>&1; then
    echo "Plan matches snapshot."
else
    echo "Plan changed!"
    diff snapshots/expected-plan.txt /tmp/current-plan.txt
    echo ""
    echo "If this is expected, update the snapshot:"
    echo "  cp /tmp/current-plan.txt snapshots/expected-plan.txt"
    exit 1
fi
```

### Container Integration Test Matrix

Test all resource types in containers:

```yaml
# test-matrix.yaml
version: "1.0"
name: integration-tests

machines:
  test:
    hostname: test
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      ephemeral: true
      privileged: true    # Needed for service tests

resources:
  # Package install
  test-packages:
    type: package
    machine: test
    provider: apt
    packages: [curl, jq]

  # File creation
  test-file:
    type: file
    machine: test
    path: /etc/test/config.yaml
    content: "test: true"
    mode: "0644"
    depends_on: [test-packages]

  # Directory creation
  test-dir:
    type: file
    machine: test
    state: directory
    path: /opt/test-app
    mode: "0755"

  # User creation
  test-user:
    type: user
    machine: test
    name: testuser
    shell: /bin/bash
    home: /home/testuser
```

Run the test matrix:

```bash
# Apply
forjar apply -f test-matrix.yaml --state-dir /tmp/test-state/

# Verify idempotency
forjar apply -f test-matrix.yaml --state-dir /tmp/test-state/
# Should show: 0 converged, N unchanged, 0 failed

# Check for drift
forjar drift -f test-matrix.yaml --state-dir /tmp/test-state/

# Clean up (ephemeral: true handles container cleanup)
```

### GPU Container Testing

Test GPU workloads across vendors using forjar's multi-vendor GPU container transport:

```bash
# Run GPU-specific integration tests (requires Docker + GPU drivers)
cargo test --features gpu-container-test
```

The GPU test matrix covers 7 tests across 2 vendors:

| Test | Vendor | What it verifies |
|------|--------|-----------------|
| CUDA lifecycle | NVIDIA | ensure/exec/cleanup with `--gpus all` |
| nvidia-smi | NVIDIA | GPU visible and queryable in container |
| CUDA env vars | NVIDIA | `CUDA_VISIBLE_DEVICES` passed via `--env` |
| ROCm lifecycle | AMD | ensure/exec/cleanup with `--device /dev/kfd /dev/dri` |
| ROCm devices | AMD | `/dev/kfd` and `/dev/dri` accessible in container |
| ROCm env vars | AMD | `ROCR_VISIBLE_DEVICES` passed via `--env` |
| Cross-vendor | Both | Identical config deployed to both CUDA and ROCm |

To test a model QA workflow across GPU vendors:

```bash
# Validate the multi-GPU dogfood configs
forjar validate -f examples/dogfood-multi-gpu.yaml
forjar validate -f examples/dogfood-apr-qa.yaml

# Plan without applying (preview resource creation)
forjar plan -f examples/dogfood-apr-qa.yaml --state-dir /tmp/gpu-qa

# Run the GPU example (no Docker required)
cargo run --example gpu_container_transport
```

### Testing Recipes in Isolation

Test recipes independently before using them in production configs:

```yaml
# test-recipe.yaml
version: "1.0"
name: recipe-test

machines:
  test:
    hostname: test
    addr: container
    transport: container
    container:
      image: ubuntu:22.04
      ephemeral: true

resources:
  web:
    type: recipe
    machine: test
    recipe: web-server
    inputs:
      domain: test.example.com
      port: 8080
      log_level: debug
```

```bash
# Validate recipe expansion
forjar validate -f test-recipe.yaml

# View expanded resources
forjar graph -f test-recipe.yaml

# Apply in container
forjar apply -f test-recipe.yaml --state-dir /tmp/recipe-test/
```

## Monitoring and Alerting

### Prometheus Metrics

Export forjar drift status as Prometheus metrics:

```bash
#!/bin/bash
# forjar-exporter.sh — run as a cron job, write to textfile collector

METRICS_FILE="/var/lib/prometheus/node-exporter/forjar.prom"

drift_json=$(forjar drift -f /opt/infra/forjar.yaml --state-dir /opt/infra/state/ --json 2>/dev/null)
drift_count=$(echo "$drift_json" | jq '.findings | length' 2>/dev/null || echo 0)

cat > "$METRICS_FILE" <<EOF
# HELP forjar_drift_count Number of resources with detected drift
# TYPE forjar_drift_count gauge
forjar_drift_count $drift_count
EOF
```

### Slack/Discord Notifications

Alert when drift is detected:

```bash
#!/bin/bash
# drift-alert.sh — called by forjar drift --alert-cmd

WEBHOOK_URL="https://hooks.slack.com/services/..."
DRIFT_COUNT="${FORJAR_DRIFT_COUNT:-0}"

if [ "$DRIFT_COUNT" -gt 0 ]; then
    curl -s -X POST "$WEBHOOK_URL" \
        -H 'Content-type: application/json' \
        -d "{\"text\": \"Forjar drift alert: $DRIFT_COUNT resources drifted\"}"
fi
```

Use with forjar:

```bash
forjar drift -f forjar.yaml --state-dir state/ --alert-cmd "./drift-alert.sh"
```

## GitOps Workflow

### Pull Request Validation

Run forjar in CI on every pull request:

```yaml
# .github/workflows/validate.yml
name: Validate Infrastructure
on: [pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install forjar
        run: cargo install forjar

      - name: Validate
        run: forjar validate -f forjar.yaml

      - name: Lint
        run: forjar lint -f forjar.yaml

      - name: Format check
        run: forjar fmt -f forjar.yaml --check

      - name: Preview plan
        run: |
          forjar plan -f forjar.yaml --state-dir state/
        # Plan output appears in CI logs for review
```

### Auto-Deploy on Merge

Deploy automatically when changes merge to main:

```yaml
# .github/workflows/deploy.yml
name: Deploy Infrastructure
on:
  push:
    branches: [main]
    paths: ['forjar.yaml', 'recipes/**']

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: production
    steps:
      - uses: actions/checkout@v4

      - name: Setup SSH
        run: |
          mkdir -p ~/.ssh
          echo "${{ secrets.DEPLOY_KEY }}" > ~/.ssh/id_ed25519
          chmod 600 ~/.ssh/id_ed25519

      - name: Install and apply
        run: |
          cargo install forjar
          forjar drift -f forjar.yaml --state-dir state/ --tripwire || true
          forjar apply -f forjar.yaml --state-dir state/
          forjar drift -f forjar.yaml --state-dir state/ --tripwire

      - name: Commit state
        run: |
          git add state/
          git commit -m "forjar: deploy $(date -I)" || true
          git push
```

## Property-Based Testing

Forjar uses `proptest` for property-based testing in critical paths:

### Hash Properties

```rust
proptest! {
    #[test]
    fn hash_deterministic(s in ".*") {
        let h1 = hash_string(&s);
        let h2 = hash_string(&s);
        assert_eq!(h1, h2, "same input must produce same hash");
    }

    #[test]
    fn hash_prefix(s in ".*") {
        let h = hash_string(&s);
        assert!(h.starts_with("blake3:"), "hash must have blake3: prefix");
    }
}
```

### Template Resolution Properties

```rust
proptest! {
    #[test]
    fn no_template_no_change(s in "[^{]*") {
        // Strings without {{ are returned unchanged
        let result = resolve_template(&s, &params, &machines).unwrap();
        assert_eq!(result, s);
    }
}
```

### DAG Properties

```rust
proptest! {
    #[test]
    fn topological_order_contains_all(resources in resource_set(1..10)) {
        let order = build_execution_order(&config).unwrap();
        assert_eq!(order.len(), resources.len());
    }
}
```

## Test Organization

### Test Categories

| Category | Location | Count | What it Tests |
|----------|----------|-------|---------------|
| Unit | `src/*/tests` | ~1400 | Individual functions |
| Integration | `src/core/executor.rs` | ~80 | Multi-component workflows |
| Property | Various `proptest!` blocks | ~50 | Invariant properties |
| Dogfood | `examples/dogfood-*.yaml` | 13 configs | Real-world validation |
| Examples | `examples/*.rs` | 19 | API documentation |

### Naming Conventions

Tests follow the pattern `test_<ticket>_<description>`:

```rust
#[test]
fn test_fj003_resolve_params() { ... }        // FJ-003: Template resolution
fn test_fj005_check_script_package() { ... }   // FJ-005: Codegen
fn test_fj016_detect_drift_file() { ... }      // FJ-016: Drift detection
fn test_fj132_hash_sensitivity() { ... }       // FJ-132: Coverage push
```

### Running Specific Test Categories

```bash
# Run all tests for a specific ticket
cargo test test_fj003

# Run tests for a specific module
cargo test core::parser

# Run tests matching a pattern
cargo test drift

# Run with output for debugging
cargo test test_fj005_check_script -- --nocapture
```

## bashrs Purifier Testing

Forjar generates shell scripts for every resource (check, apply, state_query). The `bashrs` integration ensures these scripts are safe before execution. Testing the purifier pipeline validates that codegen output passes lint, parse, and purification stages.

### The Codegen-Purifier-Assert Pattern

Every codegen function should have a corresponding test that feeds its output through bashrs validation. The standard pattern is:

```rust
#[test]
fn test_fj036_codegen_file_check_validates() {
    use crate::core::codegen;
    let r = make_test_resource(crate::core::types::ResourceType::File);
    let script = codegen::check_script(&r).unwrap();
    assert!(validate_script(&script).is_ok(), "file check failed bashrs");
}
```

The three steps are:

1. **Codegen**: Call `check_script()`, `apply_script()`, or `state_query_script()` with a test resource
2. **Purifier**: Pass the generated script through `validate_script()` (lint-only) or `purify_script()` (full AST pipeline)
3. **Assert**: Verify the script passes without errors

### Testing All Three Script Types

Each resource type generates three scripts. Test all of them:

```rust
#[test]
fn test_fj036_codegen_service_all_validate() {
    use crate::core::codegen;
    let mut r = make_test_resource(crate::core::types::ResourceType::Service);
    r.name = Some("nginx".to_string());
    r.state = Some("running".to_string());
    r.enabled = Some(true);
    for (kind, result) in [
        ("check", codegen::check_script(&r)),
        ("apply", codegen::apply_script(&r)),
        ("state_query", codegen::state_query_script(&r)),
    ] {
        let script = result.unwrap();
        assert!(
            validate_script(&script).is_ok(),
            "service {kind} failed bashrs"
        );
    }
}
```

### Validation Levels

The purifier exposes three levels of strictness. Choose the appropriate one for your test:

| Function | What It Checks | When to Use |
|----------|---------------|-------------|
| `validate_script()` | Lint errors only (warnings pass) | Standard codegen validation |
| `lint_script()` | Returns all diagnostics (errors + warnings) | Audit diagnostic counts |
| `purify_script()` | Full parse, AST purification, reformat, re-validate | Strongest guarantee; tests AST round-trip |

For most codegen tests, `validate_script()` is sufficient. Use `purify_script()` when testing scripts that must survive the full bashrs pipeline (e.g., scripts that will be exported for manual review).

### Testing Diagnostic Expectations

Some resource types intentionally produce bashrs warnings. For example, package scripts use the `$SUDO` pattern which triggers SEC002:

```rust
#[test]
fn test_fj036_lint_codegen_package_has_diagnostics() {
    use crate::core::codegen;
    let mut r = make_test_resource(crate::core::types::ResourceType::Package);
    r.provider = Some("apt".to_string());
    r.packages = vec!["curl".to_string()];
    let script = codegen::apply_script(&r).unwrap();
    let result = lint_script(&script);
    // Package scripts have $SUDO pattern — expect some diagnostics
    assert!(
        !result.diagnostics.is_empty(),
        "apt scripts should have lint findings"
    );
}
```

This tests the inverse: that known-safe patterns produce warnings (not errors) and that the warning count is stable.

### Adding Tests for New Resource Types

When adding a new resource handler, follow this checklist:

1. Create a `make_test_resource()` fixture for the new type
2. Write `test_fj036_codegen_<type>_check_validates` — check script passes `validate_script()`
3. Write `test_fj036_codegen_<type>_apply_validates` — apply script passes `validate_script()`
4. Write `test_fj036_codegen_<type>_state_query_validates` — state query passes `validate_script()`
5. If the handler uses `$SUDO`, write a diagnostic count test
6. If the handler generates heredocs, test with content containing shell metacharacters

### Running Purifier Tests

```bash
# Run all purifier tests
cargo test purifier

# Run all FJ-036 codegen integration tests
cargo test test_fj036

# Run with output to see diagnostic details
cargo test test_fj036 -- --nocapture
```

## Falsification Testing Methodology

Forjar uses a falsification-first testing methodology for critical invariants. Instead of testing that code "works correctly," falsification tests attempt to disprove a stated contract. If the test cannot falsify the property across thousands of random inputs, the property holds with high confidence.

### The FALSIFY Naming Convention

Falsification tests follow the naming pattern `FALSIFY-<DOMAIN>-<SEQ>`:

| Domain | Full Name | Module |
|--------|-----------|--------|
| **B3** | BLAKE3 State Contract | `src/tripwire/hasher.rs` |
| **DAG** | DAG Ordering Contract | `src/core/resolver.rs` |
| **CD** | Codegen Dispatch Contract | `src/core/codegen.rs` |
| **ES** | Execution Safety Contract | `src/core/state.rs`, `src/core/executor.rs` |
| **RD** | Recipe Determinism Contract | `src/core/recipe.rs` |

Each FALSIFY test has a doc comment stating the exact property being tested:

```rust
/// FALSIFY-B3-001: hash_string always produces "blake3:" prefix + 64 hex chars.
#[test]
fn falsify_b3_001_hash_string_prefix_format(s in ".*") {
    let h = hash_string(&s);
    prop_assert!(h.starts_with("blake3:"), "missing prefix");
    prop_assert_eq!(h.len(), 71, "expected 7 prefix + 64 hex = 71 chars");
}
```

### How proptest Drives Falsification

FALSIFY tests use the `proptest` crate to generate random inputs. proptest attempts to find counterexamples that violate the stated property:

```rust
proptest! {
    /// FALSIFY-B3-002: hash_string is deterministic.
    #[test]
    fn falsify_b3_002_hash_string_determinism(s in ".*") {
        let h1 = hash_string(&s);
        let h2 = hash_string(&s);
        prop_assert_eq!(h1, h2, "hash_string must be deterministic");
    }
}
```

By default, proptest runs 256 cases per test. For critical properties, this provides strong evidence that the invariant holds. If proptest finds a counterexample, it shrinks the input to the minimal failing case, making debugging straightforward.

### Key Falsification Properties in Forjar

**BLAKE3 hashing (FALSIFY-B3-\*)**:
- B3-001: Output format is always `blake3:` + 64 hex characters
- B3-002: Same input always produces the same hash
- B3-003: `composite_hash` is order-sensitive (hash(a,b) != hash(b,a))

**DAG ordering (FALSIFY-DAG-\*)**:
- DAG-001: Every dependency appears before its dependent in topological order
- DAG-002: Cycles are always detected and return `Err`
- DAG-003: Same graph always produces the same ordering (deterministic tie-breaking)

**Codegen dispatch (FALSIFY-CD-\*)**:
- CD-001: All Phase 1 resource types produce `Ok` for check, apply, and state_query
- CD-002: Dispatch is symmetric -- every type handled by `check_script` is also handled by `apply_script` and `state_query_script`

**Execution safety (FALSIFY-ES-\*)**:
- ES-001: Atomic write leaves no temp file after success
- ES-002: Jidoka `StopOnFirst` error policy always returns `should_stop=true`
- ES-003: Jidoka `ContinueIndependent` policy returns `should_stop=false`

**Recipe determinism (FALSIFY-RD-\*)**:
- RD-001: `expand_recipe` is deterministic -- same arguments always produce same output
- RD-002: `validate_int` rejects values outside declared `[min, max]`
- RD-003: Path validation rejects non-absolute paths
- RD-004: External dependencies are only injected into the first expanded resource

### Writing New Falsification Tests

When adding a new critical invariant, follow this structure:

1. **Name it**: Choose the domain prefix (or create a new one) and assign the next sequence number
2. **State the property**: Write a doc comment that precisely describes what should never be violated
3. **Choose the input strategy**: Use proptest's `in` syntax to describe the input domain
4. **Assert with `prop_assert!`**: Use proptest assertions, not `assert!`, for proper shrinking

```rust
proptest! {
    /// FALSIFY-XX-NNN: <precise statement of the property>.
    #[test]
    fn falsify_xx_nnn_description(input in "<regex-strategy>") {
        let result = function_under_test(&input);
        prop_assert!(<condition>, "violation: {}", result);
    }
}
```

Use `prop_assume!()` to skip inputs that are not relevant to the property (e.g., skip identical values when testing order sensitivity).

### Running Falsification Tests

```bash
# Run all falsification tests
cargo test falsify

# Run a specific domain
cargo test falsify_b3
cargo test falsify_dag
cargo test falsify_es

# Increase case count for higher confidence (slow)
PROPTEST_CASES=10000 cargo test falsify

# Show shrunk counterexamples on failure
cargo test falsify -- --nocapture
```

### Relationship to Regular Tests

Falsification tests complement, but do not replace, deterministic unit tests. The `test_fj<NNN>_*` tests verify specific, known scenarios. The `falsify_*` tests probe for unknown edge cases through random generation. Both are required for critical modules.

| Test Type | Convention | Purpose | Input |
|-----------|-----------|---------|-------|
| Unit | `test_fj003_resolve_params` | Known scenario verification | Fixed inputs |
| Falsification | `falsify_b3_001_hash_prefix` | Property violation search | Random inputs |
| Integration | `test_fj036_codegen_*_validates` | Cross-module contract | Constructed fixtures |

## Coverage Workflow

### Measuring Coverage

```bash
# Summary coverage report
cargo llvm-cov --summary-only

# HTML report for detailed analysis
cargo llvm-cov --html
open target/llvm-cov/html/index.html

# Coverage for specific test
cargo llvm-cov --test integration -- test_name
```

### Coverage Targets

| Module | Target | Rationale |
|--------|--------|-----------|
| core/parser.rs | 95% | Config correctness is critical |
| core/resolver.rs | 90% | Template bugs cause silent failures |
| core/planner.rs | 90% | Wrong plan = wrong apply |
| resources/* | 85% | Script generation must be correct |
| transport/* | 80% | I/O-heavy, some paths need real SSH |
| cli/* | 70% | UI code, harder to unit test |

## Performance Benchmarking

Forjar includes Criterion benchmarks that validate the performance targets from spec §9. Run them with:

```bash
cargo bench
```

### Benchmark Groups

**Primitive operations** (`blake3_string`, `blake3_file`):
- BLAKE3 string hashing: 64B to 4KB inputs
- BLAKE3 file hashing: 1KB to 1MB files
- Validates the "microseconds" hash speed claim

**YAML parsing** (`yaml_parse_config`):
- Parse a realistic 3-machine, 3-resource config
- Validates parse speed independent of validation

**DAG topological sort** (`topo_sort`):
- Kahn's algorithm on 10/50/100 node chains
- Validates linear scaling of dependency resolution

**Spec §9 targets** (`spec9_*`):
- `spec9_validate_3m_20r` — Parse + validate a 3-machine, 20-resource config
- `spec9_plan_3m_20r` — Full plan pipeline: parse → resolve DAG → diff state
- `spec9_apply_no_changes_3m_20r` — Full apply pipeline with converged locks (no-op path)
- `spec9_drift_100_resources` — Load lock file + detect drift on 100 resources
- `validate_scaling` — Parse+validate scaling: 5/20/50/100 resources

### Performance Targets

| Operation | Spec Target | Measured | Margin |
|-----------|-------------|----------|--------|
| `forjar validate` (3m, 20r) | < 10ms | ~62µs | 161x |
| `forjar plan` (3m, 20r) | < 2s | ~84µs | 23,810x |
| `forjar apply` (no changes, 3m/20r) | < 500ms | ~194µs | 2,577x |
| `forjar drift` (100 resources) | < 1s | ~356µs | 2,809x |
| Binary size (release, stripped) | < 15MB | ~13MB | 1.2x |
| Cold start (`--help`) | < 5ms | ~1.8ms | 2.8x |

### Running Specific Benchmarks

```bash
# Run only spec §9 target benchmarks
cargo bench -- spec9

# Run only scaling benchmarks
cargo bench -- validate_scaling

# Run only BLAKE3 benchmarks
cargo bench -- blake3
```

### Regression Detection

Criterion automatically saves baseline results in `target/criterion/`. To compare against a baseline:

```bash
# Save baseline
cargo bench -- --save-baseline main

# After changes, compare
cargo bench -- --baseline main
```

Any regression > 5% will be flagged in the Criterion report. For CI integration, add a benchmark step that fails on regression:

```yaml
# .github/workflows/bench.yml
- name: Benchmark
  run: cargo bench -- --output-format bencher | tee bench-output.txt
```

## Property-Based Idempotency Testing

Forjar uses [proptest](https://crates.io/crates/proptest) to verify idempotency properties that must hold for all inputs, not just specific test cases.

### Core Properties

Three idempotency properties are tested with random inputs:

1. **BLAKE3 hash idempotency**: Same content always produces the same hash. This ensures the planner's skip-if-converged logic is correct.

2. **Lock file serde roundtrip**: Serializing a `StateLock` to YAML and deserializing it back produces an identical struct. This ensures state is never lost during save/load cycles.

3. **Converged-state-is-noop**: When a resource's current hash matches the desired hash, no action is needed. This is the fundamental idempotency guarantee.

### Template Determinism

Template resolution (`{{params.key}}`) is also property-tested:

- Same params always resolve to the same output
- Missing params consistently produce errors
- Literal strings (no `{{...}}`) pass through unchanged

### Running Property Tests

```bash
# Run all proptest tests
cargo test proptest

# Run with more cases (default: 256)
PROPTEST_CASES=1000 cargo test proptest
```

Property tests generate random `Resource`, `StateLock`, and `GlobalLock` values using strategies defined in `src/core/types/tests_proptest_resource.rs`.

## Resource Coverage Model (FJ-2605)

Every resource type has a five-level coverage maturity scale:

| Level | Name | What It Proves |
|-------|------|---------------|
| L0 | No tests | Resource is completely untested |
| L1 | Unit tested | Codegen produces valid script, planner produces correct action |
| L2 | Behavior spec | YAML `.spec.yaml` with verify commands |
| L3 | Convergence tested | Apply-verify-reapply-verify in sandbox |
| L4 | Mutation tested | All applicable mutations detected |
| L5 | Preservation tested | Pairwise preservation with co-located resources |

```bash
cargo run --example coverage_model
```

## Behavior-Driven Infrastructure Specs (FJ-2602)

Behavior specs describe **what the system should look like after convergence**, not how to get there. They use `.spec.yaml` files with verifiable assertions.

### Spec Format

```yaml
# tests/behaviors/nginx-web-server.spec.yaml
name: nginx web server
config: examples/nginx.yaml
machine: web-1

behaviors:
  - name: nginx package is installed
    resource: nginx-pkg
    state: present
    verify:
      command: "dpkg -l nginx | grep -q '^ii'"
      exit_code: 0

  - name: nginx service is running
    resource: nginx-service
    state: running
    verify:
      command: "systemctl is-active nginx"
      stdout: "active"

  - name: idempotency holds
    type: convergence
    convergence:
      second_apply: noop
      state_unchanged: true
```

### Assertion Types

| Type | Field | Description |
|------|-------|-------------|
| State | `state` | Expected resource state (present, running, file) |
| Command | `verify.command` | Shell command with expected exit code |
| Stdout | `verify.stdout` | Expected stdout content |
| Stderr | `verify.stderr_contains` | Expected substring in stderr |
| File | `verify.file_exists` | Path must exist on target |
| Port | `verify.port_open` | TCP port accepting connections |
| Convergence | `type: convergence` | Second apply is no-op |

### Soft Assertions

Behavior specs use **soft assertions** — all behaviors are checked before reporting, even if earlier ones fail. This gives a complete picture of the system state.

```bash
cargo run --example behavior_spec
```

## Run Log Capture (FJ-2301)

Every `forjar apply` creates a **run log** that captures the full output of every script execution. See the types in action:

```bash
cargo run --example run_log
```

Run logs use structured delimiters for machine-parseable output:

```
=== FORJAR TRANSPORT LOG ===
resource: cargo-tools
type: package
action: apply

=== SCRIPT ===
apt-get install -y cargo-watch

=== STDOUT ===
Reading package lists...

=== STDERR ===
E: Unable to locate package cargo-watch

=== RESULT ===
exit_code: 100
duration_secs: 1.800
```

### Retention Policy

Log storage is bounded by configurable retention:

| Setting | Default | Description |
|---------|---------|-------------|
| `keep_runs` | 10 | Last N runs per machine |
| `keep_failed` | 50 | Failed runs kept regardless |
| `max_log_size` | 10 MB | Per-log file truncation |
| `max_total_size` | 500 MB | Total budget per machine |

## Infrastructure Mutation Testing (FJ-2604)

Verify drift detection catches real-world infrastructure mutations:

```bash
cargo run --example mutation_testing
```

### Mutation Operators

Eight mutation operators simulate common infrastructure drift:

| Operator | Targets | What It Tests |
|----------|---------|---------------|
| `delete_file` | file | File presence detection |
| `modify_content` | file | Content hash comparison |
| `change_permissions` | file | Mode drift detection |
| `stop_service` | service | Service state detection |
| `remove_package` | package | Package presence detection |
| `kill_process` | service | Process recovery |
| `unmount_filesystem` | mount | Mount state detection |
| `corrupt_config` | file | Partial content modification |

### Mutation Score

```
Grade A: >= 90% mutations detected
Grade B: >= 80%
Grade C: >= 60%
Grade F: < 60% — drift detection broken
```

`MutationReport::from_results()` builds per-type summaries and lists undetected mutations for targeted improvement.

## Build Metrics & Size Regression (FJ-2403)

Track binary sizes across releases and detect regressions automatically:

```bash
cargo run --example build_metrics
```

### BuildMetrics

`BuildMetrics::current()` captures compile-time metadata: version, target triple, profile (debug/release), LTO status, and Rust toolchain version. After building, populate `binary_size` and `dependency_count` for tracking.

### SizeThreshold

`SizeThreshold` defines two limits:

| Setting | Default | Description |
|---------|---------|-------------|
| `max_bytes` | 10 MB | Absolute binary size limit |
| `max_growth_pct` | 10% | Maximum growth from previous release |

```rust
let threshold = SizeThreshold::default();
let violations = threshold.check(&current_build, Some(&previous_build));
if !violations.is_empty() {
    // Block release: binary size regressed
}
```

Use in CI to fail the build when the binary grows beyond acceptable bounds.
