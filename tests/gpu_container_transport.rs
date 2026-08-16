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

/// Is an NVIDIA GPU actually usable from a container on this host?
///
/// The `gpu-container-test` feature documents its requirement as "NVIDIA
/// Container Toolkit **or** AMD ROCm drivers" — an `or` that no single machine
/// satisfies both halves of. Enabling the feature therefore ran the ROCm tests
/// on an NVIDIA box, where `docker run --device /dev/kfd` fails with "no such
/// file or directory". That is absent hardware, not a transport defect, so each
/// vendor's tests now check for their own device before asserting on it.
fn cuda_available() -> bool {
    if !docker_available() {
        return false;
    }
    if !std::path::Path::new("/dev/nvidiactl").exists() {
        eprintln!("SKIP: no NVIDIA device node (/dev/nvidiactl) on this host");
        return false;
    }
    true
}

/// Is an AMD ROCm GPU actually usable from a container on this host?
///
/// `/dev/kfd` is the AMD kernel-fusion-driver node the ROCm container requires.
fn rocm_available() -> bool {
    if !docker_available() {
        return false;
    }
    if !std::path::Path::new("/dev/kfd").exists() {
        eprintln!("SKIP: no AMD ROCm device node (/dev/kfd) on this host");
        return false;
    }
    true
}

fn docker_available() -> bool {
    let ok = std::process::Command::new("docker")
        .args(["info"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("SKIP: docker is not available on this host");
    }
    ok
}

macro_rules! require_cuda {
    () => {
        if !cuda_available() {
            return;
        }
    };
}

macro_rules! require_rocm {
    () => {
        if !rocm_available() {
            return;
        }
    };
}

/// Per-test CUDA machine. The container NAME must be unique.
///
/// Every CUDA test shared `forjar-gpu-cuda-test`, and cargo runs a test
/// binary's tests in parallel threads, so three of them raced one container
/// name and died on `Conflict. The container name is already in use` — the same
/// fixture collision already fixed in `container_transport.rs`. It reads as a
/// GPU/transport failure and is really shared mutable state in the fixture.
fn cuda_machine_named(name: &str) -> Machine {
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
            name: Some(format!("forjar-gpu-cuda-test-{name}")),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: Some("all".to_string()),
            devices: vec![],
            group_add: vec![],
            env: [("CUDA_VISIBLE_DEVICES".to_string(), "0".to_string())]
                .into_iter()
                .collect(),
            volumes: vec![],
        }),
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

/// Per-test ROCm machine. Unique container name, for the reason on
/// [`cuda_machine_named`].
fn rocm_machine_named(name: &str) -> Machine {
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
            name: Some(format!("forjar-gpu-rocm-test-{name}")),
            ephemeral: true,
            privileged: false,
            init: true,
            gpus: None,
            devices: vec!["/dev/kfd".to_string(), "/dev/dri".to_string()],
            group_add: vec!["video".to_string(), "render".to_string()],
            env: [("ROCR_VISIBLE_DEVICES".to_string(), "0".to_string())]
                .into_iter()
                .collect(),
            volumes: vec![],
        }),
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

// ---------------------------------------------------------------------------
// NVIDIA CUDA tests
// ---------------------------------------------------------------------------

#[test]
fn test_fj739_cuda_lifecycle() {
    require_cuda!();
    let machine = cuda_machine_named("lifecycle");
    container::ensure_container(&machine).expect("CUDA ensure_container failed");

    let out = container::exec_container(&machine, "echo cuda-ok", None)
        .expect("CUDA exec_container failed");
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "cuda-ok");

    container::cleanup_container(&machine).expect("CUDA cleanup failed");
}

#[test]
fn test_fj739_cuda_nvidia_smi() {
    require_cuda!();
    let machine = cuda_machine_named("smi");
    container::ensure_container(&machine).expect("CUDA ensure failed");

    let out = container::exec_container(
        &machine,
        "nvidia-smi --query-gpu=name --format=csv,noheader",
        None,
    )
    .expect("nvidia-smi exec failed");
    // nvidia-smi should succeed if NVIDIA Container Toolkit is installed
    assert!(out.success(), "nvidia-smi failed: {}", out.stderr);
    assert!(
        !out.stdout.trim().is_empty(),
        "nvidia-smi returned no GPU name"
    );

    container::cleanup_container(&machine).expect("CUDA cleanup failed");
}

#[test]
fn test_fj739_cuda_env_vars() {
    require_cuda!();
    let machine = cuda_machine_named("env");
    container::ensure_container(&machine).expect("CUDA ensure failed");

    let out = container::exec_container(&machine, "echo $CUDA_VISIBLE_DEVICES", None)
        .expect("env exec failed");
    assert!(out.success());
    assert_eq!(
        out.stdout.trim(),
        "0",
        "CUDA_VISIBLE_DEVICES not set correctly"
    );

    container::cleanup_container(&machine).expect("CUDA cleanup failed");
}

// ---------------------------------------------------------------------------
// AMD ROCm tests
// ---------------------------------------------------------------------------

#[test]
fn test_fj739_rocm_lifecycle() {
    require_rocm!();
    let machine = rocm_machine_named("lifecycle");
    container::ensure_container(&machine).expect("ROCm ensure_container failed");

    let out = container::exec_container(&machine, "echo rocm-ok", None)
        .expect("ROCm exec_container failed");
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "rocm-ok");

    container::cleanup_container(&machine).expect("ROCm cleanup failed");
}

#[test]
fn test_fj739_rocm_device_access() {
    require_rocm!();
    let machine = rocm_machine_named("devices");
    container::ensure_container(&machine).expect("ROCm ensure failed");

    let out = container::exec_container(&machine, "ls /dev/kfd /dev/dri 2>&1", None)
        .expect("device access exec failed");
    assert!(out.success(), "GPU devices not accessible: {}", out.stderr);

    container::cleanup_container(&machine).expect("ROCm cleanup failed");
}

#[test]
fn test_fj739_rocm_env_vars() {
    require_rocm!();
    let machine = rocm_machine_named("env");
    container::ensure_container(&machine).expect("ROCm ensure failed");

    let out = container::exec_container(&machine, "echo $ROCR_VISIBLE_DEVICES", None)
        .expect("env exec failed");
    assert!(out.success());
    assert_eq!(
        out.stdout.trim(),
        "0",
        "ROCR_VISIBLE_DEVICES not set correctly"
    );

    container::cleanup_container(&machine).expect("ROCm cleanup failed");
}

// ---------------------------------------------------------------------------
// Cross-vendor tests
// ---------------------------------------------------------------------------

#[test]
fn test_fj739_cross_vendor_same_config() {
    // The only test that genuinely needs BOTH vendors present, which is why it
    // is the one that can essentially never run on a real machine. It is kept
    // rather than deleted because it is the actual cross-vendor claim; the
    // guard states the requirement instead of failing on absent hardware.
    require_cuda!();
    require_rocm!();

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
    let cuda = cuda_machine_named("crossvendor");
    container::ensure_container(&cuda).expect("CUDA ensure failed");
    let cuda_out = transport::exec_script(&cuda, config_script).expect("CUDA exec failed");
    assert!(
        cuda_out.success(),
        "CUDA config deploy failed: {}",
        cuda_out.stderr
    );
    assert!(cuda_out.stdout.contains("g1_model_loads"));

    // ROCm
    let rocm = rocm_machine_named("crossvendor");
    container::ensure_container(&rocm).expect("ROCm ensure failed");
    let rocm_out = transport::exec_script(&rocm, config_script).expect("ROCm exec failed");
    assert!(
        rocm_out.success(),
        "ROCm config deploy failed: {}",
        rocm_out.stderr
    );
    assert!(rocm_out.stdout.contains("g1_model_loads"));

    // Same output from both vendors
    assert_eq!(
        cuda_out.stdout, rocm_out.stdout,
        "Cross-vendor config mismatch"
    );

    container::cleanup_container(&cuda).expect("CUDA cleanup failed");
    container::cleanup_container(&rocm).expect("ROCm cleanup failed");
}
