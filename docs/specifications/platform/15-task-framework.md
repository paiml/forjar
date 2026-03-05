# 15: Task Framework

> The infrastructure primitive for DataOps, MLOps, LLMOps, and AgentOps.

**Spec ID**: FJ-2700–FJ-2706 | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Motivation

Forjar's `type: task` resource currently runs a shell command with optional `completion_check` and `output_artifacts`. This works for simple one-shot commands, but five upstream consumers need richer primitives:

| Consumer | Domain | What They Need |
|----------|--------|---------------|
| **alimentar** | DataOps | Data quality gates, federated coordination, drift monitoring pipelines |
| **entrenar** | MLOps | GPU-allocated training jobs, distributed coordination, checkpoint management |
| **apr-cli** | LLMOps | Multi-stage model pipelines (pull→convert→compile→serve), model QA gates |
| **batuta** | AgentOps | Agent service lifecycle, model provisioning, capability-gated tool dispatch |
| **forjar itself** | InfraOps | Build tasks, CI pipelines, deployment workflows |

All five follow the same pattern: **orchestrate multi-step work across machines with resource tracking, quality gates, and failure recovery**. Forjar already has the DAG planner, transport layer, content-addressed store, and multi-machine execution. The task framework extends `type: task` to be the primitive all five build on.

### Design Principle: Primitive, Not Platform

Forjar is NOT Nomad, Airflow, or Kubeflow. It doesn't schedule, allocate, or queue. It **converges declared state on target machines**. The task framework extends this model:

- A **batch task** converges to "completed" state
- A **pipeline** converges to "all stages completed" state
- A **service task** converges to "running" state
- A **dispatch task** converges to "ready to accept triggers" state

No scheduler. No queue. No allocation. Just convergence.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│  Consumer Layer (builds ON forjar, not IN forjar)                 │
│                                                                    │
│  alimentar   entrenar    apr-cli     batuta       forjar          │
│  DataOps     MLOps       LLMOps     AgentOps     InfraOps        │
│  ─────────   ──────────  ─────────  ──────────   ───────────     │
│  quality     train plan  apr pull   agent run    cargo build     │
│  drift       train apply apr compile agent pool  cargo test      │
│  fed split   checkpoint  apr serve  tool dispatch deploy         │
└──────────────────┬───────────────────────────────────────────────┘
                   │  invokes via YAML resources
                   ▼
┌──────────────────────────────────────────────────────────────────┐
│  Forjar Task Framework (the primitive)                            │
│                                                                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │  batch   │ │ pipeline │ │ service  │ │ dispatch │            │
│  │          │ │          │ │          │ │          │            │
│  │ run once │ │ stages   │ │ long-run │ │ trigger  │            │
│  │ track    │ │ gates    │ │ health   │ │ params   │            │
│  │ artifacts│ │ cache    │ │ restart  │ │ on-demand│            │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘            │
│                                                                    │
│  ┌─────────────────────────────────────────────────────────┐     │
│  │  Common Primitives                                       │     │
│  │  • Input/output content-addressed tracking (BLAKE3)      │     │
│  │  • Quality gates (exit code, stdout parsing, threshold)  │     │
│  │  • GPU device targeting (CUDA_VISIBLE_DEVICES)           │     │
│  │  • Environment injection (secrets, params, data sources) │     │
│  │  • Retry with backoff                                    │     │
│  │  • Timeout enforcement                                   │     │
│  │  • Run log capture (spec 11)                             │     │
│  │  • Pre/post hooks                                        │     │
│  └─────────────────────────────────────────────────────────┘     │
│                                                                    │
│  ┌─────────────────────────────────────────────────────────┐     │
│  │  Existing Forjar Infrastructure                          │     │
│  │  DAG planner │ Transport layer │ Content store │ State   │     │
│  └─────────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────────┘
```

---

## FJ-2700: Task Modes

### Current `type: task` (Unchanged)

```yaml
resources:
  simple-task:
    type: task
    command: "echo hello"
    timeout: 60
    completion_check: "test -f /tmp/done"
    output_artifacts: [/tmp/done]
    working_dir: /opt/app
```

Existing fields preserved. `mode` defaults to `batch` for backward compatibility.

### Mode: batch (Default)

Run-once tasks that converge to "completed" state.

```yaml
resources:
  db-migrate:
    type: task
    mode: batch
    command: "alembic upgrade head"
    completion_check: "alembic current | grep -q head"
    output_artifacts: [/var/lib/app/alembic.stamp]
    depends_on: [install-app]
    retry:
      max_attempts: 3
      backoff: exponential
```

**State tracking**:
- `completion_check` exit 0 → `task=completed` → planner returns NO-OP
- `output_artifacts` hashed via BLAKE3 → drift detection if artifacts change
- No `completion_check` and no `output_artifacts` → always re-runs (current behavior)

### Mode: pipeline

Ordered multi-stage execution with inter-stage gates.

```yaml
resources:
  model-build:
    type: task
    mode: pipeline
    stages:
      - name: pull
        command: "apr pull '{{inputs.model_source}}'"
        outputs: [/opt/models/raw.gguf]

      - name: convert
        command: "apr convert /opt/models/raw.gguf --format apr --quantization q4_k_m"
        inputs: [/opt/models/raw.gguf]
        outputs: [/opt/models/model.apr]

      - name: compile
        command: "apr compile /opt/models/model.apr --target x86_64-linux-gnu --release"
        inputs: [/opt/models/model.apr]
        outputs: [/opt/apr/bin/model-server]

      - name: verify
        command: "apr qa /opt/apr/bin/model-server --gates G0,G1,G2"
        gate: true    # Pipeline stops if this stage fails
    cache: true       # Content-addressed: skip stages whose inputs haven't changed
```

**Stage execution**:
```
for each stage S in pipeline:
    if cache and all S.inputs unchanged (BLAKE3 match):
        skip S (cached)
        continue
    execute S.command
    if S.gate and exit_code != 0:
        mark pipeline FAILED at stage S
        stop
    hash S.outputs → store in state
```

**Content-addressed caching**: Each stage's inputs are hashed. If input hashes match the previous run, the stage is skipped. This gives Make-like incremental builds using forjar's BLAKE3 store.

### Mode: service

Long-running tasks with health checks and restart policy. Distinct from `type: service` (which manages systemd units) — `mode: service` is for forjar-managed processes.

```yaml
resources:
  inference-server:
    type: task
    mode: service
    command: "apr serve --model /opt/models/llama.gguf --port 8080"
    health_check:
      command: "curl -sf http://localhost:8080/health"
      interval: 30s
      timeout: 5s
      retries: 3
    restart: on_failure
    restart_delay: 10s
    depends_on: [model-build]
    gpu_device: 0
```

**Lifecycle**:
- `forjar apply` starts the process if not running
- Health check runs at `interval` — failure triggers restart after `restart_delay`
- `forjar destroy` stops the process
- State tracked via PID file + health check status

**When to use `mode: service` vs `type: service`**:
- `type: service` → manages systemd/launchd units (production, survives reboot)
- `mode: service` → forjar-managed process (development, testing, ephemeral)

### Mode: dispatch

Parameterized tasks triggered on-demand via `forjar run`.

```yaml
resources:
  deploy:
    type: task
    mode: dispatch
    command: "deploy.sh {{dispatch.version}} {{dispatch.env}}"
    params:
      version: { type: string, required: true }
      env: { type: enum, choices: [staging, production], default: staging }
    gate:
      command: "test -f /opt/app/deploy-ready.flag"
      message: "Deployment not ready — missing flag"
```

**Invocation**:
```bash
forjar run deploy --param version=1.2.3 --param env=production
```

**Gate**: Pre-flight check before dispatch. If gate fails, dispatch is rejected with message. Prevents accidental production deploys.

---

## FJ-2701: Input/Output Tracking

### Content-Addressed Inputs

```yaml
resources:
  build-binary:
    type: task
    mode: batch
    command: "cargo build --release --locked"
    inputs:
      - src/**/*.rs        # Glob patterns
      - Cargo.toml
      - Cargo.lock
    outputs:
      - target/release/forjar
    cache: true
```

**Algorithm**:
```
fn should_skip_task(task, state_lock):
    if not task.cache:
        return false
    for input_glob in task.inputs:
        for file in glob(input_glob):
            current_hash = blake3(file)
            if current_hash != state_lock.input_hashes[file]:
                return false   // Input changed → must re-run
    return true                // All inputs unchanged → skip
```

**Storage**: Input/output hashes stored in `state.lock.yaml` per task:
```yaml
resources:
  build-binary:
    status: Converged
    hash: "abc123..."
    details:
      input_hashes:
        src/main.rs: "def456..."
        Cargo.toml: "789abc..."
      output_hashes:
        target/release/forjar: "012def..."
      cached: true
      last_duration_ms: 45000
```

---

## FJ-2702: Quality Gates

Quality gates conditionally block downstream execution based on task output.

```yaml
resources:
  data-quality:
    type: task
    mode: batch
    command: "alimentar quality score data.parquet --json"
    gate:
      parse: json
      field: grade
      threshold: ["A", "B"]     # Block if grade is C, D, or F
      on_fail: block             # block | warn | skip_dependents

  train-model:
    type: task
    mode: batch
    command: "entrenar train config.yaml"
    depends_on: [data-quality]   # Only runs if data-quality gate passes
```

### Gate Types

| Type | Syntax | Passes When |
|------|--------|-------------|
| **Exit code** | `gate: { exit_code: 0 }` | Command exits 0 |
| **JSON field** | `gate: { parse: json, field: grade, threshold: ["A", "B"] }` | Parsed field in threshold list |
| **Regex** | `gate: { parse: stdout, regex: "score: [89]\\d" }` | Stdout matches pattern |
| **Numeric** | `gate: { parse: json, field: score, min: 80.0 }` | Numeric field >= min |
| **Script** | `gate: { command: "test -f /opt/ready" }` | Gate command exits 0 |

### Gate Actions

| Action | Effect |
|--------|--------|
| `block` | Task fails, dependents skipped (default) |
| `warn` | Task succeeds with warning, dependents run |
| `skip_dependents` | Task succeeds, but all `depends_on` this task are skipped |

---

## FJ-2703: GPU Device Targeting

GPU-consuming tasks need device assignment without a scheduler.

```yaml
resources:
  train-adapter-0:
    type: task
    mode: batch
    command: "entrenar train config.yaml --adapter 0"
    gpu_device: 0
    gpu_memory: 16384      # MB — informational, not enforced by forjar
    depends_on: [download-model]

  train-adapter-1:
    type: task
    mode: batch
    command: "entrenar train config.yaml --adapter 1"
    gpu_device: 1
    gpu_memory: 16384
    depends_on: [download-model]
```

**Implementation**: `gpu_device` injects `CUDA_VISIBLE_DEVICES={gpu_device}` into the task environment. Forjar does NOT allocate or manage GPU memory — that's the consumer's responsibility (entrenar's VRAM ledger, apr's memory budgeting).

**Why not a GPU scheduler?** Forjar is a convergence engine, not a resource allocator. GPU scheduling requires real-time monitoring, preemption, and queueing — these belong in entrenar/realizar, not in the infrastructure primitive. Forjar's job is to declare "this task runs on GPU 0" and enforce that declaration.

---

## FJ-2704: Distributed Task Coordination

Multi-machine tasks that require coordination across nodes.

### Fan-Out Pattern (alimentar federated, entrenar distributed)

```yaml
machines:
  coordinator: { addr: 10.0.1.1 }
  worker-1: { addr: 10.0.1.2, gpu_backend: nvidia }
  worker-2: { addr: 10.0.1.3, gpu_backend: nvidia }
  worker-3: { addr: 10.0.1.4, gpu_backend: nvidia }

resources:
  # Stage 1: Fan-out — run on all workers
  generate-manifests:
    type: task
    mode: batch
    machine: [worker-1, worker-2, worker-3]
    command: "alimentar fed manifest local.parquet -o /tmp/manifest.json --node-id {{machine.hostname}}"
    output_artifacts: [/tmp/manifest.json]

  # Stage 2: Gather — collect artifacts to coordinator
  collect-manifests:
    type: task
    mode: batch
    machine: coordinator
    command: "alimentar fed plan /tmp/manifests/*.json -o /tmp/plan.json"
    depends_on: [generate-manifests]
    gather:
      from: [worker-1, worker-2, worker-3]
      artifact: /tmp/manifest.json
      to: /tmp/manifests/

  # Stage 3: Fan-out again — distribute plan to all workers
  execute-splits:
    type: task
    mode: batch
    machine: [worker-1, worker-2, worker-3]
    command: "alimentar fed split local.parquet /tmp/plan.json --node-id {{machine.hostname}}"
    depends_on: [collect-manifests]
    scatter:
      from: coordinator
      artifact: /tmp/plan.json
      to: /tmp/plan.json
```

### Coordination Primitives

| Primitive | Description | Implementation |
|-----------|-------------|----------------|
| **Fan-out** | Run same task on multiple machines | `machine: [list]` — existing multi-machine support |
| **Gather** | Collect artifacts from multiple machines to one | `scp` via SSH transport to coordinator |
| **Scatter** | Distribute artifact from one machine to many | `scp` via SSH transport from coordinator |
| **Barrier** | Wait for all machines before proceeding | `depends_on` with multi-machine task |

**Not included** (consumer responsibility):
- TCP coordination protocol (entrenar's AllReduce)
- Gradient exchange (entrenar's ring topology)
- Agent message passing (batuta's MessageRouter)
- Data sharding decisions (alimentar's split strategy)

Forjar moves files and runs commands. The consumers handle their own coordination protocols.

---

## FJ-2705: Consumer Integration Patterns

### alimentar (DataOps)

```yaml
# Data quality pipeline
resources:
  ingest-data:
    type: task
    mode: batch
    command: "alimentar convert raw.csv data.parquet"
    output_artifacts: [data.parquet]

  quality-gate:
    type: task
    mode: batch
    command: "alimentar quality score data.parquet --json --profile ml-training"
    depends_on: [ingest-data]
    gate:
      parse: json
      field: grade
      threshold: ["A", "B"]

  drift-check:
    type: task
    mode: batch
    command: "alimentar drift detect reference.parquet data.parquet --json"
    depends_on: [ingest-data]
    gate:
      parse: json
      field: max_severity
      threshold: ["None", "Low"]
      on_fail: warn
```

### entrenar (MLOps)

```yaml
# Distributed training job
resources:
  prepare-data:
    type: task
    mode: batch
    command: "alimentar convert corpus.jsonl train.parquet"
    output_artifacts: [train.parquet]

  train-model:
    type: task
    mode: batch
    machine: gpu-node-1
    command: "entrenar train config.yaml --checkpoint-dir /opt/checkpoints"
    gpu_device: 0
    gpu_memory: 24576
    timeout: 86400          # 24 hours
    depends_on: [prepare-data]
    output_artifacts: [/opt/checkpoints/best/model.safetensors]
    health_check:
      command: "test -f /opt/checkpoints/training_state.json && find /opt/checkpoints/training_state.json -mmin -10"
      interval: 300s        # Check every 5 min that training is progressing

  evaluate-model:
    type: task
    mode: batch
    command: "entrenar eval /opt/checkpoints/best/model.safetensors --json"
    depends_on: [train-model]
    gate:
      parse: json
      field: accuracy
      min: 0.85
```

### apr-cli (LLMOps)

```yaml
# Full model build pipeline
resources:
  model-pipeline:
    type: task
    mode: pipeline
    stages:
      - name: pull
        command: "apr pull '{{inputs.model_source}}'"
        outputs: [/opt/models/raw.gguf]

      - name: qa-preflight
        command: "apr qa /opt/models/raw.gguf --gates G0,G1,G2 --json"
        gate: true

      - name: convert
        command: "apr convert /opt/models/raw.gguf --format apr --quantization {{inputs.quantization}}"
        inputs: [/opt/models/raw.gguf]
        outputs: [/opt/models/model.apr]

      - name: compile
        command: "apr compile /opt/models/model.apr --target x86_64-linux-gnu --release --strip --lto"
        inputs: [/opt/models/model.apr]
        outputs: [/opt/apr/bin/model-server]

      - name: qa-postflight
        command: "apr qa /opt/apr/bin/model-server --gates G3,G4,G5 --json"
        gate: true
    cache: true
    gpu_device: 0

  inference-server:
    type: task
    mode: service
    command: "apr serve --model /opt/models/model.apr --port 8080 --workers 4"
    depends_on: [model-pipeline]
    gpu_device: 0
    health_check:
      command: "curl -sf http://localhost:8080/health"
      interval: 30s
    restart: on_failure
```

### batuta (AgentOps)

```yaml
# Agent lifecycle
resources:
  agent-model:
    type: model
    source: "meta-llama/Llama-3-8B-GGUF"
    path: /opt/models/llama-3-8b.gguf
    format: gguf
    checksum: abc123...

  agent-service:
    type: task
    mode: service
    command: "batuta agent run --manifest /etc/batuta/agent.toml --daemon"
    depends_on: [agent-model, nvidia-driver]
    gpu_device: 0
    health_check:
      command: "batuta agent status --json | jq -e '.status == \"running\"'"
      interval: 60s
    restart: always
    restart_delay: 30s
    env:
      BATUTA_MODEL_PATH: /opt/models/llama-3-8b.gguf
      BATUTA_PRIVACY_TIER: sovereign

  agent-pool:
    type: task
    mode: dispatch
    command: "batuta agent run --manifest /etc/batuta/pool.toml --query '{{dispatch.query}}'"
    depends_on: [agent-service]
    params:
      query: { type: string, required: true }
```

### forjar (InfraOps / Self-Build)

```yaml
# CI build pipeline
resources:
  build:
    type: task
    mode: pipeline
    stages:
      - name: check
        command: "cargo fmt --check && cargo clippy -- -D warnings"
      - name: test
        command: "cargo test"
        gate: true
      - name: coverage
        command: "cargo llvm-cov --summary-only --fail-under-lines 95"
        gate: true
      - name: build
        command: "cargo build --release --locked"
        inputs: [src/**/*.rs, Cargo.toml, Cargo.lock]
        outputs: [target/release/forjar]
    cache: true
```

---

## FJ-2706: Task State Model

### State Machine

```
              ┌──────────┐
              │  Pending  │  (initial state, or completion_check returns "pending")
              └─────┬─────┘
                    │ apply
              ┌─────▼─────┐
              │  Running   │  (command executing)
              └──┬─────┬──┘
                 │     │
          success│     │failure
                 │     │
          ┌──────▼┐  ┌─▼───────┐
          │ Done  │  │ Failed  │
          └──┬────┘  └────┬────┘
             │            │ retry (if configured)
             │       ┌────▼─────┐
             │       │ Retrying │ → Running
             │       └──────────┘
             │
       ┌─────▼──────┐
       │  Converged  │  (completion_check returns "completed", or outputs match)
       └─────────────┘
```

### Pipeline State

```yaml
# In state.lock.yaml
resources:
  model-pipeline:
    status: Converged
    hash: "pipeline:abc123"
    details:
      mode: pipeline
      stages:
        pull:
          status: cached
          duration_ms: 0
          output_hashes: { "/opt/models/raw.gguf": "aaa111" }
        convert:
          status: completed
          duration_ms: 12340
          output_hashes: { "/opt/models/model.apr": "bbb222" }
        compile:
          status: completed
          duration_ms: 45670
          output_hashes: { "/opt/apr/bin/model-server": "ccc333" }
        verify:
          status: completed
          duration_ms: 5200
          gate_passed: true
```

### Service State

```yaml
resources:
  inference-server:
    status: Converged
    hash: "service:def456"
    details:
      mode: service
      pid: 12345
      started_at: "2026-03-05T14:30:00Z"
      health_checks_passed: 147
      health_checks_failed: 0
      restarts: 0
      last_health_check: "2026-03-05T15:45:30Z"
```

---

## Performance Targets

| Operation | Target | Mechanism |
|-----------|--------|-----------|
| Task overhead (batch) | <10ms | Completion check + hash compare |
| Pipeline skip (all cached) | <50ms | Input hash comparison per stage |
| Gather (3 machines, 1MB each) | <2s | Parallel SCP |
| Scatter (1 machine to 10) | <3s | Parallel SCP |
| Gate evaluation (JSON parse) | <5ms | serde_json in-process |
| Service health check | <1s per check | Transport exec |

---

## What Forjar Does NOT Do

| Concern | Owner | Why Not Forjar |
|---------|-------|---------------|
| GPU memory allocation | entrenar (VRAM ledger) | Requires real-time monitoring |
| Training gradient exchange | entrenar (AllReduce) | Application-level protocol |
| Agent reasoning loop | batuta (perceive-reason-act) | Application logic |
| Data transformation | alimentar (Arrow transforms) | Data domain logic |
| Model inference | realizar (CUDA/wgpu) | Compute kernel |
| Model format conversion | apr-cli (convert/compile) | Model domain logic |
| Hyperparameter search | entrenar (TPE/ASHA) | Algorithmic concern |
| Service discovery | consumer responsibility | Requires registrar |
| Job queueing | consumer responsibility | Requires scheduler |
| Resource preemption | consumer responsibility | Requires allocator |

Forjar is the **convergence primitive**. Consumers own their domain logic.

---

## Implementation

### Phase 36: Task Modes (FJ-2700) -- PARTIAL
- [x] `TaskMode` enum (batch/pipeline/service/dispatch) with serde roundtrip
- [x] `PipelineStage` struct (name, command, inputs, outputs, gate)
- [x] `QualityGate` + `HealthCheck` structs defined
- [x] `task_mode` field on Resource, validated in parser
- [x] Pipeline mode: `stages:` array, no `command:` required
- [x] Known field detection updated for all new fields
- [ ] Runtime execution of pipeline stages with caching
- [ ] Service mode health check loop and restart policy
- [ ] `mode: dispatch` with `forjar run` CLI
- **Deliverable**: Types and validation complete; runtime execution pending

### Phase 37: Input/Output Tracking (FJ-2701)
- [x] `inputs:` glob pattern hashing (BLAKE3)
- [x] `outputs:` artifact hashing and storage
- [x] `cache: true` for stage-level skip logic
- [ ] Input/output hashes in `state.lock.yaml`
- **Deliverable**: Pipeline stages skip when inputs unchanged

### Phase 38: Quality Gates (FJ-2702)
- [x] Exit code gates
- [x] JSON field parsing gates
- [x] Regex stdout gates
- [x] Numeric threshold gates
- [x] `on_fail:` actions (block, warn, skip_dependents)
- **Deliverable**: `alimentar quality score` output gates downstream training

### Phase 39: GPU Device Targeting (FJ-2703)
- [x] `gpu_device:` field → `CUDA_VISIBLE_DEVICES` injection
- [ ] `gpu_memory:` informational field in state
- [ ] Multi-GPU parallel tasks in same wave
- **Deliverable**: Two training tasks run on GPU 0 and GPU 1 simultaneously

### Phase 40: Distributed Coordination (FJ-2704)
- [x] `gather:` — collect artifacts from multiple machines
- [x] `scatter:` — distribute artifacts to multiple machines
- [x] `machine: [list]` fan-out execution (MachineTarget::Multiple)
- [ ] Barrier via `depends_on` multi-machine task
- **Deliverable**: Federated learning manifest collection across 3 nodes

### Phase 41: Consumer Integration Testing (FJ-2705)
- [ ] alimentar quality pipeline recipe
- [ ] entrenar training job recipe
- [ ] apr-cli model build pipeline recipe
- [ ] batuta agent lifecycle recipe
- [ ] forjar self-build pipeline recipe
- **Deliverable**: Five reference recipes proving the primitive works for all domains

### Phase 42: Task State Model (FJ-2706)
- [x] Pipeline state with per-stage tracking
- [x] Service state with PID, health check history
- [x] Dispatch state with invocation history
- [ ] State model documented in state compatibility section
- **Deliverable**: `forjar status` shows pipeline stage progress and service health
