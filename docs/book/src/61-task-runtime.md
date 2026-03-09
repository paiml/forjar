# Task Framework Runtime

Falsification coverage for FJ-2700, FJ-2702, FJ-2703, and FJ-2704.

## Dispatch Mode (FJ-2700)

On-demand task execution with parameter injection.

### Parameter Substitution

```rust
use forjar::core::task::dispatch::{prepare_dispatch, validate_dispatch};
use forjar::core::types::DispatchConfig;

let config = DispatchConfig {
    name: "deploy".into(),
    command: "deploy --env {{ env }} --region {{ region }}".into(),
    params: vec![
        ("env".into(), "production".into()),
        ("region".into(), "us-east-1".into()),
    ],
    timeout_secs: Some(300),
};
assert!(validate_dispatch(&config).is_ok());
let prepared = prepare_dispatch(&config, &[]);
assert_eq!(prepared.command, "deploy --env production --region us-east-1");
```

Config params apply first; overrides only fill unclaimed placeholders.

### Invocation History

```rust
use forjar::core::task::dispatch::{record_invocation, success_rate};
use forjar::core::types::{DispatchInvocation, DispatchState};

let mut state = DispatchState::default();
record_invocation(&mut state, invocation, 10); // max 10 history entries
assert!((success_rate(&state) - 100.0).abs() < 0.01);
```

## Quality Gates (FJ-2702)

Pipeline stage gates with multiple evaluation modes.

### Gate Types

| Mode | Field | Condition |
|------|-------|-----------|
| Exit code | — | `exit_code == 0` |
| JSON field | `parse: json`, `field` | Value in `threshold` list |
| JSON min | `parse: json`, `field`, `min` | `value >= min` |
| Regex | `regex` | Pattern matches stdout |

### On-Fail Actions

| Action | Behavior |
|--------|----------|
| `block` (default) | Stop pipeline |
| `warn` | Emit warning, continue |
| `skip_dependents` | Skip downstream stages |

```rust
use forjar::core::task::{evaluate_gate, GateResult};
use forjar::core::types::QualityGate;

let gate = QualityGate {
    parse: Some("json".into()),
    field: Some("coverage".into()),
    min: Some(80.0),
    on_fail: Some("block".into()),
    ..Default::default()
};
assert_eq!(evaluate_gate(&gate, 0, r#"{"coverage":95}"#), GateResult::Pass);
```

## GPU Scheduling (FJ-2703)

### Device Targeting

```rust
use forjar::core::task::gpu_env_vars;

let vars = gpu_env_vars(Some(0));
// Sets CUDA_VISIBLE_DEVICES=0 and HIP_VISIBLE_DEVICES=0
```

### Round-Robin Scheduling

```rust
use forjar::core::types::GpuSchedule;

let schedule = GpuSchedule::round_robin(&["train", "eval", "infer"], 2);
assert_eq!(schedule.cuda_visible_devices("train").as_deref(), Some("0"));
assert_eq!(schedule.cuda_visible_devices("eval").as_deref(), Some("1"));
assert_eq!(schedule.cuda_visible_devices("infer").as_deref(), Some("0"));
```

## Barrier Synchronization (FJ-2704)

Multi-machine coordination before proceeding.

```rust
use forjar::core::types::BarrierTask;

let mut barrier = BarrierTask::new("deploy", vec!["web".into(), "db".into()]);
barrier.mark_complete("web");
assert!(!barrier.is_satisfied());
barrier.mark_complete("db");
assert!(barrier.is_satisfied());
```

## Service Lifecycle (FJ-2700)

State machine for long-running processes with health checks.

### State Transitions

```
Start → CheckHealth → [healthy] → Wait → CheckHealth → ...
                    → [failures >= retries] → Restart → CheckHealth → ...
                    → [restarts > max] → Stop
```

### Decision Functions

```rust
use forjar::core::task::service::{plan_service_action, ServiceAction};
use forjar::core::types::{HealthCheck, RestartPolicy, ServiceState};

let state = ServiceState::default();
let action = plan_service_action(&state, &RestartPolicy::default(), &health_check);
assert_eq!(action, ServiceAction::Start);
```

### Backoff

Exponential backoff with cap: `delay = base * 2^restart_count`, capped at `backoff_max_secs`.

Default: base=1s, max=60s → delays of 1, 2, 4, 8, 16, 32, 60, 60...

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_task_dispatch_gate.rs` | 31 | 446 |
| `falsification_task_gpu_barrier.rs` | 13 | 144 |
| `falsification_task_service_lifecycle.rs` | 19 | 445 |
