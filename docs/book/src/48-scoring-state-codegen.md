# Scoring, State Management & Codegen

Forjar provides a comprehensive quality scoring system, crash-safe state
management, and script generation pipeline for infrastructure convergence.

## ForjarScore v2 (FJ-2800)

ForjarScore v2 uses a two-tier grading system:

**Static Grade** (design quality, always available):
- SAF — Safety (25%): file modes, curl|bash detection, plaintext secrets
- OBS — Observability (20%): tripwire, lock file, notify hooks, output descriptions
- DOC — Documentation (15%): header metadata, description, comments, param docs
- RES — Resilience (20%): failure policy, retries, hooks, deny paths
- CMP — Composability (20%): params, templates, includes, tags, recipes

**Runtime Grade** (operational quality, after apply):
- COR — Correctness (35%): validate, plan, apply, convergence, lock
- IDM — Idempotency (35%): second apply, zero changes, hash stability
- PRF — Performance (30%): budget adherence, idempotent speed, efficiency

**Grade thresholds:** A (>=90, min>=80), B (>=75, min>=60), C (>=60, min>=40), D (>=40), F

```rust
use forjar::core::scoring::{compute, format_score_report, ScoringInput};

let result = compute(&config, &ScoringInput {
    status: "qualified".into(),
    idempotency: "strong".into(),
    budget_ms: 5000,
    runtime: Some(runtime_data),
    raw_yaml: Some(yaml.into()),
});
println!("{}", format_score_report(&result));
// Grade: A/A (static/runtime)
```

## State Management (FJ-013)

### Lock Files

Per-machine state is persisted in `state/<machine>/state.lock.yaml`:

```rust
use forjar::core::state;

let lock = state::new_lock("web", "web-01");
state::save_lock(&state_dir, &lock)?; // Atomic write (temp + rename)

let loaded = state::load_lock(&state_dir, "web")?;
assert!(loaded.is_some());
```

### Global Lock

The global lock (`state/forjar.lock.yaml`) tracks all machines:

```rust
state::update_global_lock(&state_dir, "my-infra", &[
    ("web".into(), 5, 5, 0),  // (name, total, converged, failed)
    ("db".into(),  3, 2, 1),
])?;
```

### BLAKE3 Integrity (FJ-1270)

Every lock file gets a `.b3` sidecar containing its BLAKE3 hash:

```rust
use forjar::core::state::integrity;

let results = integrity::verify_state_integrity(&state_dir);
if integrity::has_errors(&results) {
    integrity::print_issues(&results, true);
    // ERROR: integrity check failed for state/web/state.lock.yaml
}
```

### Process Locking (FJ-266)

Prevents concurrent applies on the same state directory:

```rust
state::acquire_process_lock(&state_dir)?;
// ... apply ...
state::release_process_lock(&state_dir);
```

Stale locks (PID no longer running) are automatically removed.

## Codegen Dispatch (FJ-005)

The codegen module generates shell scripts for each resource type:

```rust
use forjar::core::codegen::{check_script, apply_script, state_query_script};

let check = check_script(&resource)?;   // Read current state
let apply = apply_script(&resource)?;   // Converge to desired
let query = state_query_script(&resource)?; // Query for BLAKE3
```

Supported types: Package, File, Service, Mount, User, Docker, Cron,
Network, Pepita, Model, Gpu, Task, WasmBundle, Image, Build.

Recipe types return an error — they must be expanded first via
`expand_recipe()`.

**Sudo wrapping (FJ-1394):** When `resource.sudo = true`, the apply
script wraps in `sudo bash <<'FORJAR_SUDO'` heredoc.

## Promotion Gates (FJ-3505)

Quality gates that must pass before environment promotion:

```rust
use forjar::core::promotion::evaluate_gates;

let result = evaluate_gates(&config_path, "prod", &promotion_config);
if !result.all_passed {
    println!("{} gates failed", result.failed_count());
}
```

Gate types: `validate` (deep/standard), `policy`, `coverage` (min %),
`script` (custom command).

## Promotion Events (FJ-3509)

Structured JSONL logging for promotion lifecycle:

```rust
use forjar::core::promotion_events::{log_promotion, log_rollback, PromotionParams};

log_promotion(&PromotionParams {
    state_dir: &state_dir,
    target_env: "prod",
    source: "staging", target: "prod",
    gates_passed: 3, gates_total: 3,
    rollout_strategy: Some("canary"),
})?;
```

## CIS Ubuntu 22.04 Pack (FJ-3206)

Built-in compliance pack with 24 rules covering CIS benchmark sections 1-6:

```rust
use forjar::core::cis_ubuntu_pack::{cis_ubuntu_2204_pack, severity_summary};

let pack = cis_ubuntu_2204_pack();
let (errors, warnings, info) = severity_summary(&pack);
// 14 errors, 9 warnings, 1 info
```

## Falsification Example

```bash
cargo run --example scoring_state_falsification
```
