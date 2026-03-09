//! FJ-241 / FJ-1005: GPU hardware resource handler.
//!
//! Manages GPU driver installation, verification, and state queries.
//! Supports multiple backends: nvidia (default), rocm (AMD), and cpu (no-op).

use crate::core::types::Resource;

/// Resolve the GPU backend from the resource config.
/// Defaults to "nvidia" when `gpu_backend` is None.
fn resolve_backend(resource: &Resource) -> &str {
    resource.gpu_backend.as_deref().unwrap_or("nvidia")
}

/// Generate shell script to check if a GPU driver is installed at the expected version.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("gpu0");
    let state = resource.state.as_deref().unwrap_or("present");

    match resolve_backend(resource) {
        "cpu" => format!("echo 'match:{name}'"),
        "rocm" => check_script_rocm(name, state, resource),
        _ => check_script_nvidia(name, state, resource),
    }
}

fn check_script_nvidia(name: &str, state: &str, _resource: &Resource) -> String {
    // FJ-1009: If nvidia-smi works, accept the driver regardless of version.
    // Vendor/host drivers (Lambda, RunPod, --gpus all) can't be swapped.
    match state {
        "absent" => format!(
            "if command -v nvidia-smi >/dev/null 2>&1; then\n  echo 'exists:{name}'\nelse\n  echo 'absent:{name}'\nfi"
        ),
        _ => format!(
            "if command -v nvidia-smi >/dev/null 2>&1; then\n  echo 'match:{name}'\nelse\n  echo 'missing:{name}'\nfi"
        ),
    }
}

fn check_script_rocm(name: &str, state: &str, resource: &Resource) -> String {
    let driver_version = resource.driver_version.as_deref().unwrap_or("");
    match state {
        "absent" => format!(
            "if [ -e /sys/module/amdgpu ]; then\n  echo 'exists:{name}'\nelse\n  echo 'absent:{name}'\nfi"
        ),
        _ => {
            if driver_version.is_empty() {
                format!(
                    "if command -v rocminfo >/dev/null 2>&1 || [ -e /sys/module/amdgpu ]; then\n  echo 'exists:{name}'\nelse\n  echo 'missing:{name}'\nfi"
                )
            } else {
                format!(
                    "if [ -f /sys/module/amdgpu/version ]; then\n  VER=$(cat /sys/module/amdgpu/version 2>/dev/null)\nelif [ -e /sys/module/amdgpu ]; then\n  VER=\"kernel-$(uname -r)\"\nelse\n  echo 'missing:{name}'; exit 0\nfi\nif [ \"$VER\" = '{driver_version}' ]; then\n  echo 'match:{name}'\nelse\n  echo 'mismatch:{name}'\nfi"
                )
            }
        }
    }
}

/// Generate shell script to install/remove GPU driver.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("gpu0");
    let state = resource.state.as_deref().unwrap_or("present");

    match resolve_backend(resource) {
        "cpu" => format!("echo 'installed:{name}'"),
        "rocm" => apply_script_rocm(name, state, resource),
        _ => apply_script_nvidia(name, state, resource),
    }
}

fn apply_script_nvidia(name: &str, state: &str, resource: &Resource) -> String {
    if state == "absent" {
        return format!(
            "set -euo pipefail\n$SUDO apt-get remove -y 'nvidia-driver-*' 2>/dev/null || true\necho 'removed:{name}'"
        );
    }

    let mut script =
        String::from("set -euo pipefail\nSUDO=\"\"\n[ \"$(id -u)\" -ne 0 ] && SUDO=\"sudo\"\n");
    emit_nvidia_driver_install(&mut script, resource);
    emit_cuda_toolkit(&mut script, resource);
    emit_nvidia_post_install(&mut script, resource);
    script.push_str(&format!("echo 'installed:{name}'"));
    script
}

/// PMAT-036 + FJ-1009: Install nvidia driver only when nvidia-smi is absent.
/// When nvidia-smi works, accept the host/vendor driver even on version mismatch
/// (--gpus-all containers pass through the host driver which cannot be changed).
///
/// When nvidia-smi is absent AND apt-get install fails (e.g. no NVIDIA apt repo
/// in containers), detect /.dockerenv as a container signal and accept gracefully
/// instead of failing hard — the host is expected to provide GPU access via
/// --gpus all / NVIDIA Container Toolkit.
fn emit_nvidia_driver_install(script: &mut String, resource: &Resource) {
    let driver_version = resource.driver_version.as_deref().unwrap_or("");
    if !driver_version.is_empty() {
        script.push_str(&format!(
            "if command -v nvidia-smi >/dev/null 2>&1; then\n\
             \x20 INSTALLED_VER=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader 2>/dev/null | head -1)\n\
             \x20 case \"$INSTALLED_VER\" in\n\
             \x20   '{driver_version}'*) ;;\n\
             \x20   *) echo \"NOTICE: requested driver {driver_version} but $INSTALLED_VER is installed (vendor/host driver — accepting)\" ;;\n\
             \x20 esac\n\
             elif [ -f /.dockerenv ] || grep -q docker /proc/1/cgroup 2>/dev/null; then\n\
             \x20 echo \"NOTICE: container detected, skipping driver install (host provides GPU via --gpus all)\"\n\
             else\n\
             \x20 $SUDO apt-get install -y 'nvidia-driver-{driver_version}'\n\
             fi\n"
        ));
    } else {
        script.push_str(
            "if command -v nvidia-smi >/dev/null 2>&1; then\n\
             \x20 true\n\
             elif [ -f /.dockerenv ] || grep -q docker /proc/1/cgroup 2>/dev/null; then\n\
             \x20 echo \"NOTICE: container detected, skipping driver install (host provides GPU via --gpus all)\"\n\
             else\n\
             \x20 $SUDO apt-get install -y nvidia-driver\n\
             fi\n",
        );
    }
}

fn emit_cuda_toolkit(script: &mut String, resource: &Resource) {
    if let Some(ref cuda) = resource.cuda_version {
        if !cuda.is_empty() {
            let cuda_pkg = cuda.replace('.', "-");
            script.push_str(&format!(
                "if command -v nvcc >/dev/null 2>&1; then\n\
                 \x20 true\n\
                 elif [ -f /.dockerenv ] || grep -q docker /proc/1/cgroup 2>/dev/null; then\n\
                 \x20 echo \"NOTICE: container detected, skipping CUDA toolkit install\"\n\
                 else\n\
                 \x20 $SUDO apt-get install -y 'cuda-toolkit-{cuda_pkg}'\n\
                 fi\n"
            ));
        }
    }
}

fn emit_nvidia_post_install(script: &mut String, resource: &Resource) {
    if resource.persistence_mode.unwrap_or(true) {
        script.push_str("$SUDO systemctl enable --now nvidia-persistenced 2>/dev/null || true\n");
    }
    if let Some(ref mode) = resource.compute_mode {
        let mode_val = match mode.as_str() {
            "exclusive_process" => "1",
            "prohibited" => "2",
            _ => "0",
        };
        script.push_str(&format!(
            "$SUDO nvidia-smi -c {mode_val} 2>/dev/null || true\n"
        ));
    }
}

fn apply_script_rocm(name: &str, state: &str, resource: &Resource) -> String {
    match state {
        "absent" => format!(
            "set -euo pipefail\n$SUDO apt-get remove -y amdgpu-dkms rocm-hip-runtime 2>/dev/null || true\necho 'removed:{name}'"
        ),
        _ => {
            let mut script = String::from("set -euo pipefail\nSUDO=\"\"\n[ \"$(id -u)\" -ne 0 ] && SUDO=\"sudo\"\n");

            // Install amdgpu-dkms kernel driver + ROCm HIP runtime
            script.push_str("$SUDO apt-get install -y amdgpu-dkms rocm-hip-runtime\n");

            // Install version-specific ROCm dev toolkit if requested
            if let Some(ref rocm_ver) = resource.rocm_version {
                script.push_str(&format!(
                    "$SUDO apt-get install -y 'rocm-dev{}'\n",
                    if rocm_ver.is_empty() {
                        String::new()
                    } else {
                        format!("={rocm_ver}")
                    }
                ));
            }

            script.push_str(&format!("echo 'installed:{name}'"));
            script
        }
    }
}

/// Generate shell to query GPU state (for BLAKE3 hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("gpu0");

    match resolve_backend(resource) {
        "cpu" => format!("echo 'gpu={name}:cpu-only'"),
        "rocm" => format!(
            "if command -v rocminfo >/dev/null 2>&1; then\n  DEV=$(rocminfo 2>/dev/null | grep -m1 'Marketing Name' | sed 's/.*: *//')\n  VER=$(cat /sys/module/amdgpu/version 2>/dev/null || echo \"kernel-$(uname -r)\")\n  echo \"gpu={name}:$DEV:$VER\"\nelse\n  echo 'gpu=MISSING:{name}'\nfi"
        ),
        _ => format!(
            "if command -v nvidia-smi >/dev/null 2>&1; then\n  VER=$(nvidia-smi --query-gpu=driver_version,compute_mode,memory.total --format=csv,noheader 2>/dev/null | head -1)\n  echo \"gpu={name}:$VER\"\nelse\n  echo 'gpu=MISSING:{name}'\nfi"
        ),
    }
}
