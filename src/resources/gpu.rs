//! FJ-241: GPU hardware resource handler.
//!
//! Manages GPU driver installation, verification, and state queries.
//! Checks NVIDIA driver version and GPU device availability.

use crate::core::types::Resource;

/// Generate shell script to check if a GPU driver is installed at the expected version.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("gpu0");
    let driver_version = resource.driver_version.as_deref().unwrap_or("");
    let state = resource.state.as_deref().unwrap_or("present");

    match state {
        "absent" => format!(
            "if command -v nvidia-smi >/dev/null 2>&1; then\n  echo 'exists:{}'\nelse\n  echo 'absent:{}'\nfi",
            name, name
        ),
        _ => {
            if driver_version.is_empty() {
                format!(
                    "if command -v nvidia-smi >/dev/null 2>&1; then\n  echo 'exists:{}'\nelse\n  echo 'missing:{}'\nfi",
                    name, name
                )
            } else {
                format!(
                    "if command -v nvidia-smi >/dev/null 2>&1; then\n  VER=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader 2>/dev/null | head -1)\n  if [ \"$VER\" = '{}' ]; then\n    echo 'match:{}'\n  else\n    echo 'mismatch:{}'\n  fi\nelse\n  echo 'missing:{}'\nfi",
                    driver_version, name, name, name
                )
            }
        }
    }
}

/// Generate shell script to install/remove GPU driver.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("gpu0");
    let driver_version = resource.driver_version.as_deref().unwrap_or("");
    let state = resource.state.as_deref().unwrap_or("present");

    match state {
        "absent" => format!(
            "set -euo pipefail\n$SUDO apt-get remove -y 'nvidia-driver-*' 2>/dev/null || true\necho 'removed:{}'",
            name
        ),
        _ => {
            let mut script = String::from("set -euo pipefail\nSUDO=\"\"\n[ \"$(id -u)\" -ne 0 ] && SUDO=\"sudo\"\n");

            if !driver_version.is_empty() {
                script.push_str(&format!(
                    "$SUDO apt-get install -y 'nvidia-driver-{}'\n",
                    driver_version
                ));
            } else {
                script.push_str("$SUDO apt-get install -y nvidia-driver\n");
            }

            // Install CUDA toolkit if specified
            if let Some(ref cuda) = resource.cuda_version {
                script.push_str(&format!(
                    "$SUDO apt-get install -y 'cuda-toolkit-{}'\n",
                    cuda.replace('.', "-")
                ));
            }

            // Enable nvidia-persistenced
            let persist = resource.persistence_mode.unwrap_or(true);
            if persist {
                script.push_str("$SUDO systemctl enable --now nvidia-persistenced 2>/dev/null || true\n");
            }

            // Set compute mode if specified
            if let Some(ref mode) = resource.compute_mode {
                let mode_val = match mode.as_str() {
                    "exclusive_process" => "1",
                    "prohibited" => "2",
                    _ => "0",
                };
                script.push_str(&format!(
                    "$SUDO nvidia-smi -c {} 2>/dev/null || true\n",
                    mode_val
                ));
            }

            script.push_str(&format!("echo 'installed:{}'", name));
            script
        }
    }
}

/// Generate shell to query GPU state (for BLAKE3 hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("gpu0");

    format!(
        "if command -v nvidia-smi >/dev/null 2>&1; then\n  VER=$(nvidia-smi --query-gpu=driver_version,compute_mode,memory.total --format=csv,noheader 2>/dev/null | head -1)\n  echo \"gpu={}:$VER\"\nelse\n  echo 'gpu=MISSING:{}'\nfi",
        name, name
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};
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
            driver_version: Some("535".to_string()),
            cuda_version: None,
            devices: vec![],
            persistence_mode: None,
            compute_mode: None,
            gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
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
}
