//! GPU container transport integration tests.
//!
//! Feature-gated behind `--features gpu-container-test` — requires Docker
//! and GPU hardware (NVIDIA Container Toolkit or AMD ROCm drivers).
//!
//! Run: cargo test --features gpu-container-test

#![cfg(feature = "gpu-container-test")]

use forjar::core::types::*;
use forjar::transport;
use forjar::transport::container;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn cuda_machine() -> Machine {
    Machine {
        hostname: "gpu-cuda".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec!["gpu".to_string(), "cuda".to_string()],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: Some("nvidia/cuda:12.4.1-runtime-ubuntu22.04".to_string()),
            name: Some("forjar-gpu-cuda-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: Some("all".to_string()),
            devices: vec![],
            group_add: vec![],
            env: [("CUDA_VISIBLE_DEVICES".to_string(), "0".to_string())]
                .into_iter()
                .collect(),
        }),
        pepita: None,
        cost: 0,
    }
}

fn rocm_machine() -> Machine {
    Machine {
        hostname: "gpu-rocm".to_string(),
        addr: "container".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec!["gpu".to_string(), "rocm".to_string()],
        transport: Some("container".to_string()),
        container: Some(ContainerConfig {
            runtime: "docker".to_string(),
            image: Some("rocm/dev-ubuntu-22.04:6.1".to_string()),
            name: Some("forjar-gpu-rocm-test".to_string()),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
            devices: vec!["/dev/kfd".to_string(), "/dev/dri".to_string()],
            group_add: vec!["video".to_string(), "render".to_string()],
            env: [("ROCR_VISIBLE_DEVICES".to_string(), "0".to_string())]
                .into_iter()
                .collect(),
        }),
        pepita: None,
        cost: 0,
    }
}

// ---------------------------------------------------------------------------
// NVIDIA CUDA tests
// ---------------------------------------------------------------------------

#[test]
fn test_fj739_cuda_lifecycle() {
    let machine = cuda_machine();
    container::ensure_container(&machine).expect("CUDA ensure_container failed");

    let out = container::exec_container(&machine, "echo cuda-ok")
        .expect("CUDA exec_container failed");
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "cuda-ok");

    container::cleanup_container(&machine).expect("CUDA cleanup failed");
}

#[test]
fn test_fj739_cuda_nvidia_smi() {
    let machine = cuda_machine();
    container::ensure_container(&machine).expect("CUDA ensure failed");

    let out = container::exec_container(&machine, "nvidia-smi --query-gpu=name --format=csv,noheader")
        .expect("nvidia-smi exec failed");
    // nvidia-smi should succeed if NVIDIA Container Toolkit is installed
    assert!(out.success(), "nvidia-smi failed: {}", out.stderr);
    assert!(!out.stdout.trim().is_empty(), "nvidia-smi returned no GPU name");

    container::cleanup_container(&machine).expect("CUDA cleanup failed");
}

#[test]
fn test_fj739_cuda_env_vars() {
    let machine = cuda_machine();
    container::ensure_container(&machine).expect("CUDA ensure failed");

    let out = container::exec_container(&machine, "echo $CUDA_VISIBLE_DEVICES")
        .expect("env exec failed");
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "0", "CUDA_VISIBLE_DEVICES not set correctly");

    container::cleanup_container(&machine).expect("CUDA cleanup failed");
}

// ---------------------------------------------------------------------------
// AMD ROCm tests
// ---------------------------------------------------------------------------

#[test]
fn test_fj739_rocm_lifecycle() {
    let machine = rocm_machine();
    container::ensure_container(&machine).expect("ROCm ensure_container failed");

    let out = container::exec_container(&machine, "echo rocm-ok")
        .expect("ROCm exec_container failed");
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "rocm-ok");

    container::cleanup_container(&machine).expect("ROCm cleanup failed");
}

#[test]
fn test_fj739_rocm_device_access() {
    let machine = rocm_machine();
    container::ensure_container(&machine).expect("ROCm ensure failed");

    let out = container::exec_container(&machine, "ls /dev/kfd /dev/dri 2>&1")
        .expect("device access exec failed");
    assert!(out.success(), "GPU devices not accessible: {}", out.stderr);

    container::cleanup_container(&machine).expect("ROCm cleanup failed");
}

#[test]
fn test_fj739_rocm_env_vars() {
    let machine = rocm_machine();
    container::ensure_container(&machine).expect("ROCm ensure failed");

    let out = container::exec_container(&machine, "echo $ROCR_VISIBLE_DEVICES")
        .expect("env exec failed");
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "0", "ROCR_VISIBLE_DEVICES not set correctly");

    container::cleanup_container(&machine).expect("ROCm cleanup failed");
}

// ---------------------------------------------------------------------------
// Cross-vendor tests
// ---------------------------------------------------------------------------

#[test]
fn test_fj739_cross_vendor_same_config() {
    // Deploy identical model config to both CUDA and ROCm containers
    let config_script = r#"
set -euo pipefail
mkdir -p /workspace/models
cat > /workspace/models/model.yaml << 'FORJAR_EOF'
model:
  repo: Qwen/Qwen2.5-Coder-7B-Instruct
  backends: [cpu, gpu]
  formats: [safetensors, gguf]
gates:
  g1_model_loads: true
  g2_basic_inference: true
FORJAR_EOF
cat /workspace/models/model.yaml
"#;

    // CUDA
    let cuda = cuda_machine();
    container::ensure_container(&cuda).expect("CUDA ensure failed");
    let cuda_out = transport::exec_script(&cuda, config_script).expect("CUDA exec failed");
    assert!(cuda_out.success(), "CUDA config deploy failed: {}", cuda_out.stderr);
    assert!(cuda_out.stdout.contains("g1_model_loads"));

    // ROCm
    let rocm = rocm_machine();
    container::ensure_container(&rocm).expect("ROCm ensure failed");
    let rocm_out = transport::exec_script(&rocm, config_script).expect("ROCm exec failed");
    assert!(rocm_out.success(), "ROCm config deploy failed: {}", rocm_out.stderr);
    assert!(rocm_out.stdout.contains("g1_model_loads"));

    // Same output from both vendors
    assert_eq!(cuda_out.stdout, rocm_out.stdout, "Cross-vendor config mismatch");

    container::cleanup_container(&cuda).expect("CUDA cleanup failed");
    container::cleanup_container(&rocm).expect("ROCm cleanup failed");
}
