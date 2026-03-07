# 14: Testing Strategy

> Convergence verification, idempotency testing, behavior-driven infrastructure testing, and coverage for a world-class IaC platform.

**Spec ID**: FJ-2600–FJ-2607 | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Motivation

Infrastructure as Code testing is fundamentally different from application testing. The "code" is declarative YAML that generates shell scripts, which execute on remote machines, which have pre-existing state. A passing unit test does not mean a resource converges on a real system. A passing integration test does not mean idempotency holds across all possible starting states.

### Research Context

Recent academic work highlights both the need and the difficulty:

- **NSync** (arXiv 2510.20211): Automated IaC reconciliation via API traces — demonstrates that drift detection alone is insufficient; reconciliation requires understanding high-level intent behind infrastructure changes.
- **State Reconciliation Defects** (FSE 2024): Taxonomy of reconciliation bugs in Terraform, CloudFormation, and Pulumi — shows that idempotency violations are the most common defect class.
- **Shambaugh et al.** "Asserting Reliable Convergence for Configuration Management Scripts": Introduced the **preservation property** — if for every pair of resources, applying one doesn't undo the other, then the entire configuration converges. Detected all known idempotency bugs in Puppet, plus previously unknown ones.
- **ConVerTest** (arXiv 2602.10522): Combines self-consistency, chain-of-verification, and dual execution agreement for test generation — achieves 39% improvement in test validity, 28% in coverage, 18% in mutation scores.
- **Meta's LLM Mutation Testing** (FSE 2025): LLM-generated mutations focused on compliance-critical code paths — 73% acceptance rate by engineers, demonstrating that targeted mutation testing at scale is practical.
- **InfraFix** (arXiv 2503.17220): SMT-based IaC repair across 254K scenarios with 95.7% success rate — shows formal methods can scale to real IaC.

### What Exists Today

Forjar has a strong testing foundation:

| Metric | Current |
|--------|---------|
| Unit tests | 8,365 |
| Test files | 525 (`tests_*.rs`) |
| Integration tests | 3 (`tests/`) |
| Property-based tests | 13+ proptest files |
| Falsification specs | 10 files (`tests_falsify_spec_[a-j].rs`) |
| Benchmarks | 3 Criterion harnesses |
| Coverage threshold | 95% lines |
| Mutation testing | cargo-mutants configured |
| CI workflows | 11 (lint, coverage, miri, stress, bench, audit, MSRV, release) |
| Pre-commit hooks | Complexity, file size, SATD |

**What's missing**: No infrastructure-level testing. All 8,365 tests verify Rust code behavior, not whether a resource actually converges on a real (or simulated) system. No convergence property testing. No idempotency verification beyond the Verus model. No behavior-driven infrastructure specs.

---

## Testing Pyramid for IaC

```
                    ┌─────────────┐
                    │  E2E Fleet  │  Real machines, full stack
                    │  (minutes)  │  forjar apply → verify → apply → verify
                    ├─────────────┤
                    │  Sandbox    │  Pepita/container, real scripts
                    │  (seconds)  │  Convergence + idempotency
                    ├─────────────┤
                    │  Behavior   │  YAML specs, simulated transport
                    │  (<1s)      │  Property-based, scenario-driven
                    ├─────────────┤
                    │  Unit       │  Rust functions, mocked state
                    │  (<100ms)   │  Planner, resolver, codegen, parser
                    └─────────────┘
```

| Layer | Count Target | What It Proves |
|-------|-------------|---------------|
| **Unit** | 8,000+ (existing) | Code correctness: parsing, planning, hashing, DAG resolution |
| **Behavior** | 200+ | Specification conformance: resource contracts, state transitions, convergence properties |
| **Sandbox** | 50+ | Real convergence: scripts execute, state persists, idempotency holds |
| **E2E Fleet** | 10+ | Full-stack: multi-machine, transport layers, drift detection, undo |

---

## FJ-2600: Convergence Property Testing

### The Preservation Property

From Shambaugh et al.: A configuration **converges** if applying it repeatedly from any reachable state always reaches the desired state and stays there.

**Formal definition** (adapted for forjar):

```
For all resources R in config:
    For all starting states S:
        apply(R, S) = desired_state(R)           // Convergence
        apply(R, apply(R, S)) = apply(R, S)      // Idempotency
        apply(R, apply(R', S)) = desired_state(R) // Preservation (R' doesn't undo R)
```

### Property-Based Convergence Tests

Using proptest to generate arbitrary starting states:

```rust
proptest! {
    #[test]
    fn convergence_file_resource(
        initial_content in ".*",
        desired_content in ".*",
        initial_mode in 0o000u32..0o777,
        desired_mode in 0o000u32..0o777,
    ) {
        let resource = file_resource(desired_content, desired_mode);
        let state = FileState { content: initial_content, mode: initial_mode };

        // Apply once
        let after_apply = simulate_apply(&resource, &state);
        assert_eq!(after_apply.content, desired_content);
        assert_eq!(after_apply.mode, desired_mode);

        // Apply again — must be no-op
        let after_reapply = simulate_apply(&resource, &after_apply);
        assert_eq!(after_apply, after_reapply, "idempotency violated");
    }
}
```

### Preservation Matrix

For every pair of resource types that can coexist on the same machine, verify that applying resource A doesn't undo resource B:

| A \ B | package | file | service | mount | user | cron | docker | network |
|-------|---------|------|---------|-------|------|------|--------|---------|
| **package** | — | safe | safe | safe | safe | safe | safe | safe |
| **file** | safe | **check** | safe | safe | safe | safe | safe | safe |
| **service** | safe | safe | **check** | safe | safe | safe | safe | safe |
| **mount** | safe | safe | safe | **check** | safe | safe | safe | safe |

**"check"** = potential conflict requiring preservation test (e.g., two file resources writing the same path, service restart affecting another service).

### Convergence Test Runner

```bash
forjar test convergence config.yaml          # Run convergence tests
forjar test convergence config.yaml --pairs  # Preservation matrix
forjar test convergence config.yaml --from-state dirty  # Arbitrary starting states
```

**Implementation**: For each resource, the convergence test runner:
1. Generates the apply script via codegen
2. Generates the state_query script
3. Executes apply in a sandbox (pepita namespace or container)
4. Queries state — verifies it matches desired
5. Executes apply again — verifies it's a no-op (zero changes)
6. Queries state again — verifies unchanged

---

## FJ-2601: Idempotency Verification

### Three Levels of Idempotency

| Level | Definition | How to Test |
|-------|-----------|-------------|
| **Plan idempotency** | Second plan produces all NO-OP | `forjar plan` twice, compare |
| **Script idempotency** | Second script execution has no side effects | Execute script twice, compare filesystem |
| **State idempotency** | State hash unchanged after second apply | `hash_desired_state` unchanged |

### Plan Idempotency Test

```
fn test_plan_idempotency(config):
    plan1 = forjar_plan(config)
    apply(plan1)
    plan2 = forjar_plan(config)
    assert plan2.all_noop(), "second plan should be all NO-OP"
```

### Script Idempotency Test (Sandbox)

```
fn test_script_idempotency(resource):
    sandbox = create_pepita_sandbox()

    // First apply
    script = codegen::apply_script(resource)
    exec(sandbox, script)
    state1 = snapshot_filesystem(sandbox)

    // Second apply
    exec(sandbox, script)
    state2 = snapshot_filesystem(sandbox)

    assert state1 == state2, "filesystem changed on second apply"
```

### Hash Stability Test

```
fn test_hash_stability(resource):
    hash1 = hash_desired_state(resource)
    hash2 = hash_desired_state(resource)
    assert hash1 == hash2, "same input must produce same hash"
```

### Known Idempotency Violations

Document known cases where idempotency doesn't hold:

| Resource | Violation | Reason | Mitigation |
|----------|----------|--------|------------|
| `task` | Always re-executes | Commands are imperative, not declarative | `creates:` guard, `unless:` check |
| `package` (floating version) | `latest` state may change | Upstream version bumped | Pin versions |
| `model` (URL source) | Re-download may differ | Model updated at same URL | BLAKE3 checksum |
| `file` (source: remote) | Source file may change | External dependency | Checksum or content-addressing |

---

## FJ-2602: Behavior-Driven Infrastructure Specs

### What Is Behavior Testing for IaC?

Behavior-Driven Development (BDD) for infrastructure describes **what the system should look like after convergence**, not how to get there. Forjar's YAML is already declarative — behavior specs validate that the declared state matches reality.

### Spec Format

```yaml
# tests/behaviors/nginx-web-server.spec.yaml
name: nginx web server
config: examples/nginx.yaml
machine: web-1

behaviors:
  - name: nginx package is installed
    resource: nginx-pkg
    assert:
      state: present
      verify:
        command: "dpkg -l nginx | grep -q '^ii'"
        exit_code: 0

  - name: nginx config file has correct content
    resource: nginx-config
    assert:
      state: file
      verify:
        command: "nginx -t"
        exit_code: 0
        stderr_contains: "syntax is ok"

  - name: nginx service is running
    resource: nginx-service
    assert:
      state: running
      verify:
        command: "systemctl is-active nginx"
        stdout: "active"

  - name: port 80 is open
    resource: nginx-firewall
    assert:
      verify:
        command: "ss -tlnp | grep ':80'"
        exit_code: 0

  - name: idempotency holds
    type: convergence
    assert:
      second_apply: noop
      state_unchanged: true
```

### Assertion Types

| Type | Description | Implementation |
|------|-------------|----------------|
| `state` | Resource state matches expected | Compare planner output |
| `verify.command` | Shell command passes on target | Execute via transport, check exit code |
| `verify.stdout` | Command output matches | String equality or regex |
| `verify.stderr_contains` | Stderr includes substring | Substring search |
| `verify.file_exists` | Path exists on target | `test -e <path>` |
| `verify.file_content` | File content matches | `cat` + compare or BLAKE3 |
| `verify.port_open` | Port accepting connections | `ss -tlnp` or `nc -z` |
| `convergence` | Second apply is no-op | Full convergence test |

### Running Behavior Specs

```bash
forjar test behavior tests/behaviors/       # Run all behavior specs
forjar test behavior tests/behaviors/ --sandbox pepita  # In pepita namespace
forjar test behavior tests/behaviors/ --sandbox container --image ubuntu:22.04
forjar test behavior tests/behaviors/ --parallel 4      # Parallel sandbox execution
forjar test behavior tests/behaviors/ --json            # Machine-readable output
```

### Borrowing from Probar

Probar's testing innovations applicable to forjar:

| Probar Concept | Forjar Adaptation |
|----------------|-------------------|
| **Soft assertions** | Collect all behavior failures before reporting (don't fail-fast on first broken resource) |
| **Retry with backoff** | Services may take time to start — retry `verify.command` with configurable timeout |
| **Deterministic replay** | Record full apply session (scripts, output, timing) for replay in CI |
| **Playbook testing** | YAML-defined state machines for multi-step infrastructure scenarios |
| **Popperian falsification** | Each behavior spec is a falsifiable hypothesis — design tests to break it |
| **GUI coverage → Resource coverage** | Track which resources have behavior specs (coverage of infrastructure, not code) |

---

## FJ-2603: Sandbox Testing Infrastructure

### Sandbox Types

| Sandbox | Isolation | Speed | Fidelity |
|---------|-----------|-------|----------|
| **Mock** | In-process | <1ms | Low — no real execution |
| **Pepita** | Kernel namespace | 10-50ms | High — real filesystem, real packages |
| **Container** | Docker/Podman | 500ms-1s | High — real OS, network isolation |
| **VM** | Full virtualization | 5-30s | Highest — real kernel, real hardware |

### Pepita Sandbox for Testing

Pepita namespaces are ideal for infrastructure testing — fast, isolated, real filesystem:

```
fn create_test_sandbox() -> PepitaSandbox:
    sandbox = pepita::create(PepitaConfig {
        rootfs: "ubuntu:22.04",  // Base image
        cpuset: "0",             // Single CPU
        memory_limit: "512M",    // Memory cap
        network: false,          // No network by default
        overlay: true,           // Overlay filesystem (changes discarded)
    })
    return sandbox
```

**Key property**: Overlay filesystem means each test starts from a clean state. Changes are isolated to the overlay upper directory and discarded when the sandbox exits.

### Container Sandbox for Testing

For environments without unprivileged user namespaces:

```bash
forjar test --sandbox container --image ubuntu:22.04 --rm
```

Uses `docker run --rm` with bind-mounted config. Container destroyed after test.

### Sandbox Lifecycle

```
1. Create sandbox (pepita namespace or container)
2. Bootstrap: install forjar inside sandbox (or mount binary)
3. Apply config inside sandbox
4. Run verification commands
5. Check convergence (apply again, verify no-op)
6. Capture logs and artifacts
7. Destroy sandbox
```

---

## FJ-2604: Mutation Testing for Infrastructure

### What to Mutate

Traditional mutation testing mutates source code. Infrastructure mutation testing mutates the **target system state** to verify that forjar detects and corrects the mutation.

| Mutation | Description | What It Tests |
|----------|-------------|---------------|
| **Delete file** | Remove a managed file | Drift detection + re-convergence |
| **Modify content** | Change file content | Content hash comparison |
| **Change permissions** | Alter file mode/owner | Mode drift detection |
| **Stop service** | `systemctl stop <service>` | Service state detection |
| **Remove package** | `apt remove <pkg>` | Package presence detection |
| **Change config** | Modify a managed config file | Content-addressed drift |
| **Kill process** | Kill a managed process | Service recovery |
| **Unmount filesystem** | `umount <path>` | Mount state detection |

### Infrastructure Mutation Runner

```bash
forjar test mutate config.yaml --sandbox pepita   # Run mutation tests
forjar test mutate config.yaml --mutations 50     # Number of mutations per resource
forjar test mutate config.yaml --report html      # Generate mutation report
```

**Algorithm:**

```
for each resource R in config:
    for each mutation M applicable to R:
        sandbox = create_sandbox()
        apply(config, sandbox)                    // Establish baseline
        apply_mutation(M, sandbox)                // Mutate target state
        drift = forjar_drift(config, sandbox)     // Detect drift
        assert drift.detects(R), "mutation {M} on {R} not detected"
        apply(config, sandbox)                    // Re-converge
        verify(config, sandbox)                   // Verify convergence
        destroy(sandbox)
```

### Mutation Score

```
mutation_score = detected_mutations / total_mutations

Grade:
  A: >= 90%   (all mutations detected)
  B: >= 80%   (most mutations detected)
  C: >= 60%   (significant gaps)
  F: < 60%    (drift detection broken)
```

### LLM-Augmented Mutations (Future)

Inspired by Meta's approach: use LLMs to generate realistic infrastructure mutations that mirror real-world drift scenarios:

- Configuration file edits that a human operator might make
- Package upgrades that change config file defaults
- Security patches that modify file permissions
- Container image updates that change environment variables

---

## FJ-2605: Coverage Model

### Four Coverage Dimensions

| Dimension | What It Measures | Tool |
|-----------|-----------------|------|
| **Code coverage** | Rust source lines executed | `cargo llvm-cov` |
| **Resource coverage** | Resources with behavior specs | `forjar test --coverage` |
| **Mutation coverage** | Infrastructure mutations detected | `forjar test mutate --score` |
| **State coverage** | Starting states tested | Proptest + sandbox |

### Code Coverage (Existing)

```bash
cargo llvm-cov --summary-only --fail-under-lines 95
```

Target: 95% line coverage. Enforced in CI.

### Resource Coverage (New)

Every resource type should have:

| Coverage Level | Requirement |
|----------------|-------------|
| **L1: Unit tested** | Codegen produces valid script, planner produces correct action |
| **L2: Behavior spec** | YAML behavior spec with verify commands |
| **L3: Convergence tested** | Apply-verify-reapply-verify in sandbox |
| **L4: Mutation tested** | All applicable mutations detected |
| **L5: Preservation tested** | Pairwise preservation with co-located resources |

```bash
forjar test coverage config.yaml
```

Output:
```
Resource Coverage Report
========================
nginx-pkg:     L4 (mutation tested)
nginx-config:  L3 (convergence tested)
nginx-service: L2 (behavior spec only)
app-deploy:    L1 (unit tested only)
firewall-rule: L0 (no tests)

Coverage: 3/5 at L3+, 1/5 at L4+
Target: 80% at L3+
```

### Mutation Coverage

```bash
forjar test mutate config.yaml --score
```

Output:
```
Mutation Score: 87% (Grade B)
=================================
file resources:    12/12 detected (100%)
service resources:  8/9  detected (89%)
package resources:  6/8  detected (75%)  ← version drift undetected
mount resources:    3/3  detected (100%)

Undetected mutations:
  - package "nginx": version change 1.24→1.25 not detected (floating version)
  - package "curl": removal not detected (not in state_query)
```

---

## FJ-2606: Test Runner Architecture

### `forjar test` Command

```bash
forjar test                                    # Run all test types
forjar test unit                               # Rust unit tests (cargo test)
forjar test behavior <dir>                     # Behavior specs
forjar test convergence <config>               # Convergence property tests
forjar test mutate <config>                    # Infrastructure mutation tests
forjar test coverage <config>                  # Coverage report
forjar test smoke <config>                     # Quick smoke test (validate + plan + dry-run)
```

### Test Output Format

**Default (human-readable):**
```
forjar test behavior tests/behaviors/

  nginx web server
    [PASS] nginx package is installed                    (0.3s)
    [PASS] nginx config file has correct content         (0.2s)
    [PASS] nginx service is running                      (0.5s, 2 retries)
    [FAIL] port 80 is open                               (1.0s)
           Expected: exit_code 0
           Got: exit_code 1
           Command: ss -tlnp | grep ':80'
           Stderr: (empty)
    [PASS] idempotency holds                             (1.2s)

  4 passed, 1 failed (3.2s)
```

**JSON (`--json`):**
```json
{
  "suite": "nginx web server",
  "results": [
    {"name": "nginx package is installed", "status": "pass", "duration_ms": 300},
    {"name": "port 80 is open", "status": "fail", "duration_ms": 1000,
     "expected": {"exit_code": 0}, "actual": {"exit_code": 1},
     "command": "ss -tlnp | grep ':80'", "log_path": "state/web-1/runs/test-001/nginx-firewall.log"}
  ],
  "passed": 4, "failed": 1, "duration_ms": 3200
}
```

### Retry and Timeout

Services and ports may take time to become available after apply:

```yaml
behaviors:
  - name: service is running
    resource: nginx-service
    assert:
      verify:
        command: "systemctl is-active nginx"
        stdout: "active"
        retry:
          max_attempts: 5
          interval: 2s
          backoff: exponential
        timeout: 30s
```

### Parallel Sandbox Execution

Independent resources can be tested in parallel sandboxes:

```bash
forjar test behavior tests/ --parallel 4
```

Each behavior spec gets its own sandbox. Results collected and merged.

---

## FJ-2607: CI Integration

### Test Matrix in CI

| Trigger | Tests Run | Time Budget |
|---------|-----------|-------------|
| Every commit | Unit + lint + format | <2min |
| PR to main | Unit + behavior + convergence (smoke) | <5min |
| Merge to main | Full suite + mutation + coverage | <15min |
| Weekly | Stress + E2E fleet + full mutation | <1hr |
| Release | All + cross-platform + MSRV + Miri | <30min |

### Existing CI Workflows

| Workflow | Purpose | Status |
|----------|---------|--------|
| `clean-room-gate.yml` | Per-PR gate | Production |
| `coverage.yml` | 95% line coverage | Production |
| `lint.yml` | Cross-platform clippy | Production |
| `miri.yml` | UB detection (nightly) | Production |
| `stress.yml` | Weekly race condition hunt | Production |
| `bench.yml` | Performance regression | Production |
| `msrv.yml` | Minimum Rust version | Production |
| `audit.yml` | Security vulnerabilities | Production |
| `release.yml` | Trusted publishing | Production |

### New CI Workflows

| Workflow | Purpose | Trigger |
|----------|---------|---------|
| `behavior.yml` | Behavior spec suite | PR + merge |
| `convergence.yml` | Convergence property tests | Merge + weekly |
| `mutation.yml` | Infrastructure mutation score | Weekly |
| `e2e.yml` | Full-stack fleet test | Weekly |

### Test Artifacts

Every CI test run produces:

| Artifact | Content | Retention |
|----------|---------|-----------|
| `test-results.json` | All test results, machine-readable | 90 days |
| `coverage.lcov` | Line coverage data | 90 days |
| `mutation-report.html` | Mutation score with details | 90 days |
| `sandbox-logs/` | Per-resource logs from sandbox tests | 30 days |
| `behavior-report.html` | Behavior spec results | 90 days |

---

## Performance Targets

| Test Suite | Target Duration | Mechanism |
|------------|----------------|-----------|
| Unit tests | <15s | Parallelized, no I/O |
| Behavior specs (50 specs) | <30s | Parallel sandboxes |
| Convergence (10 resources) | <60s | Pepita namespace per resource |
| Mutation (50 mutations) | <5min | Parallel sandboxes, cached base |
| Full suite | <10min | Parallel everything |

---

## Implementation

### Phase 28: Convergence Property Testing (FJ-2600) -- IMPLEMENTED
- [x] `forjar test convergence` command — wired to `run_convergence_parallel()` with real output
- [x] Proptest generators for resource starting states (arb_convergent_resource)
- [x] Preservation matrix for resource pairs (CONV-003)
- [x] Sandbox integration for real convergence verification (simulated mode)
- [x] Hash stability (CONV-001), plan convergence (CONV-002), hash sensitivity (CONV-006)
- [x] CLI dispatches to `convergence_runner.rs` — builds targets from config, runs parallel tests
- [x] `ConvergenceTestConfig` with `SandboxBackend` field — dispatches via `resolve_mode()`
- [x] `convergence_container.rs` — real container-based convergence testing (ephemeral Docker/Podman)
- **Deliverable**: 6 proptest convergence properties verified (CONV-001 through CONV-006)
- **Note**: Container backend fully operational. Pepita/chroot degrade to simulated mode.

### Phase 29: Idempotency Verification (FJ-2601) -- DONE
- [x] Plan idempotency test (CONV-005: plan twice, second identical)
- [x] Codegen idempotency test (CONV-004: same resource → same script)
- [x] Hash stability test (CONV-001: same input → same hash)
- [x] Known violation documentation per resource type (spec section "Known Idempotency Violations")
- **Deliverable**: Idempotency violations detected before merge

### Phase 30: Behavior-Driven Infrastructure Specs (FJ-2602) -- IMPLEMENTED
- [x] `.spec.yaml` format: `BehaviorSpec`, `BehaviorEntry`, `VerifyCommand`, `ConvergenceAssert` types
- [x] Assertion types: state, verify.command, verify.stdout, verify.stderr_contains, convergence, port_open, file_exists
- [x] Soft assertions: `BehaviorReport::from_results()` collects all failures
- [x] Retry config: `VerifyCommand.retries` and `retry_delay_secs` fields
- [x] `forjar test behavior` CLI — parses .spec.yaml files, executes verify commands via bash, reports per-behavior pass/fail
- [x] `execute_behavior()` — runs `verify.command` via `bash -euo pipefail`, compares exit code and stdout
- **Deliverable**: `forjar test behavior` executes YAML behavior specs with real command verification

### Phase 31: Sandbox Testing Infrastructure (FJ-2603) — IMPLEMENTED
- [x] `SandboxConfig` with backend, cleanup, timeout, capture_overlay
- [x] `SandboxBackend` enum: Pepita, Container, Chroot
- [x] `SandboxPhase` lifecycle: Created → Applied → Verified → Destroyed
- [x] Parallel sandbox execution
- [x] `backend_available()` detection (pepita binary, docker/podman, root check)
- [x] `resolve_mode()` dispatch: Sandbox if backend available, Simulated fallback
- [x] Both convergence and mutation runners use `SandboxBackend` in config
- [x] `convergence_container.rs`: Real container-based convergence testing (ephemeral Docker/Podman)
- [x] `mutation_container.rs`: Real container-based mutation testing (baseline → mutate → drift → re-converge)
- [x] `run_mutation_test_dispatch()` routes to container or simulated based on runtime availability
- **Deliverable**: Tests run in isolated sandboxes with real system state
- **Note**: Container backend fully implemented. Pepita/chroot gracefully degrade to simulated mode.

### Phase 32: Infrastructure Mutation Testing (FJ-2604) -- IMPLEMENTED
- [x] Mutation operator types: `MutationOperator` enum (8 operators with resource type applicability)
- [x] `MutationResult` with detected/reconverged tracking
- [x] `MutationScore` with grade calculation (A/B/C/F)
- [x] `MutationReport` with per-type summaries and undetected mutation listing
- [x] `TypeMutationSummary` with detection percentage
- [x] Mutation runner with parallel sandbox execution (`run_mutation_parallel`)
- [x] CLI dispatches to `mutation_runner.rs` — builds targets from config, reports score/grade
- [x] Undetected mutation reporting in CLI
- [x] `SandboxBackend` in `MutationRunConfig` — dispatches via `resolve_mode()`
- [x] `run_mutation_test_dispatch()` routes to `mutation_container.rs` when Docker/Podman available
- **Deliverable**: `forjar test mutate` with mutation score >= 80%
- **Note**: Container backend dispatches to real execution. Pepita/chroot degrade to simulated.

### Phase 33: Coverage Model (FJ-2605) -- IMPLEMENTED
- [x] Five-level resource coverage tracking (L0-L5)
- [x] `forjar test coverage` report (CoverageReport type)
- [x] Coverage badge: `CoverageBadge` with `BadgeColor::from_pct()` (6 color tiers)
- [x] CI threshold: `CoverageThreshold` with `check(line_pct, branch_pct)` enforcement
- **Deliverable**: Resource coverage report alongside code coverage

### Phase 34: Test Runner and CLI (FJ-2606) — IMPLEMENTED
- [x] `forjar test` types: `TestCommand`, `TestSubcommand` (behavior/convergence/mutation/all)
- [x] `TestResult`, `TestArtifact`, `TestSuiteReport` with `pass_rate()` and `format_summary()`
- [x] Parallel execution engine (`run_tests_parallel` with `thread::scope`)
- [x] Test artifact collection (runtime wiring, `--verbose` flag)
- [x] `--group behavior|mutation|convergence` dispatch to specialized runners
- **Deliverable**: Single command runs all test types

### Phase 35: CI Integration (FJ-2607) -- IMPLEMENTED
- [x] `behavior.yml` workflow — runs behavior specs with artifact upload
- [x] `convergence.yml` workflow — runs proptest convergence + hash stability
- [x] `mutation.yml` workflow — runs cargo-mutants with report upload
- [x] Test artifact upload and retention — all 3 workflows use `upload-artifact@v4` with 14-day retention
- **Deliverable**: Full test pyramid in CI with appropriate triggers

---

## References

- [NSync: Automated Cloud IaC Reconciliation (arXiv 2510.20211)](https://arxiv.org/abs/2510.20211)
- [State Reconciliation Defects in IaC (FSE 2024)](https://dl.acm.org/doi/10.1145/3660790)
- [Shambaugh et al. — Asserting Reliable Convergence for CM Scripts](https://www.researchgate.net/publication/306058901_Asserting_Reliable_Convergence_for_Configuration_Management_Scripts)
- [ConVerTest: Consistency Meets Verification (arXiv 2602.10522)](https://arxiv.org/abs/2602.10522)
- [Meta LLM Mutation Testing (FSE 2025)](https://www.infoq.com/news/2026/01/meta-llm-mutation-testing/)
- [InfraFix: Technology-Agnostic IaC Repair (arXiv 2503.17220)](https://arxiv.org/abs/2503.17220)
- [TerraFormer: LLM IaC with Formal Verification (arXiv 2601.08734)](https://arxiv.org/abs/2601.08734)
- [Static Analysis of IaC: A Survey (arXiv 2206.10344)](https://arxiv.org/abs/2206.10344)
- [MC/DC Coverage for Rust (AIAA 2025)](https://arc.aiaa.org/doi/10.2514/1.I011558)
- [cargo-mutants](https://crates.io/crates/cargo-mutants)
- [Probar Testing Framework](https://github.com/paiml/probar) — Brick architecture, soft assertions, deterministic replay, Popperian falsification
