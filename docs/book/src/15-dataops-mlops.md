# DataOps & MLOps Pipelines

Forjar manages infrastructure for data and ML workloads using its convergence primitives.
The `task` resource type provides pipeline orchestration, while `model` and `gpu` resources
handle ML-specific infrastructure. Upstream consumers (alimentar, entrenar, apr-cli, batuta)
build domain-specific pipelines on these primitives.

## Architecture: Forjar as Convergence Primitive

Forjar provides the infrastructure layer. Domain tools compose on top:

| Layer | Tool | Responsibility |
|-------|------|----------------|
| Infrastructure | **forjar** | Converge packages, files, services, GPU drivers, models |
| DataOps | alimentar | Data quality gates, lineage, schema validation |
| MLOps | entrenar | Distributed training, checkpoints, hyperparameters |
| LLMOps | apr-cli | Model pull, convert, compile, serve |
| AgentOps | batuta | Agent lifecycle, dispatch, coordination |

Forjar does NOT embed data validation, model evaluation, or training orchestration.
Those are consumer responsibilities. Forjar ensures the infrastructure converges.

## Data Sources

Four data source types parameterize configurations at plan time:

```yaml
data:
  db-password:
    type: file
    value: secrets/db-password.txt
  git-sha:
    type: command
    value: git rev-parse HEAD
  api-ip:
    type: dns
    value: api.example.com
  prod-outputs:
    type: forjar-state
    config: production
    state_dir: ../production/state
    outputs: [web_ip, db_ip]
    max_staleness: "1h"
```

Values resolve before planning: `{{data.git-sha}}`, `{{data.db-password}}`.

## GPU Resource Management

The `gpu` resource type converges GPU driver and toolkit state:

```yaml
resources:
  gpu-setup:
    type: gpu
    machine: gpu-node
    gpu_backend: nvidia        # nvidia (default), rocm, cpu
    driver_version: "550"
    cuda_version: "12.4"
    persistence_mode: true
    compute_mode: default
    devices: [0, 1]            # GPU device indices
```

For AMD ROCm:

```yaml
resources:
  rocm-setup:
    type: gpu
    machine: amd-node
    gpu_backend: rocm
    rocm_version: "6.0"
    devices: [0]
```

## Model Resources

The `model` resource type manages ML model artifacts:

```yaml
resources:
  llama-model:
    type: model
    machine: inference-node
    name: llama-3.2-1b
    source: huggingface://meta-llama/Llama-3.2-1B
    format: gguf
    quantization: q4_k_m
    checksum: "blake3:abc123..."   # Pin exact version
    cache_dir: /models/cache
    depends_on: [gpu-setup]
```

Model drift detection uses BLAKE3 checksums. If a model file changes on disk
(corruption, unauthorized modification), `forjar drift` reports it.

## Task Resources for Pipelines

The `task` resource type runs arbitrary commands with convergence guarantees:

```yaml
resources:
  download-data:
    type: task
    machine: data-node
    command: "scripts/download-dataset.sh"
    output_artifacts:
      - /data/training/dataset.parquet
    completion_check: "test -f /data/training/dataset.parquet"
    timeout: 3600

  train-model:
    type: task
    machine: gpu-node
    command: "python train.py --epochs 10 --seed 42"
    working_dir: /opt/training
    output_artifacts:
      - /models/checkpoints/latest.pt
    completion_check: "test -f /models/checkpoints/latest.pt"
    depends_on: [download-data, gpu-setup]
    timeout: 7200

  deploy-model:
    type: task
    machine: inference-node
    command: "systemctl restart model-server"
    depends_on: [train-model, llama-model]
```

Key task fields:
- `completion_check`: skip apply if already done (idempotency)
- `output_artifacts`: glob paths hashed for drift detection
- `timeout`: seconds before command is killed
- `working_dir`: execution directory

## Full ML Pipeline Example

A complete GPU training pipeline using real forjar resources:

```yaml
version: "1.0"
name: ml-training-pipeline

machines:
  gpu:
    hostname: gpu-01
    addr: 10.0.0.50
    roles: [training]
  inference:
    hostname: inf-01
    addr: 10.0.0.51
    roles: [serving]

resources:
  # Infrastructure: GPU drivers
  gpu-driver:
    type: gpu
    machine: gpu
    gpu_backend: nvidia
    driver_version: "550"
    cuda_version: "12.4"

  # Infrastructure: Python environment
  python-env:
    type: package
    machine: gpu
    provider: apt
    packages: [python3-pip, python3-venv]

  # Infrastructure: Training dependencies
  pip-deps:
    type: task
    machine: gpu
    command: "pip install torch transformers datasets"
    completion_check: "python -c 'import torch'"
    depends_on: [python-env]

  # Data: Download training data
  training-data:
    type: task
    machine: gpu
    command: "python scripts/download_data.py"
    output_artifacts: [/data/training/train.parquet]
    completion_check: "test -f /data/training/train.parquet"
    working_dir: /opt/ml

  # Training: Run training job
  train:
    type: task
    machine: gpu
    command: "python train.py --seed 42 --epochs 10"
    output_artifacts: [/models/latest/model.safetensors]
    depends_on: [gpu-driver, pip-deps, training-data]
    working_dir: /opt/ml
    timeout: 14400  # 4 hours

  # Model: Deploy trained model
  model-artifact:
    type: model
    machine: inference
    name: fine-tuned-llama
    source: /models/latest/model.safetensors
    format: safetensors
    depends_on: [train]

  # Service: Model serving
  model-server:
    type: service
    machine: inference
    name: model-server
    state: running
    enabled: true
    restart_on: [model-artifact]
    depends_on: [model-artifact]

policy:
  failure: stop_on_first
  tripwire: true
  convergence_budget: 18000  # 5 hours total
```

## Training Checkpoint Management

Use the checkpoint command to manage training artifacts:

```bash
# List checkpoints for a resource
forjar checkpoint --file pipeline.yaml --resource train

# Garbage collect old checkpoints (keep last 3)
forjar checkpoint --file pipeline.yaml --gc --keep 3

# JSON output for CI integration
forjar checkpoint --file pipeline.yaml --json
```

## Drift Detection for ML Assets

BLAKE3 hashing catches unauthorized model or data changes:

```bash
# Check if models or data have drifted
forjar drift --file pipeline.yaml

# Auto-remediate: re-converge drifted resources
forjar apply --file pipeline.yaml --only-drifted
```

## Consumer Integration Patterns

### alimentar (DataOps)

alimentar calls `forjar apply` to converge data infrastructure, then adds data quality gates:

```bash
# alimentar workflow:
forjar apply -f data-infra.yaml          # converge infra
alimentar validate /data/training         # data quality
alimentar lineage /data/training --graph  # lineage tracking
```

### entrenar (MLOps)

entrenar coordinates distributed training across forjar-managed GPU nodes:

```bash
# entrenar workflow:
forjar apply -f gpu-cluster.yaml         # converge GPU infra
entrenar train --config training.yaml    # distributed training
entrenar eval --model latest --gate 0.92 # evaluation gate
```

### apr-cli (LLMOps)

apr-cli manages model lifecycle on forjar-converged infrastructure:

```bash
# apr-cli workflow:
forjar apply -f inference-infra.yaml     # converge infra
apr pull meta-llama/Llama-3.2-1B         # download model
apr compile model.gguf --target cuda     # compile for GPU
apr serve --port 8080                    # start inference
```

## GPU Clean-Room CI

For reproducible GPU testing in CI environments:

```yaml
# .github/workflows/gpu-test.yml
jobs:
  gpu-test:
    runs-on: [self-hosted, gpu]
    container:
      image: rust-cuda:1.89
    steps:
      - uses: actions/checkout@v4
      - run: cargo install --path . --root /usr/local
      - run: forjar apply -f dogfood-gpu-training.yaml
```

The clean-room CI environment ensures GPU tests run in isolation with deterministic
CUDA/ROCm versions. The `rust-cuda:1.89` image provides cargo at `/root/.cargo/bin/`.
