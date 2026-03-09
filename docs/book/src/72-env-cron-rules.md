# Environments, Cron Source & Rules Runtime

Falsification coverage for FJ-3500, FJ-3103, and FJ-3106.

## Environment Resolution (FJ-3500)

Environments override base config params and machine addresses:

```rust
use forjar::core::types::environment::*;
use forjar::core::types::Machine;

let mut base_params = HashMap::new();
base_params.insert("region".into(), Value::String("us-east-1".into()));

let mut staging = Environment::default();
staging.params.insert("tier".into(), Value::String("staging".into()));
staging.machines.insert("web1".into(), MachineOverride { addr: "10.1.0.1".into() });

let resolved = resolve_env_params(&base_params, &staging);
// region=us-east-1, tier=staging (override wins)

let machines = resolve_env_machines(&base_machines, &staging);
// web1.addr = 10.1.0.1 (overridden), db1.addr = 10.0.0.2 (base)
```

### Promotion Gates

Gates classify as `validate`, `policy`, `coverage`, or `script`:

```rust
let gate = PromotionGate {
    coverage: Some(CoverageGateOptions { min: 90 }),
    ..Default::default()
};
assert_eq!(gate.gate_type(), "coverage");
```

### Environment Diffing

```rust
let diff = diff_environments("staging", &staging, "prod", &prod, &base_params, &base_machines);
println!("{} total diffs, identical={}", diff.total_diffs(), diff.is_identical());
```

## Cron Source (FJ-3103)

Parses standard 5-field cron expressions:

```rust
use forjar::core::cron_source::{parse_cron, matches, schedule_summary, CronTime};

let sched = parse_cron("30 9 * * 1-5").unwrap(); // 9:30 weekdays
let time = CronTime { minute: 30, hour: 9, day: 3, month: 3, weekday: 1 };
assert!(matches(&sched, &time));
```

Supports: `*`, exact values, ranges (`9-17`), steps (`*/15`), lists (`8,12,18`).

## Rules Runtime (FJ-3106)

Evaluates infrastructure events against rulebook configs:

```rust
use forjar::core::rules_runtime::*;
use forjar::core::types::*;

let config = RulebookConfig { rulebooks: vec![rulebook] };
let mut tracker = CooldownTracker::default();

// Find which rulebooks match an event
let matched = matching_rulebooks(&event, &config);

// Get actions to fire (respects cooldowns)
let actions = fired_actions(&event, &config, &mut tracker);

// Full evaluation with disabled/cooldown status
let results = evaluate_event(&event, &config, &mut tracker);
```

### Action Types

| Type | Purpose |
|------|---------|
| `apply` | Run `forjar apply` on resource subset |
| `destroy` | Remove specific resources |
| `script` | Execute shell command |
| `notify` | Send notification to channel |

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_env_promotion.rs` | 27 | ~260 |
| `falsification_cron_rules.rs` | 36 | ~420 |
