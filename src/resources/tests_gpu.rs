//! GPU resource handler tests.

use super::gpu::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};
use std::collections::HashMap;

fn make_gpu_resource(name: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Gpu,
        machine: MachineTarget::Single("gpu-box".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: None,
        content: None,
        source: None,
        target: None,
        owner: None,
        group: None,
        mode: None,
        name: Some(name.to_string()),
        enabled: None,
        restart_on: vec![],
        triggers: vec![],
        fs_type: None,
        options: None,
        uid: None,
        shell: None,
        home: None,
        groups: vec![],
        ssh_authorized_keys: vec![],
        system_user: false,
        schedule: None,
        command: None,
        image: None,
        ports: vec![],
        environment: vec![],
        volumes: vec![],
        restart: None,
        protocol: None,
        port: None,
        action: None,
        from_addr: None,
        recipe: None,
        inputs: HashMap::new(),
        arch: vec![],
        tags: vec![],
        resource_group: None,
        when: None,
        count: None,
        for_each: None,
        chroot_dir: None,
        namespace_uid: None,
        namespace_gid: None,
        seccomp: false,
        netns: false,
        cpuset: None,
        memory_limit: None,
        overlay_lower: None,
        overlay_upper: None,
        overlay_work: None,
        overlay_merged: None,
        format: None,
        quantization: None,
        checksum: None,
        cache_dir: None,
        gpu_backend: None,
        driver_version: Some("535".to_string()),
        cuda_version: None,
        rocm_version: None,
        devices: vec![],
        persistence_mode: None,
        compute_mode: None,
        gpu_memory_limit_mb: None,
        output_artifacts: vec![],
        completion_check: None,
        timeout: None,
        working_dir: None,
        task_mode: None,
        task_inputs: vec![],
        stages: vec![],
        cache: false,
        gpu_device: None,
        restart_delay: None,
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
        store: false,
        sudo: false,
        script: None,
    }
}

#[test]
fn test_fj241_check_gpu_present() {
    let r = make_gpu_resource("gpu0");
    let script = check_script(&r);
    assert!(script.contains("nvidia-smi"));
    assert!(script.contains("535"));
    assert!(script.contains("match:gpu0"));
}

#[test]
fn test_fj241_check_gpu_absent() {
    let mut r = make_gpu_resource("gpu0");
    r.state = Some("absent".to_string());
    let script = check_script(&r);
    assert!(script.contains("absent:gpu0"));
}

#[test]
fn test_fj241_apply_gpu_install() {
    let r = make_gpu_resource("gpu0");
    let script = apply_script(&r);
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("nvidia-driver-535"));
    assert!(script.contains("installed:gpu0"));
}

#[test]
fn test_fj241_apply_gpu_absent() {
    let mut r = make_gpu_resource("gpu0");
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("removed:gpu0"));
}

#[test]
fn test_fj241_state_query() {
    let r = make_gpu_resource("gpu0");
    let script = state_query_script(&r);
    assert!(script.contains("nvidia-smi"));
    assert!(script.contains("gpu=gpu0"));
    assert!(script.contains("gpu=MISSING"));
}

#[test]
fn test_fj241_apply_with_cuda() {
    let mut r = make_gpu_resource("gpu0");
    r.cuda_version = Some("12.3".to_string());
    let script = apply_script(&r);
    assert!(script.contains("cuda-toolkit-12-3"));
}

#[test]
fn test_fj241_apply_persistence_mode() {
    let r = make_gpu_resource("gpu0");
    let script = apply_script(&r);
    assert!(script.contains("nvidia-persistenced"));
}

#[test]
fn test_fj241_apply_compute_mode() {
    let mut r = make_gpu_resource("gpu0");
    r.compute_mode = Some("exclusive_process".to_string());
    let script = apply_script(&r);
    assert!(script.contains("nvidia-smi -c 1"));
}

#[test]
fn test_fj241_check_no_driver_version() {
    let mut r = make_gpu_resource("gpu0");
    r.driver_version = None;
    let script = check_script(&r);
    assert!(script.contains("exists:gpu0"));
    assert!(script.contains("missing:gpu0"));
    assert!(!script.contains("535"));
}

#[test]
fn test_fj241_gpu_yaml_parsing() {
    let yaml = r#"
version: "1.0"
name: gpu-test
machines:
  gpu:
    hostname: gpu
    addr: 10.0.0.1
resources:
  nvidia:
    type: gpu
    machine: gpu
    name: gpu0
    driver_version: "535"
    cuda_version: "12.3"
    persistence_mode: true
    compute_mode: exclusive_process
    devices: [0, 1]
"#;
    let config: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let r = &config.resources["nvidia"];
    assert_eq!(r.resource_type, ResourceType::Gpu);
    assert_eq!(r.driver_version.as_deref(), Some("535"));
    assert_eq!(r.cuda_version.as_deref(), Some("12.3"));
    assert_eq!(r.persistence_mode, Some(true));
    assert_eq!(r.compute_mode.as_deref(), Some("exclusive_process"));
    assert_eq!(r.devices, vec![0, 1]);
}

// ── FJ-1005: ROCm backend tests ──

#[test]
fn test_fj1005_check_rocm_present() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("rocm".to_string());
    r.driver_version = Some("6.3".to_string());
    let script = check_script(&r);
    assert!(script.contains("/sys/module/amdgpu/version"));
    assert!(script.contains("6.3"));
    assert!(script.contains("match:gpu0"));
    // FJ-1125: verify in-tree fallback path exists
    assert!(script.contains("kernel-$(uname -r)"));
    assert!(script.contains("/sys/module/amdgpu"));
}

#[test]
fn test_fj1005_check_rocm_no_version() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("rocm".to_string());
    r.driver_version = None;
    let script = check_script(&r);
    assert!(script.contains("rocminfo"));
    assert!(script.contains("exists:gpu0"));
    assert!(script.contains("missing:gpu0"));
    // FJ-1125: accepts /sys/module/amdgpu as presence signal
    assert!(script.contains("/sys/module/amdgpu"));
}

#[test]
fn test_fj1005_check_rocm_absent() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("rocm".to_string());
    r.state = Some("absent".to_string());
    let script = check_script(&r);
    assert!(script.contains("/sys/module/amdgpu"));
    assert!(script.contains("absent:gpu0"));
}

#[test]
fn test_fj1005_apply_rocm_install() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("rocm".to_string());
    r.driver_version = None;
    let script = apply_script(&r);
    assert!(script.contains("amdgpu-dkms"));
    assert!(script.contains("rocm-hip-runtime"));
    assert!(script.contains("installed:gpu0"));
}

#[test]
fn test_fj1005_apply_rocm_with_version() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("rocm".to_string());
    r.rocm_version = Some("6.0".to_string());
    let script = apply_script(&r);
    assert!(script.contains("amdgpu-dkms"));
    assert!(script.contains("rocm-dev=6.0"));
}

#[test]
fn test_fj1005_apply_rocm_absent() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("rocm".to_string());
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("apt-get remove"));
    assert!(script.contains("amdgpu-dkms"));
    assert!(script.contains("removed:gpu0"));
}

#[test]
fn test_fj1005_state_query_rocm() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("rocm".to_string());
    let script = state_query_script(&r);
    assert!(script.contains("rocminfo"));
    assert!(script.contains("gpu=gpu0"));
    assert!(script.contains("amdgpu/version"));
    // FJ-1125: falls back to kernel version instead of "unknown"
    assert!(script.contains("kernel-$(uname -r)"));
    assert!(!script.contains("echo unknown"));
}

// ── FJ-1125: In-tree driver fallback tests ──

#[test]
fn test_fj1125_check_rocm_version_fallback_chain() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("rocm".to_string());
    r.driver_version = Some("kernel-6.8.0-101-generic".to_string());
    let script = check_script(&r);
    // DKMS path tried first
    assert!(script.contains("if [ -f /sys/module/amdgpu/version ]"));
    // In-tree fallback: module dir exists but no version file
    assert!(script.contains("elif [ -e /sys/module/amdgpu ]"));
    assert!(script.contains("kernel-$(uname -r)"));
    // Missing path: no module at all
    assert!(script.contains("echo 'missing:gpu0'; exit 0"));
    // Version comparison
    assert!(script.contains("kernel-6.8.0-101-generic"));
    assert!(script.contains("match:gpu0"));
    assert!(script.contains("mismatch:gpu0"));
}

#[test]
fn test_fj1125_check_rocm_no_version_accepts_module_dir() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("rocm".to_string());
    r.driver_version = None;
    let script = check_script(&r);
    // Both rocminfo and /sys/module/amdgpu accepted as presence signals
    assert!(script.contains("command -v rocminfo"));
    assert!(script.contains("[ -e /sys/module/amdgpu ]"));
    assert!(script.contains("exists:gpu0"));
}

#[test]
fn test_fj1125_state_query_rocm_kernel_fallback() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("rocm".to_string());
    let script = state_query_script(&r);
    // Falls back to kernel version, never outputs "unknown"
    assert!(script
        .contains("cat /sys/module/amdgpu/version 2>/dev/null || echo \"kernel-$(uname -r)\""));
    assert!(!script.contains("echo unknown"));
}

// ── PMAT-036: GPU apply_script idempotency ──

#[test]
fn test_pmat036_apply_nvidia_skips_install_when_driver_present() {
    // apply_script should check nvidia-smi first and skip apt-get install
    // if the driver is already present.
    let r = make_gpu_resource("gpu0");
    let script = apply_script(&r);
    // Must check nvidia-smi BEFORE running apt-get install
    let smi_pos = script
        .find("nvidia-smi")
        .expect("apply must check nvidia-smi");
    let apt_pos = script
        .find("apt-get install")
        .expect("apply must have apt-get install");
    assert!(
        smi_pos < apt_pos,
        "nvidia-smi check must come before apt-get install"
    );
    // Must have conditional: skip install if driver already present
    assert!(
        script.contains("command -v nvidia-smi") || script.contains("nvidia-smi --query"),
        "must check for existing driver"
    );
}

#[test]
fn test_pmat036_apply_nvidia_no_version_skips_when_exists() {
    // With no driver_version specified, apply_script should skip install
    // if nvidia-smi is available (driver already present via any method).
    let mut r = make_gpu_resource("gpu0");
    r.driver_version = None;
    let script = apply_script(&r);
    // Must guard install behind a nvidia-smi presence check
    assert!(
        script.contains("command -v nvidia-smi"),
        "must check for existing driver before installing"
    );
    // The nvidia-smi check must come before any apt-get install
    let smi_pos = script.find("command -v nvidia-smi").unwrap();
    let apt_pos = script.find("apt-get install").unwrap();
    assert!(
        smi_pos < apt_pos,
        "nvidia-smi check must come before apt-get install"
    );
}

#[test]
fn test_pmat036_check_nvidia_version_prefix_match() {
    // driver_version: "550" should match nvidia-smi output "550.127.05"
    // (prefix match on major version, not exact string match)
    let mut r = make_gpu_resource("gpu0");
    r.driver_version = Some("550".to_string());
    let script = check_script(&r);
    // Should use prefix/starts-with comparison, not exact equality
    assert!(
        !script.contains("\"$VER\" = '550'"),
        "must not do exact match on driver version — 550 won't match 550.127.05"
    );
    // Should still check for 550 prefix
    assert!(script.contains("550"));
}

// ── FJ-1005: CPU backend tests ──

#[test]
fn test_fj1005_cpu_backend_check() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("cpu".to_string());
    let script = check_script(&r);
    assert_eq!(script, "echo 'match:gpu0'");
}

#[test]
fn test_fj1005_cpu_backend_apply() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("cpu".to_string());
    let script = apply_script(&r);
    assert_eq!(script, "echo 'installed:gpu0'");
}

#[test]
fn test_fj1005_cpu_backend_state_query() {
    let mut r = make_gpu_resource("gpu0");
    r.gpu_backend = Some("cpu".to_string());
    let script = state_query_script(&r);
    assert_eq!(script, "echo 'gpu=gpu0:cpu-only'");
}

// ── FJ-1005: Default backend + YAML parsing ──

#[test]
fn test_fj1005_default_backend_is_nvidia() {
    let r = make_gpu_resource("gpu0");
    assert!(r.gpu_backend.is_none());
    // Default behavior should be nvidia
    let script = check_script(&r);
    assert!(script.contains("nvidia-smi"));
}

#[test]
fn test_fj1005_gpu_backend_yaml_parsing() {
    let yaml = r#"
version: "1.0"
name: gpu-test
machines:
  amd:
    hostname: amd
    addr: 10.0.0.2
resources:
  radeon:
    type: gpu
    machine: amd
    name: gpu0
    gpu_backend: rocm
    driver_version: "6.3"
    rocm_version: "6.0"
"#;
    let config: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let r = &config.resources["radeon"];
    assert_eq!(r.resource_type, ResourceType::Gpu);
    assert_eq!(r.gpu_backend.as_deref(), Some("rocm"));
    assert_eq!(r.driver_version.as_deref(), Some("6.3"));
    assert_eq!(r.rocm_version.as_deref(), Some("6.0"));
}
