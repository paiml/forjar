# Conditional Evaluation, Progressive Rollout & Shell Purification

Falsification coverage for FJ-202, FJ-3507, and FJ-036.

## Conditional Evaluation (FJ-202)

`when:` expressions control per-machine resource inclusion at plan time:

```rust
use forjar::core::conditions::evaluate_when;

let machine = Machine { arch: "x86_64".into(), roles: vec!["gpu".into()], .. };
let params = HashMap::from([("env".into(), Value::String("prod".into()))]);

// Literal booleans (case-insensitive)
evaluate_when("true", &params, &machine);    // Ok(true)

// Template substitution + operators
evaluate_when("{{machine.arch}} == \"x86_64\"", &params, &machine);  // Ok(true)
evaluate_when("{{params.env}} != \"staging\"", &params, &machine);   // Ok(true)
evaluate_when("{{machine.roles}} contains \"gpu\"", &params, &machine); // Ok(true)
```

Machine fields: `arch`, `hostname`, `addr`, `user`, `roles`. Operators: `==`, `!=`, `contains`.

## Progressive Rollout (FJ-3507)

Three strategies with health checks and auto-rollback:

```rust
use forjar::core::rollout::{plan_rollout, execute_rollout, run_health_check};

// Canary: deploy to N machines first, then percentage steps
let config = RolloutConfig { strategy: "canary".into(), canary_count: 1, .. };
let steps = plan_rollout(&config, 10); // canary(1) → 25% → 50% → 100%

// Percentage: deploy in percentage waves (default: 25/50/75/100)
let config = RolloutConfig { strategy: "percentage".into(), percentage_steps: vec![25, 50, 100], .. };

// All-at-once: single step to all machines
let config = RolloutConfig { strategy: "all-at-once".into(), .. };

// Execute with health checks
let result = execute_rollout(&config, 10, false); // dry_run=false
assert!(result.completed);
assert_eq!(result.deployed_count(), 10); // deduped across steps

// Health check with timeout
let (passed, msg) = run_health_check("curl -f http://localhost/health", Some("30s"));
```

## Shell Purification (FJ-036)

Three levels of shell safety via bashrs integration:

```rust
use forjar::core::purifier::*;

// Level 1: Lint validation (errors only, warnings pass)
validate_script("echo hello\n")?; // Ok(())

// Level 2: Error counting
assert_eq!(lint_error_count("echo hello\n"), 0);

// Level 3: Full purification (parse → purify AST → format → validate)
let purified = purify_script("x=1\necho $x\n")?;

// Smart path: validate first, purify only if needed
let result = validate_or_purify("echo hello\n")?; // fast path
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_conditions_rollout_purifier.rs` | 35 | ~350 |
