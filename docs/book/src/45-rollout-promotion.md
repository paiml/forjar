# Progressive Rollout & Promotion Gates

Forjar implements progressive rollout strategies with health check enforcement and quality gates for environment promotion.

## Progressive Rollout (FJ-3507)

Three rollout strategies with health check integration:

| Strategy | Behavior |
|----------|----------|
| `canary` | Deploy to N canary machines first, then percentage steps |
| `percentage` | Deploy in configurable percentage steps (default: 25/50/75/100) |
| `all-at-once` | Deploy to all machines in one step |

### Configuration

```yaml
environments:
  prod:
    promotion:
      from: staging
      rollout:
        strategy: canary
        canary_count: 1
        percentage_steps: [25, 50, 100]
        health_check: "curl -sf http://localhost:8080/health"
        health_timeout: "30s"
```

### Health Check Timeout Enforcement

Health checks are enforced with a configurable timeout. If the command does not complete within the specified duration, the child process is killed and the step fails:

```rust
use forjar::core::rollout::run_health_check;

let (passed, msg) = run_health_check("curl -sf http://localhost/health", Some("10s"));
if !passed {
    println!("Health check failed: {msg}");
}
```

Timeout formats: `30s` (seconds), `5m` (minutes), or bare number (seconds). Default: 30s.

### Auto-Rollback

If a health check fails at any step, the rollout stops and records the failure step for rollback:

```rust
use forjar::core::rollout::execute_rollout;
use forjar::core::types::environment::RolloutConfig;

let config = RolloutConfig {
    strategy: "canary".into(),
    canary_count: 1,
    health_check: Some("curl -sf http://localhost/health".into()),
    health_timeout: Some("10s".into()),
    percentage_steps: vec![50, 100],
};
let result = execute_rollout(&config, 10, false);
if !result.completed {
    println!("Rollback at step {}", result.rollback_at.unwrap());
}
```

## Promotion Gates (FJ-3505)

Four gate types for environment promotion:

| Gate | What it checks |
|------|---------------|
| `validate` | Runs `forjar validate` (optional: deep mode) |
| `policy` | Evaluates policy-as-code rules |
| `coverage` | Parses `cargo llvm-cov --summary-only` for minimum coverage |
| `script` | Runs a custom shell script |

### Coverage Gate

The coverage gate runs `cargo llvm-cov --summary-only` and parses the TOTAL line for line coverage percentage. If the tool is not available, it falls back to advisory mode:

```yaml
environments:
  prod:
    promotion:
      from: staging
      gates:
        - validate:
            deep: true
        - coverage:
            min: 95
        - script: "make integration-test"
```

### Gate Evaluation

All gates must pass for promotion to proceed:

```rust
use forjar::core::promotion::evaluate_gates;

let result = evaluate_gates(&config_path, "prod", &promotion);
if result.all_passed {
    println!("Promotion approved: {} gates passed", result.passed_count());
} else {
    println!("{} gates failed", result.failed_count());
}
```

## Falsification

```bash
cargo test --test falsification_rollout_promotion
```

Key invariants verified:
- Canary plan includes canary step and 100% final step
- Percentage plan uses default steps (25/50/75/100) when none specified
- All-at-once produces a single 100% step
- Zero machines yields empty plan
- Health check timeout kills slow commands within configured duration
- Rollback triggered on first health check failure
- Deployed count deduplicates machines across steps
- Failed steps excluded from deployed count
