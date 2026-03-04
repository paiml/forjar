# DataOps & MLOps Pipelines

Forjar manages the full lifecycle of data and ML infrastructure: data sources, validation, GPU provisioning, model training, evaluation, and deployment.

## Data Sources

Forjar supports four data source types for parameterizing configurations:

```yaml
data:
  - name: db-creds
    type: file
    path: secrets/db.yaml

  - name: git-sha
    type: command
    command: git rev-parse HEAD

  - name: dns-check
    type: dns
    hostname: api.example.com

  - name: prod-state
    type: forjar-state
    path: ../production/state
    max_staleness: "1h"
```

Data sources are resolved before planning, making their values available as `{{data.db-creds.password}}` in resource templates.

## Data Validation Resources

Declarative data quality checks ensure data integrity before downstream consumption:

```yaml
resources:
  - name: validate-training-data
    type: data_validation
    schema: schemas/training.json
    source: /data/training/latest.parquet
    checks:
      - no_nulls: [label, features]
      - freshness: 24h
      - min_rows: 10000
```

## Dataset Versioning & Lineage

Content-addressed dataset snapshots track data provenance:

```yaml
resources:
  - name: training-dataset-v3
    type: dataset
    path: /data/training/v3
    lineage:
      parent: training-dataset-v2
      transform: scripts/augment.py
      hash_algorithm: blake3
```

Lineage graphs show the full transformation chain from raw data to model input, enabling reproducibility audits.

## GPU Resource Management

Forjar natively manages GPU infrastructure:

```yaml
resources:
  - name: gpu-driver
    type: gpu_driver
    gpu_backend: nvidia    # or: rocm, cpu
    version: "550.54.14"
    ensure: present

  - name: cuda-toolkit
    type: package
    name: cuda-toolkit-12-4
    depends_on: [gpu-driver]
```

GPU backends are auto-detected. The `gpu_backend` field supports `nvidia` (default), `rocm` (AMD), and `cpu` (fallback).

## ML Model Resources

Model lifecycle management from training to serving:

```yaml
resources:
  - name: sentiment-model
    type: model
    format: safetensors           # or: gguf, pytorch, onnx
    source: huggingface://org/model-name
    version: "2.1.0"
    registry: /models/registry
    ensure: present

  - name: model-eval-gate
    type: model_eval
    model: sentiment-model
    metrics:
      accuracy: ">= 0.92"
      latency_p99: "<= 50ms"
    dataset: /data/eval/holdout.parquet
```

## Training Reproducibility

Forjar ensures training runs are reproducible by managing the full environment:

```yaml
# dogfood-gpu-training.yaml
resources:
  # Phase 1: GPU infrastructure
  - name: gpu-driver
    type: gpu_driver
    gpu_backend: nvidia

  # Phase 2: Training environment
  - name: conda-env
    type: package
    provider: conda
    name: training-env
    source: environment.yml

  # Phase 3: Data preparation
  - name: training-data
    type: dataset
    path: /data/training
    hash_algorithm: blake3

  # Phase 4: Training
  - name: train-model
    type: command
    command: python train.py --seed 42
    depends_on: [gpu-driver, conda-env, training-data]

  # Phase 5: Evaluation gate
  - name: eval-gate
    type: model_eval
    model: train-model
    metrics:
      loss: "< 0.05"
```

Data parity contracts verify that training data on all machines has identical BLAKE3 hashes before training begins.

## Pipeline DAG Orchestration

Multi-stage pipelines are expressed as dependency chains:

```yaml
resources:
  - name: extract
    type: command
    command: scripts/extract.sh

  - name: transform
    type: command
    command: scripts/transform.py
    depends_on: [extract]

  - name: validate
    type: data_validation
    source: /data/transformed
    depends_on: [transform]

  - name: load
    type: command
    command: scripts/load.sh
    depends_on: [validate]
```

Forjar's topological sorter ensures stages execute in the correct order, and the saga pattern handles failures gracefully across stages.

## Model Registry & Checkpoints

Content-addressed model storage with checkpoint management:

```bash
# Pin a model version
forjar pin add sentiment-model@2.1.0 --hash abc123

# List model versions
forjar store list --type model

# Checkpoint during training
forjar store gc --keep-generations 5  # Keep last 5 checkpoints
```

Models are stored in forjar's content-addressed store, enabling deduplication across versions and instant rollback to any checkpoint.

## GPU Clean-Room CI

For reproducible GPU testing in CI:

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

The clean-room CI environment ensures GPU tests run in isolation with deterministic CUDA/ROCm versions.
