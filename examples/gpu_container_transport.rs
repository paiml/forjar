//! Demonstrate multi-vendor GPU container transport.
//!
//! Parses a config with NVIDIA CUDA and AMD ROCm machines, validates it,
//! resolves the DAG, and produces a plan. Shows GPU-specific container
//! fields: --gpus, --device, --group-add, --env.
//!
//! No Docker or GPU hardware required (doesn't actually execute).
//!
//! Usage: cargo run --example gpu_container_transport

use forjar::core::types::{ContainerConfig, ForjarConfig, Machine, PlanAction, PlannedChange};
use forjar::core::{parser, planner, resolver};
use std::collections::HashMap;

fn gpu_config_yaml() -> &'static str {
    r#"
version: "1.0"
name: gpu-container-demo
description: "Multi-vendor GPU container transport: NVIDIA CUDA + AMD ROCm"

params:
  model_repo: "Qwen/Qwen2.5-Coder-7B-Instruct"
  workdir: /workspace/models

machines:
  gpu-cuda:
    hostname: gpu-cuda
    addr: container
    transport: container
    roles: [gpu, cuda]
    container:
      runtime: docker
      image: nvidia/cuda:12.4.1-runtime-ubuntu22.04
      name: forjar-cuda-demo
      ephemeral: true
      gpus: all
      env:
        CUDA_VISIBLE_DEVICES: "0,1"

  gpu-rocm:
    hostname: gpu-rocm
    addr: container
    transport: container
    roles: [gpu, rocm]
    container:
      runtime: docker
      image: rocm/dev-ubuntu-22.04:6.1
      name: forjar-rocm-demo
      ephemeral: true
      devices:
        - /dev/kfd
        - /dev/dri
      group_add:
        - video
        - render
      env:
        ROCR_VISIBLE_DEVICES: "0"

resources:
  cuda-workspace:
    type: file
    machine: gpu-cuda
    state: directory
    path: /workspace/models
    owner: root
    mode: "0755"
    tags: [gpu, setup]

  rocm-workspace:
    type: file
    machine: gpu-rocm
    state: directory
    path: /workspace/models
    owner: root
    mode: "0755"
    tags: [gpu, setup]

  model-config-cuda:
    type: file
    machine: gpu-cuda
    depends_on: [cuda-workspace]
    path: /workspace/models/model.yaml
    content: |
      model:
        repo: "{{params.model_repo}}"
        backends: [cpu, gpu]
        formats: [safetensors, gguf]
    mode: "0644"
    tags: [gpu, config]

  model-config-rocm:
    type: file
    machine: gpu-rocm
    depends_on: [rocm-workspace]
    path: /workspace/models/model.yaml
    content: |
      model:
        repo: "{{params.model_repo}}"
        backends: [cpu, gpu]
        formats: [safetensors, gguf]
    mode: "0644"
    tags: [gpu, config]

  cuda-check:
    type: file
    machine: gpu-cuda
    depends_on: [cuda-workspace]
    path: /workspace/models/gpu-check.sh
    content: |
      #!/bin/bash
      set -euo pipefail
      echo "CUDA_VISIBLE_DEVICES=$CUDA_VISIBLE_DEVICES"
      nvidia-smi --query-gpu=name,memory.total --format=csv,noheader
    mode: "0755"
    tags: [gpu, cuda]

  rocm-check:
    type: file
    machine: gpu-rocm
    depends_on: [rocm-workspace]
    path: /workspace/models/gpu-check.sh
    content: |
      #!/bin/bash
      set -euo pipefail
      echo "ROCR_VISIBLE_DEVICES=$ROCR_VISIBLE_DEVICES"
      rocm-smi --showid
      ls -la /dev/kfd /dev/dri
    mode: "0755"
    tags: [gpu, rocm]

policy:
  failure: stop_on_first
  parallel_machines: true
  tripwire: true
  lock_file: true
"#
}

fn print_gpu_machine(name: &str, machine: &Machine) {
    println!("Machine: {}", name);
    println!("  roles: {:?}", machine.roles);
    println!(
        "  is_container_transport: {}",
        machine.is_container_transport()
    );
    println!("  container_name: {}", machine.container_name());
    if let Some(ref c) = machine.container {
        print_container_config(c);
    }
    println!();
}

fn print_container_config(c: &ContainerConfig) {
    println!("  runtime: {}", c.runtime);
    println!("  image: {:?}", c.image);
    if let Some(ref gpus) = c.gpus {
        println!("  gpus: {} (NVIDIA --gpus)", gpus);
    }
    if !c.devices.is_empty() {
        println!("  devices: {:?} (--device passthrough)", c.devices);
    }
    if !c.group_add.is_empty() {
        println!("  group_add: {:?} (--group-add)", c.group_add);
    }
    if !c.env.is_empty() {
        println!("  env: {:?} (--env)", c.env);
    }
}

fn print_plan(changes: &[PlannedChange]) {
    for change in changes {
        let symbol = match change.action {
            PlanAction::Create => "+",
            PlanAction::Update => "~",
            PlanAction::Destroy => "-",
            PlanAction::NoOp => " ",
        };
        println!("  {} {}", symbol, change.description);
    }
}

fn validate_and_plan(config: &ForjarConfig) {
    let errors = parser::validate_config(config);
    if errors.is_empty() {
        println!("Validation: OK");
    } else {
        for e in &errors {
            eprintln!("  ERROR: {}", e);
        }
        std::process::exit(1);
    }

    let order = resolver::build_execution_order(config).expect("DAG resolution failed");
    println!("Execution order: {:?}\n", order);

    let locks = HashMap::new();
    let plan = planner::plan(config, &order, &locks, None);
    println!("Plan: {}", plan.name);
    print_plan(&plan.changes);
}

fn main() {
    let config = parser::parse_config(gpu_config_yaml()).expect("YAML parse failed");
    println!("Parsed: {}", config.name);
    println!(
        "  {} machine(s), {} resource(s)\n",
        config.machines.len(),
        config.resources.len()
    );

    for (name, machine) in &config.machines {
        print_gpu_machine(name, machine);
    }

    validate_and_plan(&config);

    println!("\nGPU container lifecycle:");
    println!("  NVIDIA: docker run -d --name forjar-cuda-demo --init --gpus all --env CUDA_VISIBLE_DEVICES=0,1 nvidia/cuda:12.4.1-runtime-ubuntu22.04 sleep infinity");
    println!("  AMD:    docker run -d --name forjar-rocm-demo --init --device /dev/kfd --device /dev/dri --group-add video --group-add render --env ROCR_VISIBLE_DEVICES=0 rocm/dev-ubuntu-22.04:6.1 sleep infinity");
}
